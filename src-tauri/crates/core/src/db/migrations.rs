use rusqlite::Connection;

use crate::errors::Result;

const MIGRATIONS: &[&str] = &[
    // v1: initial schema
    r#"
    CREATE TABLE IF NOT EXISTS categories (
        id         INTEGER PRIMARY KEY AUTOINCREMENT,
        name       TEXT NOT NULL UNIQUE,
        sort_order INTEGER NOT NULL DEFAULT 0,
        is_default INTEGER NOT NULL DEFAULT 0
    );

    CREATE TABLE IF NOT EXISTS sound_clips (
        id          INTEGER PRIMARY KEY AUTOINCREMENT,
        name        TEXT NOT NULL,
        category_id INTEGER REFERENCES categories(id) ON DELETE SET NULL,
        file_path   TEXT NOT NULL,
        duration_ms INTEGER NOT NULL DEFAULT 0,
        volume      REAL NOT NULL DEFAULT 1.0,
        shortcut    TEXT,
        sort_order  INTEGER NOT NULL DEFAULT 0,
        enabled     INTEGER NOT NULL DEFAULT 1,
        created_at  TEXT NOT NULL DEFAULT (datetime('now'))
    );

    CREATE INDEX IF NOT EXISTS idx_sound_clips_category ON sound_clips(category_id);
    CREATE INDEX IF NOT EXISTS idx_sound_clips_shortcut ON sound_clips(shortcut);

    CREATE TABLE IF NOT EXISTS settings (
        key   TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );

    INSERT OR IGNORE INTO categories (name, sort_order, is_default)
        VALUES ('Default', 0, 1);
    "#,
];

#[cfg(test)]
const LATEST_VERSION: i64 = MIGRATIONS.len() as i64;

pub fn run(conn: &Connection) -> Result<()> {
    let version: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    for (index, migration) in MIGRATIONS.iter().enumerate().skip(version as usize) {
        let target = (index + 1) as i64;
        tracing::debug!(target, "applying database migration");
        conn.execute_batch(migration)?;
        conn.pragma_update(None, "user_version", target)?;
    }
    Ok(())
}

#[cfg(test)]
pub fn run_in_memory(conn: &Connection) -> Result<()> {
    run(conn)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn memory() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        conn
    }

    #[test]
    fn migrations_apply() {
        let conn = memory();
        run(&conn).unwrap();
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, LATEST_VERSION);
        assert_eq!(run(&conn).unwrap(), ());
    }

    #[test]
    fn default_category_created() {
        let conn = memory();
        run(&conn).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM categories WHERE is_default = 1", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(count, 1);
    }
}
