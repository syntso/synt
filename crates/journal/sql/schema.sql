CREATE TABLE meta (
    key   TEXT PRIMARY KEY,
    value BLOB NOT NULL
) STRICT;

CREATE TABLE events (
    seq      INTEGER PRIMARY KEY,
    at       INTEGER NOT NULL,
    actor    BLOB    NOT NULL,
    kind     INTEGER NOT NULL,
    call     INTEGER,
    action   TEXT    NOT NULL,
    resource TEXT,
    details  BLOB    NOT NULL,
    link     BLOB    NOT NULL
) STRICT;

CREATE INDEX events_by_call     ON events (call)          WHERE call IS NOT NULL;
CREATE INDEX events_by_resource ON events (resource, seq) WHERE resource IS NOT NULL;

CREATE TRIGGER events_are_append_only BEFORE UPDATE ON events
BEGIN
    SELECT RAISE(ABORT, 'journal events are append-only');
END;
