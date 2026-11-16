use fin_app::goals::Goal;
use fin_app::App;
use fin_core::NaiveDate;

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 16).unwrap()
}

fn goal(name: &str, target: i64, saved: i64, by: &str) -> Goal {
    Goal {
        name: name.into(),
        target_cents: target,
        saved_cents: saved,
        by_date: by.into(),
    }
}

fn empty() -> App {
    App::in_memory().unwrap().with_clock(today)
}

/// Three completed months (Aug-Oct) that each keep $2,000.
fn with_history() -> App {
    let mut app = empty();
    let mut s = String::from("Date,Description,Amount\n");
    for m in [8, 9, 10] {
        s += &format!(
            "{m:02}/01/2026,ACME PAYROLL,5000.00\n{m:02}/03/2026,RENT PAYMENT LANDLORD,-3000.00\n"
        );
    }
    app.import_statement(s.as_bytes(), "Checking", "checking")
        .unwrap();
    app
}

#[test]
fn monthly_need_rounds_up_and_counts_part_months() {
    let a = empty();
    // Nov 16 to Dec 16 is one month; to Feb 20 it is 4 (Dec, Jan, Feb, and a bit).
    a.set_goal(goal("Trip", 100_000, 0, "2027-02-20")).unwrap();
    let g = &a.goals().unwrap()[0];
    assert_eq!(g.months_left, 4);
    assert_eq!(g.monthly_needed_cents, 25_000);
    a.set_goal(goal("Odd", 100_001, 0, "2027-02-20")).unwrap();
    let odd = a
        .goals()
        .unwrap()
        .into_iter()
        .find(|g| g.name == "Odd")
        .unwrap();
    assert_eq!(odd.monthly_needed_cents, 25_001);
}

#[test]
fn what_is_already_saved_counts() {
    let a = empty();
    a.set_goal(goal("Car", 1_000_000, 400_000, "2027-11-16"))
        .unwrap();
    let g = &a.goals().unwrap()[0];
    assert_eq!(g.percent, 40);
    assert_eq!(g.monthly_needed_cents, 50_000);
}

#[test]
fn status_compares_with_what_is_actually_kept() {
    let a = with_history();
    a.set_goal(goal("Easy", 600_000, 0, "2027-11-16")).unwrap(); // $500 a month
    a.set_goal(goal("Hard", 6_000_000, 0, "2027-11-16"))
        .unwrap(); // $5,000 a month
    let rows = a.goals().unwrap();
    let easy = rows.iter().find(|g| g.name == "Easy").unwrap();
    let hard = rows.iter().find(|g| g.name == "Hard").unwrap();
    assert_eq!(easy.status, "on_track");
    assert_eq!(hard.status, "behind");
    assert_eq!(easy.typical_monthly_cents, Some(200_000));
}

#[test]
fn without_history_the_status_is_unknown_not_a_guess() {
    let a = empty();
    a.set_goal(goal("Later", 600_000, 0, "2027-11-16")).unwrap();
    assert_eq!(a.goals().unwrap()[0].status, "unknown");
    assert_eq!(a.goals().unwrap()[0].typical_monthly_cents, None);
}

#[test]
fn done_and_past_due() {
    let a = empty();
    a.set_goal(goal("Done", 100_000, 100_000, "2027-01-01"))
        .unwrap();
    a.set_goal(goal("Over saved", 100_000, 250_000, "2027-01-01"))
        .unwrap();
    a.set_goal(goal("Late", 100_000, 10_000, "2026-10-01"))
        .unwrap();
    let rows = a.goals().unwrap();
    assert_eq!(
        rows.iter().find(|g| g.name == "Done").unwrap().status,
        "done"
    );
    let over = rows.iter().find(|g| g.name == "Over saved").unwrap();
    assert_eq!((over.status.as_str(), over.percent), ("done", 100));
    let late = rows.iter().find(|g| g.name == "Late").unwrap();
    assert_eq!(
        (
            late.status.as_str(),
            late.months_left,
            late.monthly_needed_cents
        ),
        ("past_due", 0, 90_000)
    );
}

#[test]
fn soonest_first_and_same_name_replaces() {
    let a = empty();
    a.set_goal(goal("B later", 1, 0, "2028-01-01")).unwrap();
    a.set_goal(goal("A sooner", 1, 0, "2027-01-01")).unwrap();
    assert_eq!(a.goals().unwrap()[0].name, "A sooner");
    a.set_goal(goal("a SOONER", 5, 0, "2027-01-01")).unwrap();
    let rows = a.goals().unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].target_cents, 5);
}

#[test]
fn removing_works_and_unknown_is_an_error() {
    let a = empty();
    a.set_goal(goal("Trip", 100, 0, "2027-01-01")).unwrap();
    a.remove_goal("trip").unwrap();
    assert!(a.goals().unwrap().is_empty());
    assert!(a.remove_goal("trip").is_err());
}

#[test]
fn bad_input_is_refused_with_a_reason() {
    let a = empty();
    assert!(a
        .set_goal(goal("  ", 100, 0, "2027-01-01"))
        .unwrap_err()
        .contains("name"));
    assert!(a
        .set_goal(goal("x", 0, 0, "2027-01-01"))
        .unwrap_err()
        .contains("target"));
    assert!(a
        .set_goal(goal("x", 100, -1, "2027-01-01"))
        .unwrap_err()
        .contains("saved"));
    assert!(a
        .set_goal(goal("x", 100, 0, "next year"))
        .unwrap_err()
        .contains("YYYY-MM-DD"));
}

#[test]
fn editing_is_locked_after_the_trial_but_reading_works() {
    let a = empty();
    a.set_goal(goal("Trip", 100, 0, "2027-01-01")).unwrap();
    a.db().set_setting("first_run", "2026-01-01").unwrap();
    assert!(a.set_goal(goal("New", 100, 0, "2027-01-01")).is_err());
    assert!(a.remove_goal("Trip").is_err());
    assert_eq!(a.goals().unwrap().len(), 1);
}

#[test]
fn goals_and_debt_terms_travel_in_a_backup() {
    let mut src = App::in_memory().unwrap().with_clock(today);
    src.import_statement(
        "Date,Description,Amount\n10/01/2026,STORE,-100.00\n".as_bytes(),
        "Visa",
        "credit",
    )
    .unwrap();
    src.set_goal(goal("Trip", 100_000, 0, "2027-06-01"))
        .unwrap();
    src.set_debt_terms("Visa", 1999, 2_500).unwrap();
    let json = src.backup_json().unwrap();
    let mut fresh = App::in_memory().unwrap().with_clock(today);
    fresh.restore_backup(&json).unwrap();
    assert_eq!(fresh.goals().unwrap()[0].name, "Trip");
    assert!(fresh.debt_plan(0).unwrap().debts[0].has_terms);
}
