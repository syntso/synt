const RISKY: [&str; 27] = [
    "pg_terminate_backend",
    "pg_cancel_backend",
    "pg_reload_conf",
    "pg_rotate_logfile",
    "pg_promote",
    "pg_switch_wal",
    "pg_create_restore_point",
    "pg_read_file",
    "pg_read_binary_file",
    "pg_ls_dir",
    "pg_stat_file",
    "pg_logical_emit_message",
    "pg_drop_replication_slot",
    "pg_create_logical_replication_slot",
    "pg_create_physical_replication_slot",
    "lo_import",
    "lo_export",
    "lo_unlink",
    "lo_from_bytea",
    "lo_put",
    "set_config",
    "load_file",
    "sys_exec",
    "sys_eval",
    "load_extension",
    "readfile",
    "writefile",
];

const RISKY_PREFIXES: [&str; 3] = ["dblink", "pg_replication_origin", "fts3_tokenizer"];

pub(super) fn is_risky(name: &str) -> bool {
    let name = name.to_ascii_lowercase();

    RISKY.contains(&name.as_str()) || RISKY_PREFIXES.iter().any(|prefix| name.starts_with(prefix))
}
