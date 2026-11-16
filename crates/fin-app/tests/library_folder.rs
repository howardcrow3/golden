use fin_app::App;
use fin_core::NaiveDate;
use std::path::{Path, PathBuf};

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

const CSV: &str = "Date,Description,Amount\n10/01/2026,ACME PAYROLL,5000.00\n";

fn tmp(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("golden-libtest-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    p
}

fn app(default: &Path) -> App {
    App::in_memory()
        .unwrap()
        .with_clock(|| d(2026, 11, 16))
        .with_library(Some(default.to_path_buf()))
}

fn files_under(root: &Path) -> Vec<String> {
    let mut out = vec![];
    fn walk(p: &std::path::Path, out: &mut Vec<String>) {
        if let Ok(rd) = std::fs::read_dir(p) {
            for e in rd.flatten() {
                if e.path().is_dir() {
                    walk(&e.path(), out);
                } else {
                    out.push(e.file_name().to_string_lossy().to_string());
                }
            }
        }
    }
    walk(root, &mut out);
    out
}

#[test]
fn new_imports_go_to_the_chosen_folder_and_old_files_stay() {
    let default = tmp("default");
    let chosen = tmp("chosen");
    let mut a = app(&default);
    a.import_file("a.csv", CSV.as_bytes(), "Checking", "checking")
        .unwrap();
    assert_eq!(files_under(&default).len(), 1);
    let shown = a.set_library_folder(chosen.to_str().unwrap()).unwrap();
    assert_eq!(shown, chosen.to_string_lossy());
    let csv2 = "Date,Description,Amount\n11/01/2026,ACME PAYROLL,5000.00\n";
    a.import_file("b.csv", csv2.as_bytes(), "Checking", "checking")
        .unwrap();
    assert_eq!(files_under(&default).len(), 1, "old file untouched");
    assert_eq!(files_under(&chosen).len(), 1, "new file in new folder");
    let _ = std::fs::remove_dir_all(&default);
    let _ = std::fs::remove_dir_all(&chosen);
}

#[test]
fn empty_path_goes_back_to_the_default() {
    let default = tmp("default2");
    let chosen = tmp("chosen2");
    let mut a = app(&default);
    a.set_library_folder(chosen.to_str().unwrap()).unwrap();
    assert_eq!(a.library_root().unwrap(), chosen.as_path());
    a.set_library_folder("  ").unwrap();
    assert_eq!(a.library_root().unwrap(), default.as_path());
    let _ = std::fs::remove_dir_all(&chosen);
}

#[test]
fn the_choice_survives_a_restart() {
    let default = tmp("default3");
    let chosen = tmp("chosen3");
    let mut a = app(&default);
    a.set_library_folder(chosen.to_str().unwrap()).unwrap();
    // A restart builds the App again over the same database and calls with_library(default).
    let again = a.with_library(Some(default.to_path_buf()));
    assert_eq!(again.library_root().unwrap(), chosen.as_path());
    let _ = std::fs::remove_dir_all(&chosen);
}

#[test]
fn bad_folders_are_refused_with_a_reason_and_change_nothing() {
    let default = tmp("default4");
    let mut a = app(&default);
    let e = a.set_library_folder("relative/path").unwrap_err();
    assert!(e.contains("full path"), "{e}");
    // A path under a regular file can't be created.
    let file = tmp("afile");
    std::fs::write(&file, b"x").unwrap();
    let e = a
        .set_library_folder(file.join("sub").to_str().unwrap())
        .unwrap_err();
    assert!(e.contains("Couldn't use that folder"), "{e}");
    assert_eq!(a.library_root().unwrap(), default.as_path());
    let _ = std::fs::remove_file(&file);
}

#[test]
fn blocked_after_the_trial() {
    let default = tmp("default5");
    let mut a = app(&default);
    a.db().set_setting("first_run", "2026-01-01").unwrap();
    assert!(a.set_library_folder("/tmp/whatever-golden").is_err());
}
