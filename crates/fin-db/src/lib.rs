//! SQLite storage: migrations, accounts, and the transaction ledger with
//! instant week/month/quarter/year totals.
pub mod ledger;
pub mod migrations;

use rusqlite::Connection;
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("invalid: {0}")]
    Invalid(String),
}

pub type Result<T> = std::result::Result<T, DbError>;

/// A handle to the household database.
pub struct Db {
    pub(crate) conn: Connection,
}

impl Db {
    pub fn open(path: impl AsRef<Path>) -> Result<Db> {
        Self::init(Connection::open(path)?)
    }

    pub fn open_in_memory() -> Result<Db> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Db> {
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let _ = conn.pragma_update(None, "journal_mode", "WAL");
        let mut db = Db { conn };
        migrations::run(&mut db.conn)?;
        Ok(db)
    }

    pub fn schema_version(&self) -> Result<i64> {
        Ok(self
            .conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))?)
    }

    pub fn connection(&self) -> &Connection {
        &self.conn
    }
}
