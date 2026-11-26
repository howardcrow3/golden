use fin_app::App;
use fin_core::NaiveDate;

fn app() -> App {
    App::in_memory()
        .unwrap()
        .with_clock(|| NaiveDate::from_ymd_opt(2026, 11, 16).unwrap())
}

#[test]
fn marks_start_empty_and_toggle_sorted_without_duplicates() {
    let app = app();
    assert!(app.cancel_marks().unwrap().is_empty());
    app.set_cancel_mark("NETFLIX", true).unwrap();
    app.set_cancel_mark("GYM", true).unwrap();
    app.set_cancel_mark("NETFLIX", true).unwrap();
    assert_eq!(app.cancel_marks().unwrap(), ["GYM", "NETFLIX"]);
    app.set_cancel_mark("GYM", false).unwrap();
    assert_eq!(app.cancel_marks().unwrap(), ["NETFLIX"]);
}

#[test]
fn a_blank_payee_is_an_error() {
    assert!(app().set_cancel_mark("  ", true).is_err());
}
