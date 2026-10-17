pub(super) const CREATE: &str = "
CREATE TABLE IF NOT EXISTS sync_projects (
    project TEXT PRIMARY KEY,
    place TEXT NOT NULL,
    cursor TEXT NOT NULL,
    enabled_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS sync_revisions (
    project TEXT NOT NULL,
    revision TEXT NOT NULL,
    document TEXT NOT NULL,
    parents TEXT NOT NULL,
    at INTEGER NOT NULL,
    path TEXT NOT NULL,
    body BLOB,
    conflicts TEXT NOT NULL,
    author TEXT,
    pushed INTEGER NOT NULL,
    PRIMARY KEY (project, revision)
);

CREATE INDEX IF NOT EXISTS sync_revisions_by_document
    ON sync_revisions (project, document);

CREATE TABLE IF NOT EXISTS sync_documents (
    project TEXT NOT NULL,
    document TEXT NOT NULL,
    path TEXT NOT NULL,
    on_disk TEXT,
    PRIMARY KEY (project, document)
);

CREATE TABLE IF NOT EXISTS sync_held (
    project TEXT NOT NULL,
    revision TEXT NOT NULL,
    sealed TEXT NOT NULL,
    PRIMARY KEY (project, revision)
);
";
