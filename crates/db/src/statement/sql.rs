use sqlparser::ast::{CopySource, CopyTarget, ObjectName, SetExpr, Statement, Visit};
use sqlparser::parser::Parser;

use super::Classification;
use super::effects::Effects;
use crate::{Effect, Engine};

const UNPARSABLE: &str = "unparsable";
const SEVERAL: &str = "several statements";
const EMPTY: &str = "empty";

const INFO_PRAGMAS: [&str; 9] = [
    "table_info",
    "table_xinfo",
    "table_list",
    "index_list",
    "index_info",
    "index_xinfo",
    "foreign_key_list",
    "foreign_key_check",
    "integrity_check",
];

pub(super) fn classify(engine: Engine, text: &str) -> Classification {
    let Some(dialect) = engine.dialect() else {
        return Classification::new(Effect::Privileged, UNPARSABLE);
    };

    let Ok(statements) = Parser::parse_sql(dialect.as_ref(), text) else {
        return Classification::new(Effect::Privileged, UNPARSABLE);
    };

    match statements.as_slice() {
        [] => Classification::new(Effect::Privileged, EMPTY),
        [statement] => Classification::new(effect(statement), kind(statement)),
        _ => Classification::new(Effect::Privileged, SEVERAL),
    }
}

fn effect(statement: &Statement) -> Effect {
    let own = own_effect(statement);

    let mut effects = Effects::default();
    let _visited = statement.visit(&mut effects);

    effects.found().map_or(own, |found| found.max(own))
}

fn own_effect(statement: &Statement) -> Effect {
    match statement {
        Statement::Query(_)
        | Statement::Explain { analyze: false, .. }
        | Statement::ExplainTable { .. }
        | Statement::ShowFunctions { .. }
        | Statement::ShowVariable { .. }
        | Statement::ShowStatus { .. }
        | Statement::ShowVariables { .. }
        | Statement::ShowCreate { .. }
        | Statement::ShowColumns { .. }
        | Statement::ShowDatabases { .. }
        | Statement::ShowSchemas { .. }
        | Statement::ShowCharset(_)
        | Statement::ShowTables { .. }
        | Statement::ShowViews { .. }
        | Statement::ShowCollation { .. } => Effect::Read,
        Statement::Pragma { name, value, .. } => pragma_effect(name, value.is_some()),
        Statement::Explain {
            analyze: true,
            statement,
            ..
        } => own_effect(statement),
        Statement::Insert(_)
        | Statement::Update(_)
        | Statement::Delete(_)
        | Statement::Merge(_)
        | Statement::Copy {
            to: false,
            source: CopySource::Table { .. },
            target: CopyTarget::Stdin,
            ..
        } => Effect::Write,
        _ => Effect::Privileged,
    }
}

fn pragma_effect(name: &ObjectName, sets_value: bool) -> Effect {
    let name = name.to_string().to_ascii_lowercase();
    let name = name.rsplit('.').next().unwrap_or_default();

    if !sets_value || INFO_PRAGMAS.contains(&name) {
        return Effect::Read;
    }

    Effect::Privileged
}

fn kind(statement: &Statement) -> String {
    let known = match statement {
        Statement::Query(query) => match query.body.as_ref() {
            SetExpr::Insert(_) => "insert",
            SetExpr::Update(_) => "update",
            SetExpr::Delete(_) => "delete",
            SetExpr::Merge(_) => "merge",
            _ => "select",
        },
        Statement::Insert(_) => "insert",
        Statement::Update(_) => "update",
        Statement::Delete(_) => "delete",
        Statement::Merge(_) => "merge",
        Statement::Copy { .. } => "copy",
        Statement::Explain { .. } => "explain",
        Statement::ExplainTable { .. } => "describe",
        Statement::Pragma { .. } => "pragma",
        _ => return first_word(statement),
    };

    known.to_owned()
}

fn first_word(statement: &Statement) -> String {
    let text = statement.to_string();

    text.split_whitespace()
        .next()
        .unwrap_or(UNPARSABLE)
        .to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::classify;
    use crate::{Effect, Engine};

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
