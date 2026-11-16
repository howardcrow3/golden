use rusqlite::Connection;

/// Ordered, append-only migrations. `PRAGMA user_version` records how many have run.
const MIGRATIONS: &[&str] = &[
    // 1: accounts and ledger lines. Every line carries its day and a precomputed key per
    // grain so week/month/quarter/year totals are a plain indexed GROUP BY.
    r#"
    CREATE TABLE account (
        id          INTEGER PRIMARY KEY,
        name        TEXT NOT NULL,
        kind        TEXT NOT NULL CHECK (kind IN ('checking','savings','credit','investment','retirement','loan','property','other')),
        institution TEXT,
        last4       TEXT,
        closed      INTEGER NOT NULL DEFAULT 0
    );
    CREATE TABLE category (
        id     INTEGER PRIMARY KEY,
        name   TEXT NOT NULL UNIQUE,
        parent INTEGER REFERENCES category(id),
        kind   TEXT NOT NULL CHECK (kind IN ('income','expense','transfer'))
    );
    CREATE TABLE txn (
        id          INTEGER PRIMARY KEY,
        account_id  INTEGER NOT NULL REFERENCES account(id) ON DELETE CASCADE,
        day         TEXT NOT NULL,            -- YYYY-MM-DD
        week_key    TEXT NOT NULL,            -- 2026-W07
        month_key   TEXT NOT NULL,            -- 2026-10
        quarter_key TEXT NOT NULL,            -- 2026-Q4
        year_key    TEXT NOT NULL,            -- 2026
        cents       INTEGER NOT NULL,         -- negative = money out
        payee       TEXT NOT NULL DEFAULT '',
        memo        TEXT NOT NULL DEFAULT '',
        category_id INTEGER REFERENCES category(id),
        stream      TEXT,                     -- income stream label, if any
        dedupe_key  TEXT UNIQUE               -- stable hash so re-importing a file adds nothing
    );
    CREATE INDEX txn_day ON txn(day);
    CREATE INDEX txn_week ON txn(week_key);
    CREATE INDEX txn_month ON txn(month_key);
    CREATE INDEX txn_quarter ON txn(quarter_key);
    CREATE INDEX txn_year ON txn(year_key);
    CREATE INDEX txn_account ON txn(account_id, day);
    "#,
    // 2: household members (birthdates drive age rules).
    r#"
    CREATE TABLE person (
        id         INTEGER PRIMARY KEY,
        name       TEXT NOT NULL,
        birth_date TEXT NOT NULL,
        dependent  INTEGER NOT NULL DEFAULT 0
    );
    "#,
    // 3: small key/value settings (trial start, licence token).
    r#"
    CREATE TABLE setting (
        key   TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );
    "#,
];

pub fn run(conn: &mut Connection) -> rusqlite::Result<()> {
    let current: usize =
        conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))? as usize;
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(current) {
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", (i + 1) as i64)?;
        tx.commit()?;
    }
    Ok(())
}

pub fn latest_version() -> i64 {
    MIGRATIONS.len() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_fresh_db_to_latest() {
        let mut c = Connection::open_in_memory().unwrap();
        run(&mut c).unwrap();
        let v: i64 = c
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(v, latest_version());
    }

    #[test]
    fn rerunning_is_a_no_op() {
        let mut c = Connection::open_in_memory().unwrap();
        run(&mut c).unwrap();
        run(&mut c).unwrap();
        let n: i64 = c
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE name='txn'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 1);
    }
}
