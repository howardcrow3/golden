use fin_app::App;
use std::path::Path;

const OCT: &str =
    "Date,Description,Amount\n10/01/2026,ACME PAYROLL,5000.00\n10/31/2026,NETFLIX.COM,-15.49\n";
const NOV: &str = "Date,Description,Amount\n11/02/2026,ACME PAYROLL,5000.00\n";

fn app(root: &Path) -> App {
    App::in_memory()
        .unwrap()
        .with_library(Some(root.to_path_buf()))
}

#[test]
fn the_original_is_filed_by_year_type_and_month() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app(dir.path());
    let r = app
        .import_file(
            "download (3).csv",
            OCT.as_bytes(),
            "Chase Checking",
            "checking",
        )
        .unwrap();
    assert_eq!(
        r.filed_as.as_deref(),
        Some("2026/Checking/2026-10 Chase Checking statement.csv")
    );
    assert_eq!(r.period_end.as_deref(), Some("2026-10-31"));
    assert_eq!(
        std::fs::read(dir.path().join(r.filed_as.unwrap())).unwrap(),
        OCT.as_bytes()
    );
}

#[test]
fn importing_the_same_file_again_files_nothing_new() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app(dir.path());
    app.import_file("a.csv", OCT.as_bytes(), "Chase Checking", "checking")
        .unwrap();
    let again = app
        .import_file("a.csv", OCT.as_bytes(), "Chase Checking", "checking")
        .unwrap();
    assert_eq!(again.added, 0);
    assert!(again.filed_as.is_none());
    let files: Vec<_> = std::fs::read_dir(dir.path().join("2026/Checking"))
        .unwrap()
        .collect();
    assert_eq!(files.len(), 1);
}

#[test]
fn two_different_files_for_the_same_month_get_numbered_names() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app(dir.path());
    let a = app
        .import_file("a.csv", OCT.as_bytes(), "Chase Checking", "checking")
        .unwrap();
    let more = "Date,Description,Amount\n10/15/2026,SHELL OIL,-40.00\n10/20/2026,TRADER JOES,-20.00\n10/31/2026,CAFE,-3.00\n";
    let b = app
        .import_file("b.csv", more.as_bytes(), "Chase Checking", "checking")
        .unwrap();
    assert_ne!(a.filed_as, b.filed_as);
    assert!(b.filed_as.unwrap().ends_with("(2).csv"));
}

#[test]
fn different_accounts_and_months_land_in_their_own_folders() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app(dir.path());
    app.import_file("a.csv", OCT.as_bytes(), "Chase Checking", "checking")
        .unwrap();
    let v = app
        .import_file("b.csv", NOV.as_bytes(), "Visa", "credit")
        .unwrap();
    assert_eq!(
        v.filed_as.as_deref(),
        Some("2026/Credit card/2026-11 Visa Credit card statement.csv")
    );
}

#[test]
fn a_bad_file_files_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = app(dir.path());
    assert!(app
        .import_file("x.csv", b"hello,world\n1,2\n", "X", "checking")
        .is_err());
    assert!(std::fs::read_dir(dir.path()).unwrap().next().is_none());
}

#[test]
fn without_a_library_import_still_works() {
    let mut app = App::in_memory().unwrap();
    let r = app
        .import_file("a.csv", OCT.as_bytes(), "Chase Checking", "checking")
        .unwrap();
    assert_eq!(r.added, 2);
    assert!(r.filed_as.is_none());
}
