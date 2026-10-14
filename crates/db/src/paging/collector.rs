use serde_json::Value;

use super::{Page, Rows};

const CUT: &str = " …";
const JSON_OVERHEAD: usize = 4;

#[derive(Debug)]
pub(crate) struct Collector {
    page: Page,
    seen: u64,
    kept: Vec<Vec<Value>>,
    bytes: usize,
    more: bool,
    values_cut: bool,
}

impl Collector {
    pub(crate) fn new(page: Page) -> Self {
        Self {
            page,
            seen: 0,
            kept: Vec::new(),
            bytes: 0,
            more: false,
            values_cut: false,
        }
    }

    pub(crate) fn wants_more(&self) -> bool {
        !self.more
    }

    pub(crate) fn offer(&mut self, row: Vec<Value>) {
        if self.more {
            return;
        }

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

    pub(crate) fn finish(self, columns: Vec<String>, affected: Option<u64>) -> Rows {
        let offset = self.page.offset();
        let kept = u64::try_from(self.kept.len()).unwrap_or(u64::MAX);

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

    use super::Collector;
    use crate::Page;

    fn numbered(count: u64) -> impl Iterator<Item = Vec<Value>> {
        (1..=count).map(|number| vec![json!(number)])
    }

    fn collect(page: Page, rows: impl Iterator<Item = Vec<Value>>) -> crate::Rows {
        let mut collector = Collector::new(page);

        for row in rows {
            collector.offer(row);
        }

        collector.finish(vec!["n".to_owned()], None)
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
}
