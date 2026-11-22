use fin_app::report::save_report;

#[test]
fn saves_each_kind_and_numbers_repeats() {
    let d = tempfile::tempdir().unwrap();
    let a = save_report(d.path(), "Plan 2026", "csv", "a,b\n1,2\n").unwrap();
    let b = save_report(d.path(), "Plan 2026", "csv", "x").unwrap();
    assert_eq!(a.file_name().unwrap(), "Plan 2026.csv");
    assert_eq!(b.file_name().unwrap(), "Plan 2026 (2).csv");
    assert_eq!(std::fs::read_to_string(&a).unwrap(), "a,b\n1,2\n");
    for k in ["html", "json"] {
        assert!(save_report(d.path(), "r", k, "{}").unwrap().exists());
    }
}

#[test]
fn names_cannot_escape_the_folder() {
    let d = tempfile::tempdir().unwrap();
    let p = save_report(d.path(), "../../etc/evil", "html", "x").unwrap();
    assert_eq!(p.parent().unwrap(), d.path());
    assert_eq!(p.file_name().unwrap(), "etcevil.html");
    let p = save_report(d.path(), "///", "json", "x").unwrap();
    assert_eq!(p.file_name().unwrap(), "golden-report.json");
}

#[test]
fn other_file_types_are_refused() {
    let d = tempfile::tempdir().unwrap();
    assert!(save_report(d.path(), "x", "exe", "x").is_err());
    assert!(save_report(d.path(), "x", "html/../sh", "x").is_err());
}
