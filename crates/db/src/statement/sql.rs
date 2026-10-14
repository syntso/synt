mod effects;
mod risky;
mod single;

use sqlparser::ast::Statement;
use sqlparser::parser::Parser;

use self::single::SingleStatement;
use crate::{Classification, Engine};

pub(super) struct SqlText<'a> {
    engine: Engine,
    text: &'a str,
}

impl<'a> SqlText<'a> {
    pub(super) fn new(engine: Engine, text: &'a str) -> Self {
        Self { engine, text }
    }

    pub(super) fn classification(&self) -> Classification {
        let Some(statements) = self.parse() else {
            return Classification::unparsable();
        };

        match statements.as_slice() {
            [] => Classification::empty(),
            [statement] => SingleStatement::new(statement).classification(),
            _ => Classification::several(),
        }
    }

    fn parse(&self) -> Option<Vec<Statement>> {
        let dialect = self.engine.dialect()?;

        Parser::parse_sql(dialect.as_ref(), self.text).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::SqlText;
    use crate::{Classification, Effect, Engine};

    fn classify(engine: Engine, text: &str) -> Classification {
        SqlText::new(engine, text).classification()
    }

    fn effect(engine: Engine, text: &str) -> Effect {
        classify(engine, text).effect
    }

    fn kind(engine: Engine, text: &str) -> String {
        classify(engine, text).kind
    }

    #[test]
    fn plain_reads_are_read() {
        for text in [
            "SELECT * FROM orders WHERE id = $1",
            "select count(*) from orders o join customers c on c.id = o.customer_id",
            "WITH recent AS (SELECT * FROM orders LIMIT 10) SELECT * FROM recent",
            "EXPLAIN SELECT * FROM orders",
            "SHOW search_path",
            "VALUES (1, 2)",
        ] {
            assert_eq!(effect(Engine::Postgres, text), Effect::Read, "{text}");
        }

        for text in ["SHOW TABLES", "DESCRIBE orders", "SHOW CREATE TABLE orders"] {
            assert_eq!(effect(Engine::Mysql, text), Effect::Read, "{text}");
        }

        assert_eq!(
            effect(Engine::Sqlite, "PRAGMA table_info('orders')"),
            Effect::Read
        );
    }

    #[test]
    fn data_changes_are_writes() {
        for text in [
            "INSERT INTO orders (id) VALUES (1)",
            "UPDATE orders SET total = 0 WHERE id = 1",
            "DELETE FROM orders WHERE id = 1",
            "COPY orders FROM STDIN",
            "MERGE INTO orders o USING incoming i ON o.id = i.id \
             WHEN MATCHED THEN UPDATE SET total = i.total",
        ] {
            assert_eq!(effect(Engine::Postgres, text), Effect::Write, "{text}");
        }
    }

    #[test]
    fn copy_from_a_program_or_a_server_file_is_privileged() {
        for text in [
            "COPY orders FROM PROGRAM 'id'",
            "COPY orders FROM '/etc/passwd'",
            "COPY orders TO PROGRAM 'id'",
            "COPY orders TO '/tmp/orders.csv'",
        ] {
            assert_eq!(effect(Engine::Postgres, text), Effect::Privileged, "{text}");
        }
    }

    #[test]
    fn a_select_that_locks_rows_is_a_write() {
        let text = "SELECT * FROM orders WHERE id = 1 FOR UPDATE";

        assert_eq!(effect(Engine::Postgres, text), Effect::Write);
    }

    #[test]
    fn a_select_hiding_an_insert_in_a_cte_is_a_write() {
        let text = "WITH moved AS (DELETE FROM orders WHERE id = 1 RETURNING *) \
                    SELECT * FROM moved";

        assert_eq!(effect(Engine::Postgres, text), Effect::Write);
    }

    #[test]
    fn select_into_makes_a_table_and_is_privileged() {
        let text = "SELECT * INTO archive FROM orders";

        assert_eq!(effect(Engine::Postgres, text), Effect::Privileged);
    }

    #[test]
    fn a_select_calling_an_administrative_function_is_privileged() {
        for text in [
            "SELECT pg_terminate_backend(42)",
            "SELECT pg_read_file('/etc/passwd')",
            "SELECT set_config('role', 'admin', false)",
            "SELECT * FROM dblink('host=x', 'select 1') AS t(a int)",
        ] {
            assert_eq!(effect(Engine::Postgres, text), Effect::Privileged, "{text}");
        }

        assert_eq!(
            effect(Engine::Mysql, "SELECT LOAD_FILE('/etc/passwd')"),
            Effect::Privileged
        );
    }

    #[test]
    fn schema_changes_and_grants_are_privileged() {
        for text in [
            "CREATE TABLE t (id int)",
            "DROP TABLE orders",
            "TRUNCATE orders",
            "ALTER TABLE orders ADD COLUMN note text",
            "GRANT SELECT ON orders TO reporting",
            "ALTER SYSTEM SET work_mem = '1GB'",
            "SET ROLE admin",
            "COPY orders TO '/tmp/orders.csv'",
            "BEGIN",
            "COMMIT",
            "VACUUM",
        ] {
            assert_eq!(effect(Engine::Postgres, text), Effect::Privileged, "{text}");
        }
    }

    #[test]
    fn explain_analyze_is_judged_by_the_statement_it_runs() {
        assert_eq!(
            effect(Engine::Postgres, "EXPLAIN ANALYZE SELECT 1"),
            Effect::Read
        );
        assert_eq!(
            effect(Engine::Postgres, "EXPLAIN ANALYZE DELETE FROM orders"),
            Effect::Write
        );
    }

    #[test]
    fn several_statements_are_privileged_even_when_each_reads() {
        let classified = classify(Engine::Postgres, "SELECT 1; SELECT 2");

        assert_eq!(classified.effect, Effect::Privileged);
        assert_eq!(classified.kind, "several statements");
    }

    #[test]
    fn a_read_followed_by_a_write_is_privileged() {
        let text = "SELECT 1; DROP TABLE orders";

        assert_eq!(effect(Engine::Mysql, text), Effect::Privileged);
    }

    #[test]
    fn what_the_parser_cannot_read_is_privileged() {
        let classified = classify(Engine::Postgres, "SELEKT everything");

        assert_eq!(classified.effect, Effect::Privileged);
        assert_eq!(classified.kind, "unparsable");
    }

    #[test]
    fn an_empty_text_is_privileged() {
        assert_eq!(effect(Engine::Sqlite, "  ;  "), Effect::Privileged);
    }

    #[test]
    fn a_pragma_that_sets_a_value_is_privileged() {
        assert_eq!(
            effect(Engine::Sqlite, "PRAGMA journal_mode = WAL"),
            Effect::Privileged
        );
    }

    #[test]
    fn statements_are_named_by_what_they_do() {
        assert_eq!(kind(Engine::Postgres, "select 1"), "select");
        assert_eq!(kind(Engine::Postgres, "insert into t values (1)"), "insert");
        assert_eq!(kind(Engine::Postgres, "drop table t"), "drop");
        assert_eq!(kind(Engine::Mysql, "describe t"), "describe");
    }
}
