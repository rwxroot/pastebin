CREATE TABLE pastes (
    id TEXT PRIMARY KEY NOT NULL,
    content TEXT NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    expires_at INTEGER
);
