pub(super) const VERSION: i64 = 2;

pub(super) const CREATE: &str = "
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
";

pub(super) const FROM_VERSION_1: &str = "ALTER TABLE grants ADD COLUMN destination TEXT;";
