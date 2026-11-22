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
