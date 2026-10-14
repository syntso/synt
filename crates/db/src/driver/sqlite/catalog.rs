const USER_TABLES: &str = "m.name NOT LIKE 'sqlite\\_%' ESCAPE '\\'";

pub(super) fn tables() -> String {
    format!(
        "SELECT m.name, m.type FROM sqlite_schema m \
         WHERE m.type IN ('table', 'view') AND {USER_TABLES} ORDER BY m.name"
    )
}

pub(super) fn columns() -> String {
    format!(
        "SELECT m.name, p.name, p.type, p.\"notnull\", p.dflt_value, p.pk \
         FROM sqlite_schema m JOIN pragma_table_info(m.name) p \
         WHERE m.type IN ('table', 'view') AND {USER_TABLES} ORDER BY m.name, p.cid"
    )
}

pub(super) fn indexes() -> String {
    format!(
        "SELECT m.name, l.name, coalesce(i.name, '(expression)'), l.\"unique\", l.origin = 'pk' \
         FROM sqlite_schema m JOIN pragma_index_list(m.name) l JOIN pragma_index_info(l.name) i \
         WHERE m.type = 'table' AND {USER_TABLES} ORDER BY m.name, l.name, i.seqno"
    )
}

pub(super) fn foreign_keys() -> String {
    format!(
        "SELECT m.name, f.id, f.\"from\", f.\"table\", coalesce(f.\"to\", '') \
         FROM sqlite_schema m JOIN pragma_foreign_key_list(m.name) f \
         WHERE m.type = 'table' AND {USER_TABLES} ORDER BY m.name, f.id, f.seq"
    )
}
