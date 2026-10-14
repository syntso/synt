pub(super) const OUTSIDE_SYSTEM: &str = "n.nspname NOT IN ('pg_catalog', 'information_schema') \
     AND n.nspname NOT LIKE 'pg\\_toast%' AND n.nspname NOT LIKE 'pg\\_temp%'";

pub(super) fn tables() -> String {
    format!(
        "SELECT n.nspname::text, c.relname::text, \
                CASE c.relkind WHEN 'v' THEN 'view' WHEN 'm' THEN 'materialized view' \
                WHEN 'f' THEN 'foreign table' ELSE 'table' END \
         FROM pg_catalog.pg_class c \
         JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace \
         WHERE c.relkind IN ('r', 'p', 'v', 'm', 'f') AND {OUTSIDE_SYSTEM} \
         ORDER BY 1, 2"
    )
}

pub(super) fn columns() -> String {
    format!(
        "SELECT n.nspname::text, c.relname::text, a.attname::text, \
                pg_catalog.format_type(a.atttypid, a.atttypmod), NOT a.attnotnull, \
                pg_catalog.pg_get_expr(d.adbin, d.adrelid) \
         FROM pg_catalog.pg_attribute a \
         JOIN pg_catalog.pg_class c ON c.oid = a.attrelid \
         JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace \
         LEFT JOIN pg_catalog.pg_attrdef d ON d.adrelid = a.attrelid AND d.adnum = a.attnum \
         WHERE a.attnum > 0 AND NOT a.attisdropped \
           AND c.relkind IN ('r', 'p', 'v', 'm', 'f') AND {OUTSIDE_SYSTEM} \
         ORDER BY n.nspname, c.relname, a.attnum"
    )
}

pub(super) fn indexes() -> String {
    format!(
        "SELECT n.nspname::text, t.relname::text, i.relname::text, \
                coalesce(a.attname::text, '(expression)'), x.indisunique, x.indisprimary \
         FROM pg_catalog.pg_index x \
         JOIN pg_catalog.pg_class t ON t.oid = x.indrelid \
         JOIN pg_catalog.pg_class i ON i.oid = x.indexrelid \
         JOIN pg_catalog.pg_namespace n ON n.oid = t.relnamespace \
         CROSS JOIN LATERAL unnest(x.indkey::int2[]) WITH ORDINALITY AS k(attnum, position) \
         LEFT JOIN pg_catalog.pg_attribute a ON a.attrelid = t.oid AND a.attnum = k.attnum \
         WHERE {OUTSIDE_SYSTEM} \
         ORDER BY n.nspname, t.relname, i.relname, k.position"
    )
}

pub(super) fn foreign_keys() -> String {
    format!(
        "SELECT n.nspname::text, t.relname::text, c.conname::text, a.attname::text, \
                rn.nspname::text, r.relname::text, ra.attname::text \
         FROM pg_catalog.pg_constraint c \
         JOIN pg_catalog.pg_class t ON t.oid = c.conrelid \
         JOIN pg_catalog.pg_namespace n ON n.oid = t.relnamespace \
         JOIN pg_catalog.pg_class r ON r.oid = c.confrelid \
         JOIN pg_catalog.pg_namespace rn ON rn.oid = r.relnamespace \
         CROSS JOIN LATERAL unnest(c.conkey, c.confkey) WITH ORDINALITY AS k(attnum, refnum, position) \
         JOIN pg_catalog.pg_attribute a ON a.attrelid = c.conrelid AND a.attnum = k.attnum \
         JOIN pg_catalog.pg_attribute ra ON ra.attrelid = c.confrelid AND ra.attnum = k.refnum \
         WHERE c.contype = 'f' AND {OUTSIDE_SYSTEM} \
         ORDER BY n.nspname, t.relname, c.conname, k.position"
    )
}
