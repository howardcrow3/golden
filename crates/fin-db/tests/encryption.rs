#![cfg(feature = "sqlcipher")]
use fin_db::Db;

fn plain_sqlite_header(bytes: &[u8]) -> bool {
    bytes.starts_with(b"SQLite format 3\0")
}

#[test]
fn encrypted_file_is_not_readable_as_plain_sqlite() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("secret.db");
    {
        let db = Db::open_encrypted(&path, "correct horse battery staple").unwrap();
        db.add_account("Checking", "checking", Some("Chase"), Some("4821"))
            .unwrap();
    }
    let bytes = std::fs::read(&path).unwrap();
    assert!(!plain_sqlite_header(&bytes));
    assert!(!String::from_utf8_lossy(&bytes).contains("Chase"));
}

#[test]
fn right_key_reopens_and_wrong_key_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("secret.db");
    {
        let db = Db::open_encrypted(&path, "k1").unwrap();
        db.add_person(
            "Pat",
            chrono::NaiveDate::from_ymd_opt(1976, 4, 2).unwrap(),
            false,
        )
        .unwrap();
    }
    let db = Db::open_encrypted(&path, "k1").unwrap();
    assert_eq!(db.people().unwrap().len(), 1);
    drop(db);
    assert!(Db::open_encrypted(&path, "nope").is_err());
    assert!(Db::open(&path).is_err() || Db::open(&path).unwrap().people().is_err());
}

#[test]
fn rekey_changes_the_password() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("secret.db");
    {
        let db = Db::open_encrypted(&path, "old").unwrap();
        db.add_account("A", "savings", None, None).unwrap();
        db.rekey("new").unwrap();
    }
    assert!(Db::open_encrypted(&path, "old").is_err());
    let db = Db::open_encrypted(&path, "new").unwrap();
    assert_eq!(db.txn_count().unwrap(), 0);
}

#[test]
fn migrations_run_on_encrypted_databases() {
    let dir = tempfile::tempdir().unwrap();
    let db = Db::open_encrypted(dir.path().join("m.db"), "k").unwrap();
    assert_eq!(
        db.schema_version().unwrap(),
        fin_db::migrations::latest_version()
    );
}
