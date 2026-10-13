use std::fmt::Write;

use rusqlite::types::Value;

use super::row::COLUMNS;
use crate::filter::Filter;
use crate::model::Seq;

pub(super) struct Query {
    pub(super) sql: String,
    pub(super) params: Vec<Value>,
}

impl Query {
    pub(super) fn page(filter: &Filter, after: Seq, limit: u32) -> Self {
        let mut query = Self {
            sql: format!("SELECT {COLUMNS} FROM events WHERE seq > ?1"),
            params: vec![Value::Integer(after.to_sql())],
        };

        if let Some(actor) = &filter.actor {
            query.condition("actor = ?", Value::Blob(actor.encode()));
        }
        if let Some(kind) = filter.kind {
            query.condition("kind = ?", Value::Integer(kind.code().into()));
        }
        if let Some(action) = &filter.action {
            query.condition("action = ?", Value::Text(action.as_str().to_owned()));
        }
        if let Some(resource) = &filter.resource {
            query.condition("resource = ?", Value::Text(resource.as_str().to_owned()));
        }
        if let Some(call) = filter.call {
            query.condition("(seq = ? OR call = ?)", Value::Integer(call.to_sql()));
        }
        if let Some(since) = filter.since {
            query.condition("at >= ?", Value::Integer(since.as_millis()));
        }
        if let Some(until) = filter.until {
            query.condition("at < ?", Value::Integer(until.as_millis()));
        }

        query.params.push(Value::Integer(limit.into()));
        let _ = write!(query.sql, " ORDER BY seq LIMIT ?{}", query.params.len());

        query
    }

    fn condition(&mut self, template: &str, value: Value) {
        self.params.push(value);
        let index = self.params.len();

        self.sql.push_str(" AND ");
        self.sql
            .push_str(&template.replace('?', &format!("?{index}")));
    }
}

#[cfg(test)]
mod tests {
    use super::Query;
    use crate::filter::Filter;
    use crate::model::{EventKind, Seq};

    #[test]
    fn numbers_every_parameter() {
        let filter = Filter::all().kind(EventKind::Finished).call(Seq::new(41));
        let query = Query::page(&filter, Seq::new(10), 50);

        assert!(query.sql.ends_with(
            "WHERE seq > ?1 AND kind = ?2 AND (seq = ?3 OR call = ?3) ORDER BY seq LIMIT ?4"
        ));
        assert_eq!(query.params.len(), 4);
    }
}
