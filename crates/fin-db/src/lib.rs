//! SQLite storage: migrations, accounts, and the transaction ledger with
//! instant week/month/quarter/year totals.
pub mod bank;
pub mod documents;
pub mod ledger;
pub mod migrations;
pub mod settings;

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

    /// Open (or create) an encrypted database. Requires the `sqlcipher` feature; fails with a
    /// clear error on a wrong key, and the file on disk is unreadable without the key.
    #[cfg(feature = "sqlcipher")]
    pub fn open_encrypted(path: impl AsRef<Path>, key: &str) -> Result<Db> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "key", key)?;
        // Reading the schema forces SQLCipher to try the key now rather than on first query.
        conn.query_row("SELECT count(*) FROM sqlite_master", [], |r| {
            r.get::<_, i64>(0)
        })
        .map_err(|_| DbError::Invalid("wrong key or not a Golden database".into()))?;
        Self::init(conn)
    }

    /// True when `path` holds an unencrypted SQLite database (used to upgrade older installs).
    pub fn is_plain_file(path: impl AsRef<Path>) -> bool {
        use std::io::Read;
        let mut head = [0u8; 16];
        std::fs::File::open(path)
            .and_then(|mut f| f.read_exact(&mut head))
            .map(|_| &head == b"SQLite format 3\0")
            .unwrap_or(false)
    }

    /// Encrypt an existing plain database file in place with `key`, keeping all data.
    /// The original is replaced only after the encrypted copy has been written.
    #[cfg(feature = "sqlcipher")]
    pub fn encrypt_plain_file(path: impl AsRef<Path>, key: &str) -> Result<()> {
        let path = path.as_ref();
        let tmp = path.with_extension("db.enc-tmp");
        let _ = std::fs::remove_file(&tmp);
        {
            let conn = Connection::open(path)?;
            let _ = conn.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()));
            let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
            let tmp_str = tmp.to_string_lossy().replace('\'', "''");
            let key_sql = key.replace('\'', "''");
            conn.execute_batch(&format!(
                "ATTACH DATABASE '{tmp_str}' AS enc KEY '{key_sql}';
                 SELECT sqlcipher_export('enc');
                 PRAGMA enc.user_version = {version};
                 DETACH DATABASE enc;"
            ))?;
        }
        for ext in ["db-wal", "db-shm"] {
            let _ = std::fs::remove_file(path.with_extension(ext));
        }
        std::fs::rename(&tmp, path).map_err(|e| DbError::Invalid(e.to_string()))?;
        Ok(())
    }

    /// Change the encryption key of an open encrypted database.
    #[cfg(feature = "sqlcipher")]
    pub fn rekey(&self, new_key: &str) -> Result<()> {
        self.conn.pragma_update(None, "rekey", new_key)?;
        Ok(())
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
