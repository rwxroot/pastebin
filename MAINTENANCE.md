# Maintenance

Some housekeeping stuff.

## Remove expired pastes

Handled automatically: `init_server` deletes expired rows once at start up.

For a manual sweep:

```bash
sqlite3 pastebin.db "DELETE FROM pastes WHERE expires_at IS NOT NULL AND expires_at < unixepoch();"
```

## Other useful queries

Count pastes:

```bash
sqlite3 pastebin.db "SELECT COUNT(*) FROM pastes;"
```

Show expired but not yet deleted rows:

```bash
sqlite3 pastebin.db "SELECT id, created_at, expires_at FROM pastes WHERE expires_at IS NOT NULL AND expires_at < unixepoch() ORDER BY expires_at;"
```
