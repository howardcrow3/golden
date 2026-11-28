use fin_app::App;
use fin_core::NaiveDate;

fn sep() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 9, 15).unwrap()
}
fn nov() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 16).unwrap()
}

fn app() -> App {
    let mut app = App::in_memory().unwrap().with_clock(sep);
    app.import_statement(
        "Date,Description,Amount\n09/01/2026,ACME PAYROLL,5000.00\n09/03/2026,RENT PAYMENT LANDLORD,-1500.00\n10/01/2026,ACME PAYROLL,5000.00\n10/03/2026,RENT PAYMENT LANDLORD,-1500.00\n11/01/2026,ACME PAYROLL,5000.00\n"
            .as_bytes(),
        "Checking",
        "checking",
    )
    .unwrap();
    app
}

#[test]
fn starting_and_stopping_need_a_licence_after_the_trial() {
    let a = app();
    a.start_progress_tracking().unwrap();
    a.db().set_setting("first_run", "2026-01-01").unwrap();
    assert!(a.start_progress_tracking().is_err());
    assert!(a.stop_progress_tracking().is_err());
    // What was saved during the trial is still there to read.
    assert!(a.progress().unwrap().is_some());
}

#[test]
fn nothing_is_reported_before_tracking_starts() {
    assert_eq!(app().progress().unwrap(), None);
}

#[test]
fn the_start_month_is_zero_and_later_months_show_the_change() {
    let a = app();
    a.start_progress_tracking().unwrap();
    let a = a.with_clock(nov);
    let r = a.progress().unwrap().unwrap();
    assert_eq!(r.started, "2026-09-15");
    let actual: Vec<i64> = r.points.iter().map(|p| p.actual_change_cents).collect();
    assert_eq!(actual, vec![0, 350_000, 850_000]);
    assert_eq!(r.points[0].projected_change_cents, 0);
    assert!(r.points[2].projected_change_cents > 0);
    let last = r.points.last().unwrap();
    assert_eq!(
        r.ahead_cents,
        last.actual_change_cents - last.projected_change_cents
    );
}

#[test]
fn stopping_clears_it_and_starting_again_replaces_it() {
    let a = app();
    a.start_progress_tracking().unwrap();
    a.stop_progress_tracking().unwrap();
    assert_eq!(a.progress().unwrap(), None);
    a.start_progress_tracking().unwrap();
    assert!(a.progress().unwrap().is_some());
}

#[test]
fn journal_entries_are_compared_with_the_projection() {
    let a = app();
    a.start_progress_tracking().unwrap();
    a.add_journal_entry("2026-10-31", 9_999_999_900, "Year-end check")
        .unwrap();
    a.add_journal_entry("2026-10-31", 1_00, "Replaced").unwrap();
    a.add_journal_entry("2026-09-30", 5_000_000, "").unwrap();
    let r = a.progress().unwrap().unwrap();
    assert_eq!(r.journal.len(), 2);
    // Oldest first, and the same date was replaced.
    assert_eq!(r.journal[0].date, "2026-09-30");
    assert_eq!(r.journal[1].note, "Replaced");
    for j in &r.journal {
        assert_eq!(j.ahead_cents, j.savings_cents - j.projected_cents);
    }
    a.remove_journal_entry("2026-09-30").unwrap();
    assert_eq!(a.progress().unwrap().unwrap().journal.len(), 1);
}

#[test]
fn bad_journal_entries_are_readable_errors() {
    let a = app();
    assert!(a
        .add_journal_entry("last week", 1, "")
        .unwrap_err()
        .contains("year-month-day"));
    assert!(a
        .add_journal_entry("2026-10-01", -5, "")
        .unwrap_err()
        .contains("negative"));
}
