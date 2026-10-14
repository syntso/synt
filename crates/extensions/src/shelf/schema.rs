pub(super) const CREATE: &str = "
CREATE TABLE IF NOT EXISTS extensions (
    id TEXT PRIMARY KEY,
    version TEXT NOT NULL,
    sha256 TEXT NOT NULL,
    manifest_sha256 TEXT NOT NULL DEFAULT '',
    signed INTEGER NOT NULL,
    enabled INTEGER NOT NULL,
    installed_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS extension_items (
    extension TEXT NOT NULL,
    key TEXT NOT NULL,
    value BLOB NOT NULL,
    PRIMARY KEY (extension, key)
);

CREATE TABLE IF NOT EXISTS extension_index (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    serial INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS extension_settings (
    extension TEXT PRIMARY KEY,
    vals TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS extension_files (
    extension TEXT NOT NULL,
    handle TEXT NOT NULL,
    path TEXT NOT NULL,
    access TEXT NOT NULL,
    picked_at INTEGER NOT NULL,
    PRIMARY KEY (extension, handle)
);

CREATE TABLE IF NOT EXISTS extension_revisions (
    project TEXT NOT NULL,
    path TEXT NOT NULL,
    revision TEXT NOT NULL,
    parents TEXT NOT NULL,
    body BLOB NOT NULL,
    seq INTEGER NOT NULL,
    PRIMARY KEY (project, path, revision)
);
";

pub(super) const MANIFEST_COLUMN: &str =
    "ALTER TABLE extensions ADD COLUMN manifest_sha256 TEXT NOT NULL DEFAULT ''";

pub(super) const HAS_MANIFEST_COLUMN: &str =
    "SELECT 1 FROM pragma_table_info('extensions') WHERE name = 'manifest_sha256'";
