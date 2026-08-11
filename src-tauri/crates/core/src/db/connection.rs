use rusqlite::Connection;

use super::{migrations, repository::Repository};
use crate::errors::{Error, Result};

/// Open (and create if missing) the SQLite database under the app data dir.
pub fn open_database() -> Result<Connection> {
    let dir = Error::ensure_data_dir()?;
    let path = dir.join("soundboard.sqlite3");
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    Ok(conn)
}

#[derive(Debug)]
pub struct Database {
    pub conn: Connection,
}

impl Database {
    pub fn open() -> Result<Self> {
        Ok(Self {
            conn: open_database()?,
        })
    }

    /// Run migrations, then return a [`Repository`] bound to this database.
    pub fn init(self) -> Result<Repository> {
        migrations::run(&self.conn)?;
        Ok(Repository::new(self.conn))
    }
}
