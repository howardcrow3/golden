//! One test only: the logger is process-wide.
#[test]
fn init_writes_scrubbed_lines_and_ui_events() {
    let dir = tempfile::tempdir().unwrap();
    fin_log::init(dir.path(), false).unwrap();
    log::info!("imported 12 rows into account 987654321");
    log::debug!("hidden at info level");
    log::warn!("balance was $1,200.00");
    fin_log::from_ui(
        "error",
        "Banks",
        "sync failed for me@example.com\nsecond line",
    );
    let t = fin_log::tail(50);
    assert!(t.contains("log started"), "{t}");
    assert!(t.contains("imported 12 rows into account [num]"), "{t}");
    assert!(t.contains("balance was [amt]"), "{t}");
    assert!(
        t.contains("ERROR ui:Banks sync failed for [email] second line"),
        "{t}"
    );
    assert!(!t.contains("hidden at info level"), "{t}");
    assert!(!t.contains("987654321") && !t.contains("1,200"));
    assert_eq!(fin_log::dir().unwrap(), dir.path());
}
