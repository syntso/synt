use redis::Value as RedisValue;
use serde_json::Value;

use super::cell::Cell;
use crate::paging::Collector;
use crate::{Access, Page, Rows};

const VALUE: &str = "value";
const KEY: &str = "key";

pub(super) struct Reply(RedisValue);

impl Reply {
    pub(super) fn new(value: RedisValue) -> Self {
        Self(value)
    }

    pub(super) fn rows(self, page: Page) -> Rows {
        let columns = self.columns();
        let mut collector = Collector::new(page, Access::ReadOnly);

        for row in self.into_rows() {
            if collector.take(|| row).is_break() {
                break;
            }
        }

        collector.finish(columns, || None)
    }

    fn columns(&self) -> Vec<String> {
        let names: &[&str] = match self.0 {
            RedisValue::Map(_) => &[KEY, VALUE],
            _ => &[VALUE],
        };

        names.iter().map(|name| (*name).to_owned()).collect()
    }

    fn into_rows(self) -> Vec<Vec<Value>> {
        match self.0 {
            RedisValue::Array(items) | RedisValue::Set(items) => items
                .into_iter()
                .map(|item| vec![Cell::json(item)])
                .collect(),
            RedisValue::Map(pairs) => pairs
                .into_iter()
                .map(|(key, value)| vec![Cell::json(key), Cell::json(value)])
                .collect(),
            single => vec![vec![Cell::json(single)]],
        }
    }
}

#[cfg(test)]
mod tests {
    use redis::Value as RedisValue;
    use serde_json::json;

    use super::Reply;
    use crate::{Page, Rows};

    fn rows(reply: RedisValue) -> Rows {
        Reply::new(reply).rows(Page::new(100, 1 << 20))
    }

    #[test]
    fn scalar_replies_become_one_row() {
        let rows = rows(RedisValue::BulkString(b"ann".to_vec()));

        assert_eq!(rows.columns, ["value"]);
        assert_eq!(rows.rows, vec![vec![json!("ann")]]);
    }

    #[test]
    fn list_replies_become_a_row_per_item() {
        let reply = RedisValue::Array(vec![RedisValue::Int(1), RedisValue::Nil]);

        assert_eq!(rows(reply).rows, vec![vec![json!(1)], vec![json!(null)]]);
    }

    #[test]
    fn map_replies_become_key_and_value_rows() {
        let reply = RedisValue::Map(vec![(
            RedisValue::SimpleString("a".to_owned()),
            RedisValue::Int(1),
        )]);

        let rows = rows(reply);

        assert_eq!(rows.columns, ["key", "value"]);
        assert_eq!(rows.rows, vec![vec![json!("a"), json!(1)]]);
    }

    #[test]
    fn a_long_reply_is_cut_to_the_page() {
        let reply = RedisValue::Array((0..5).map(RedisValue::Int).collect());

        let rows = Reply::new(reply).rows(Page::new(2, 1 << 20));

        assert_eq!(rows.rows, vec![vec![json!(0)], vec![json!(1)]]);
        assert_eq!(rows.next_offset, Some(2));
    }
}
