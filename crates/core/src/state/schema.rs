use rusqlite::Connection;

const VERSION: i64 = 5;

const CREATE: &str = "
CREATE TABLE IF NOT EXISTS open_projects (
    path TEXT PRIMARY KEY,
    opened_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS grants (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    actor TEXT NOT NULL,
    action TEXT NOT NULL,
    resource TEXT,
    destination TEXT,
    created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS secrets (
    name TEXT PRIMARY KEY,
    bytes INTEGER NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS secret_uses (
    name TEXT PRIMARY KEY,
    used_for TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS ssh_keys (
    name TEXT PRIMARY KEY,
    kind TEXT NOT NULL,
    public_key TEXT NOT NULL,
    fingerprint TEXT NOT NULL,
    created_at INTEGER NOT NULL
);
";

const FROM_VERSION_1: &str = "ALTER TABLE grants ADD COLUMN destination TEXT;";

pub(super) fn migrate(connection: &Connection) -> rusqlite::Result<()> {
    let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;

    connection.execute_batch(CREATE)?;

    if version == 1 {
        connection.execute_batch(FROM_VERSION_1)?;
    }

    connection.pragma_update(None, "user_version", VERSION)
}
