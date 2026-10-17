use std::ops::ControlFlow;

use serde_json::Value;

use super::{Page, Rows};
use crate::Access;

const CUT: &str = " …";
const JSON_OVERHEAD: usize = 4;

#[derive(Debug)]
pub(crate) struct Collector {
    page: Page,
    access: Access,
    seen: u64,
    kept: Vec<Vec<Value>>,
    bytes: usize,
    more: bool,
    values_cut: bool,
}

impl Collector {
    pub(crate) fn new(page: Page, access: Access) -> Self {
        Self {
            page,
            access,
            seen: 0,
            kept: Vec::new(),
            bytes: 0,
            more: false,
            values_cut: false,
        }
    }

    pub(crate) fn take(&mut self, row: impl FnOnce() -> Vec<Value>) -> ControlFlow<()> {
        if !self.more {
            self.offer(row());
            return ControlFlow::Continue(());
        }

        if self.access.is_read_only() {
            return ControlFlow::Break(());
        }

        ControlFlow::Continue(())
    }

    pub(crate) fn finish(
        self,
        columns: Vec<String>,
        affected: impl FnOnce() -> Option<u64>,
    ) -> Rows {
        let offset = self.page.offset();
        let kept = u64::try_from(self.kept.len()).unwrap_or(u64::MAX);
        let affected = if self.access.is_read_only() {
            None
        } else {
            affected()
        };

        Rows {
            columns,
            next_offset: self.more.then_some(offset + kept),
            rows: self.kept,
            offset,
            more: self.more,
            affected,
            bytes: self.bytes,
            values_cut: self.values_cut,
        }
    }

    fn offer(&mut self, row: Vec<Value>) {
        self.seen += 1;

        if self.seen <= self.page.offset() {
            return;
        }

        if self.kept.len() == self.page.rows() {
            self.more = true;
            return;
        }

        self.keep(row);
    }

    fn keep(&mut self, row: Vec<Value>) {
        let room = self.page.bytes().saturating_sub(self.bytes);
        let mut size = encoded_size(&row);

        if size > room && !self.kept.is_empty() {
            self.more = true;
            return;
        }

        let row = if size > room {
            self.values_cut = true;
            let cut = cut_to_fit(row, room);
            size = encoded_size(&cut);
            cut
        } else {
            row
        };

        self.bytes += size;
        self.kept.push(row);
    }
}

fn cut_to_fit(row: Vec<Value>, room: usize) -> Vec<Value> {
    let share = (room / row.len().max(1)).saturating_sub(JSON_OVERHEAD);

    row.into_iter()
        .map(|value| cut_value(value, share))
        .collect()
}

fn cut_value(value: Value, share: usize) -> Value {
    let Value::String(text) = value else {
        if encoded_size(&value) > share {
            return Value::String(CUT.trim().to_owned());
        }
        return value;
    };

    if encoded_size(&text) <= share {
        return Value::String(text);
    }

    let mut budget = share.saturating_sub(CUT.len());

    loop {
        let kept = prefix(&text, budget);

        if encoded_size(&kept) <= share.saturating_sub(CUT.len()) || budget == 0 {
            return Value::String(format!("{kept}{CUT}"));
        }

        budget /= 2;
    }
}

fn prefix(text: &str, bytes: usize) -> &str {
    let mut end = bytes.min(text.len());

    while !text.is_char_boundary(end) {
        end -= 1;
    }

    &text[..end]
}

fn encoded_size<T: serde::Serialize + ?Sized>(value: &T) -> usize {
    serde_json::to_vec(value).map_or(usize::MAX, |encoded| encoded.len())
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use std::ops::ControlFlow;

    use super::Collector;
    use crate::{Access, Page};

    fn numbered(count: u64) -> impl Iterator<Item = Vec<Value>> {
        (1..=count).map(|number| vec![json!(number)])
    }

    fn collect(page: Page, rows: impl Iterator<Item = Vec<Value>>) -> crate::Rows {
        let mut collector = Collector::new(page, Access::ReadOnly);

        for row in rows {
            if collector.take(|| row).is_break() {
                break;
            }
        }

        collector.finish(vec!["n".to_owned()], || Some(0))
    }

    #[test]
    fn a_page_holds_at_most_its_row_limit_and_says_where_to_go_on() {
        let rows = collect(Page::new(3, 1 << 20), numbered(10));

        assert_eq!(
            rows.rows,
            vec![vec![json!(1)], vec![json!(2)], vec![json!(3)]]
        );
        assert!(rows.more);
        assert_eq!(rows.next_offset, Some(3));
    }

    #[test]
    fn the_next_page_starts_where_the_last_one_stopped() {
        let rows = collect(Page::new(3, 1 << 20).starting_at(9), numbered(10));

        assert_eq!(rows.rows, vec![vec![json!(10)]]);
        assert!(!rows.more);
        assert_eq!(rows.next_offset, None);
    }

    #[test]
    fn the_byte_limit_ends_a_page_before_the_row_that_would_cross_it() {
        let rows = collect(Page::new(100, 10), numbered(10));

        assert_eq!(rows.count(), 3);
        assert!(rows.bytes <= 10);
        assert_eq!(rows.next_offset, Some(3));
    }

    #[test]
    fn a_single_row_larger_than_the_page_is_cut_rather_than_lost() {
        let huge = vec![json!("x".repeat(10_000))];

        let rows = collect(Page::new(10, 100), std::iter::once(huge));

        assert_eq!(rows.count(), 1);
        assert!(rows.values_cut);
        assert!(rows.bytes <= 100);
        assert!(rows.rows[0][0].as_str().expect("text").ends_with(" …"));
    }

    #[test]
    fn an_exact_fit_does_not_claim_more_rows() {
        let rows = collect(Page::new(2, 1 << 20), numbered(2));

        assert!(!rows.more);
    }

    #[test]
    fn a_read_stops_once_the_page_is_full_but_a_write_reads_on() {
        let mut read = Collector::new(Page::new(1, 1 << 20), Access::ReadOnly);
        let mut write = Collector::new(Page::new(1, 1 << 20), Access::ReadWrite);

        for collector in [&mut read, &mut write] {
            assert_eq!(collector.take(|| vec![json!(1)]), ControlFlow::Continue(()));
            assert_eq!(collector.take(|| vec![json!(2)]), ControlFlow::Continue(()));
        }

        assert_eq!(read.take(|| vec![json!(3)]), ControlFlow::Break(()));
        assert_eq!(write.take(|| vec![json!(3)]), ControlFlow::Continue(()));
    }

    #[test]
    fn only_a_write_reports_the_rows_it_changed() {
        let read = Collector::new(Page::new(1, 1 << 20), Access::ReadOnly);
        let write = Collector::new(Page::new(1, 1 << 20), Access::ReadWrite);

        assert_eq!(read.finish(Vec::new(), || Some(7)).affected, None);
        assert_eq!(write.finish(Vec::new(), || Some(7)).affected, Some(7));
    }
}
