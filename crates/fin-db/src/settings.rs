use crate::{Db, Result};
use rusqlite::{params, OptionalExtension};

impl Db {
    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row("SELECT value FROM setting WHERE key=?1", [key], |r| {
                r.get(0)
            })
            .optional()?)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO setting(key,value) VALUES (?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    /// Set only if missing; returns the stored value either way.
    pub fn set_setting_if_absent(&self, key: &str, value: &str) -> Result<String> {
        self.conn.execute(
            "INSERT OR IGNORE INTO setting(key,value) VALUES (?1,?2)",
            params![key, value],
        )?;
        Ok(self.get_setting(key)?.unwrap_or_else(|| value.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_round_trip_and_overwrite() {
        let db = Db::open_in_memory().unwrap();
        assert_eq!(db.get_setting("a").unwrap(), None);
        db.set_setting("a", "1").unwrap();
        db.set_setting("a", "2").unwrap();
        assert_eq!(db.get_setting("a").unwrap().as_deref(), Some("2"));
    }

    #[test]
    fn if_absent_keeps_the_first_value() {
        let db = Db::open_in_memory().unwrap();
        assert_eq!(
            db.set_setting_if_absent("first", "2026-11-01").unwrap(),
            "2026-11-01"
        );
        assert_eq!(
            db.set_setting_if_absent("first", "2027-01-01").unwrap(),
            "2026-11-01"
        );
    }
}
