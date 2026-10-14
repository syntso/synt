use redis::Value as Reply;
use serde_json::{Value, json};

use crate::cells::{float, text_or_binary};

pub(super) fn to_json(reply: Reply) -> Value {
    match reply {
        Reply::Nil => Value::Null,
        Reply::Int(number) => json!(number),
        Reply::BulkString(bytes) | Reply::BigNumber(bytes) => text_or_binary(&bytes),
        Reply::Array(items) | Reply::Set(items) => list(items),
        Reply::SimpleString(text) | Reply::VerbatimString { text, .. } => Value::String(text),
        Reply::Okay => Value::String("OK".to_owned()),
        Reply::Map(pairs) => Value::Array(pairs.into_iter().map(pair).collect()),
        Reply::Attribute { data, .. } => to_json(*data),
        Reply::Double(number) => float(number),
        Reply::Boolean(flag) => Value::Bool(flag),
        Reply::Push { data, .. } => list(data),
        Reply::ServerError(error) => json!({ "error": error.to_string() }),
        other => Value::String(format!("{other:?}")),
    }
}

pub(super) fn rows(reply: Reply) -> (Vec<String>, Vec<Vec<Value>>) {
    match reply {
        Reply::Array(items) | Reply::Set(items) => (
            vec!["value".to_owned()],
            items.into_iter().map(|item| vec![to_json(item)]).collect(),
        ),
        Reply::Map(pairs) => (
            vec!["key".to_owned(), "value".to_owned()],
            pairs
                .into_iter()
                .map(|(key, value)| vec![to_json(key), to_json(value)])
                .collect(),
        ),
        single => (vec!["value".to_owned()], vec![vec![to_json(single)]]),
    }
}

fn list(items: Vec<Reply>) -> Value {
    Value::Array(items.into_iter().map(to_json).collect())
}

fn pair((key, value): (Reply, Reply)) -> Value {
    Value::Array(vec![to_json(key), to_json(value)])
}

#[cfg(test)]
mod tests {
    use redis::Value as Reply;
    use serde_json::json;

    use super::{rows, to_json};

    #[test]
    fn scalar_replies_become_one_row() {
        let (columns, rows) = rows(Reply::BulkString(b"ann".to_vec()));

        assert_eq!(columns, ["value"]);
        assert_eq!(rows, vec![vec![json!("ann")]]);
    }

    #[test]
    fn list_replies_become_a_row_per_item() {
        let reply = Reply::Array(vec![Reply::Int(1), Reply::Nil]);

        assert_eq!(rows(reply).1, vec![vec![json!(1)], vec![json!(null)]]);
    }

    #[test]
    fn map_replies_become_key_and_value_rows() {
        let reply = Reply::Map(vec![(Reply::SimpleString("a".to_owned()), Reply::Int(1))]);

        let (columns, rows) = rows(reply);

        assert_eq!(columns, ["key", "value"]);
        assert_eq!(rows, vec![vec![json!("a"), json!(1)]]);
    }

    #[test]
    fn nested_replies_keep_their_shape() {
        let reply = Reply::Array(vec![
            Reply::BulkString(b"0".to_vec()),
            Reply::Array(vec![Reply::Okay]),
        ]);

        assert_eq!(to_json(reply), json!(["0", ["OK"]]));
    }
}
