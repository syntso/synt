const IN_SCOPE: &str = "(TABLE_SCHEMA = DATABASE() OR (DATABASE() IS NULL AND TABLE_SCHEMA \
     NOT IN ('mysql', 'information_schema', 'performance_schema', 'sys')))";

pub(super) fn tables() -> String {
    format!(
        "SELECT TABLE_SCHEMA, TABLE_NAME, \
                CASE TABLE_TYPE WHEN 'VIEW' THEN 'view' WHEN 'SYSTEM VIEW' THEN 'view' \
                ELSE 'table' END \
         FROM information_schema.TABLES WHERE {IN_SCOPE} \
         ORDER BY TABLE_SCHEMA, TABLE_NAME"
    )
}

pub(super) fn columns() -> String {
    format!(
        "SELECT TABLE_SCHEMA, TABLE_NAME, COLUMN_NAME, COLUMN_TYPE, \
                IS_NULLABLE = 'YES', COLUMN_DEFAULT \
         FROM information_schema.COLUMNS WHERE {IN_SCOPE} \
         ORDER BY TABLE_SCHEMA, TABLE_NAME, ORDINAL_POSITION"
    )
}

pub(super) fn indexes() -> String {
    format!(
        "SELECT TABLE_SCHEMA, TABLE_NAME, INDEX_NAME, COALESCE(COLUMN_NAME, '(expression)'), \
                NON_UNIQUE = 0, INDEX_NAME = 'PRIMARY' \
         FROM information_schema.STATISTICS WHERE {IN_SCOPE} \
         ORDER BY TABLE_SCHEMA, TABLE_NAME, INDEX_NAME, SEQ_IN_INDEX"
    )
}

pub(super) fn foreign_keys() -> String {
    format!(
        "SELECT TABLE_SCHEMA, TABLE_NAME, CONSTRAINT_NAME, COLUMN_NAME, \
                REFERENCED_TABLE_SCHEMA, REFERENCED_TABLE_NAME, REFERENCED_COLUMN_NAME \
         FROM information_schema.KEY_COLUMN_USAGE \
         WHERE REFERENCED_TABLE_NAME IS NOT NULL AND {IN_SCOPE} \
         ORDER BY TABLE_SCHEMA, TABLE_NAME, CONSTRAINT_NAME, ORDINAL_POSITION"
    )
}
