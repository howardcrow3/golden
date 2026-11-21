use fin_app::App;
use fin_core::NaiveDate;

fn app() -> App {
    App::in_memory()
        .unwrap()
        .with_clock(|| NaiveDate::from_ymd_opt(2026, 11, 16).unwrap())
}

fn temp(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("golden-watch-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn imports_new_files_on_each_scan_and_never_twice() {
    let dir = temp("scan");
    let mut a = app();
    assert!(a.scan_watched().is_none(), "nothing watched yet");
    a.watch_folder(dir.to_str().unwrap(), "Checking", "checking")
        .unwrap();
    assert_eq!(a.watched_folder().unwrap().unwrap().account, "Checking");
    assert_eq!(a.scan_watched().unwrap().imported.len(), 0);
    std::fs::write(
        dir.join("oct.csv"),
        "Date,Description,Amount\n10/01/2026,ACME PAYROLL,5000.00\n",
    )
    .unwrap();
    let r = a.scan_watched().unwrap();
    assert_eq!(r.imported.len(), 1);
    let again = a.scan_watched().unwrap();
    assert_eq!(again.imported.len(), 0);
    assert_eq!(again.already_imported, 1);
    assert_eq!(a.accounts().unwrap().len(), 1);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn stopping_ends_the_scans_and_bad_folders_are_refused() {
    let dir = temp("stop");
    let mut a = app();
    assert!(a
        .watch_folder("/definitely/not/here", "X", "checking")
        .is_err());
    assert!(a.watch_folder("  ", "X", "checking").is_err());
    a.watch_folder(dir.to_str().unwrap(), "Checking", "checking")
        .unwrap();
    a.stop_watching().unwrap();
    assert!(a.watched_folder().unwrap().is_none());
    assert!(a.scan_watched().is_none());
    // A folder that disappears just stops yielding scans.
    a.watch_folder(dir.to_str().unwrap(), "Checking", "checking")
        .unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(a.scan_watched().is_none());
}
