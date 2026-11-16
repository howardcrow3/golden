use fin_app::keystore::{generate_key, key_or_create, KeyStore, MemoryKeyStore};

#[test]
fn keys_are_random_hex_and_stable_once_stored() {
    let a = generate_key().unwrap();
    let b = generate_key().unwrap();
    assert_eq!(a.len(), 64);
    assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
    assert_ne!(a, b);
    let store = MemoryKeyStore::default();
    let (k1, created1) = key_or_create(&store).unwrap();
    let (k2, created2) = key_or_create(&store).unwrap();
    assert!(created1 && !created2);
    assert_eq!(k1, k2);
    assert_eq!(store.get().unwrap(), Some(k1));
}

#[cfg(feature = "sqlcipher")]
mod encrypted {
    use fin_app::keystore::{open_private, MemoryKeyStore};
    use fin_app::App;
    use fin_db::Db;

    const CSV: &str = "Date,Description,Amount\n10/01/2026,ACME PAYROLL,5000.00\n";

    #[test]
    fn first_run_creates_an_encrypted_database_and_reopens_with_the_same_key() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("golden.db");
        let store = MemoryKeyStore::default();
        {
            let mut app = App::new(open_private(&path, &store).unwrap());
            app.import_statement(CSV.as_bytes(), "Checking", "checking")
                .unwrap();
        }
        assert!(!Db::is_plain_file(&path));
        let app = App::new(open_private(&path, &store).unwrap());
        assert_eq!(app.accounts().unwrap().len(), 1);
    }

    #[test]
    fn older_plain_install_is_upgraded_without_losing_data() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("golden.db");
        {
            let mut app = App::new(Db::open(&path).unwrap());
            app.import_statement(CSV.as_bytes(), "Checking", "checking")
                .unwrap();
        }
        let store = MemoryKeyStore::default();
        let app = App::new(open_private(&path, &store).unwrap());
        assert!(!Db::is_plain_file(&path));
        assert_eq!(app.accounts().unwrap()[0].balance_cents, 500_000);
    }

    #[test]
    fn missing_key_for_an_encrypted_file_is_an_error_not_an_empty_database() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("golden.db");
        drop(open_private(&path, &MemoryKeyStore::default()).unwrap());
        let err = open_private(&path, &MemoryKeyStore::default())
            .err()
            .unwrap();
        assert!(err.contains("key"));
    }
}
