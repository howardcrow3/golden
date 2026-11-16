use fin_app::App;
use fin_core::NaiveDate;

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

/// Statement lines for each month in [from, to): paycheck 5000, rent 1500, groceries 500,
/// plus a December bonus of 10,000 when `bonus`.
fn csv(months: &[(i32, u32)], bonus: bool) -> String {
    let mut s = String::from("Date,Description,Amount\n");
    for (y, m) in months {
        s += &format!("{m:02}/01/{y},ACME PAYROLL,5000.00\n");
        s += &format!("{m:02}/03/{y},RENT PAYMENT LANDLORD,-1500.00\n");
        s += &format!("{m:02}/10/{y},POS DEBIT TRADER JOE'S #12,-500.00\n");
        if bonus && *m == 12 {
            s += &format!("{m:02}/20/{y},ACME BONUS PAYOUT,10000.00\n");
        }
    }
    s
}

fn months_between(from: (i32, u32), count: u32) -> Vec<(i32, u32)> {
    let mut v = vec![];
    let (mut y, mut m) = from;
    for _ in 0..count {
        v.push((y, m));
        m += 1;
        if m == 13 {
            m = 1;
            y += 1;
        }
    }
    v
}

fn app_with(months: &[(i32, u32)], bonus: bool, today: NaiveDate) -> App {
    let mut app = App::in_memory().unwrap();
    app = match today.to_string().as_str() {
        "2026-11-16" => app.with_clock(|| d(2026, 11, 16)),
        "2026-11-01" => app.with_clock(|| d(2026, 11, 1)),
        _ => panic!("add a clock"),
    };
    app.import_statement(csv(months, bonus).as_bytes(), "Checking", "checking")
        .unwrap();
    app
}

#[test]
fn no_history_is_a_readable_error() {
    let app = App::in_memory().unwrap().with_clock(|| d(2026, 11, 16));
    let e = app.forecast(6).unwrap_err();
    assert!(e.contains("Import at least one full month"), "{e}");
}

#[test]
fn month_count_is_bounded() {
    let app = app_with(&months_between((2026, 8), 3), false, d(2026, 11, 16));
    assert!(app.forecast(0).is_err());
    assert!(app.forecast(61).is_err());
    assert!(app.forecast(60).is_ok());
}

#[test]
fn steady_months_project_the_average_and_start_from_the_current_position() {
    // Aug-Oct complete and nothing yet in November.
    let months = months_between((2026, 8), 3);
    let app = app_with(&months, false, d(2026, 11, 16));
    let f = app.forecast(3).unwrap();
    assert_eq!(f.history_months, 3);
    assert!(!f.seasonal_income);
    // Position: 3 months of (5000-1500-500) = 9000.
    assert_eq!(f.start_balance_cents, 900_000);
    assert_eq!(f.months[0].month, "2026-11");
    assert_eq!(f.months[1].month, "2026-12");
    // Full future months repeat the average.
    assert_eq!(f.months[1].income_cents, 500_000);
    assert_eq!(f.months[1].spending_cents, 200_000);
    assert_eq!(f.months[1].net_cents, 300_000);
    assert_eq!(
        f.months[2].balance_cents,
        f.months[1].balance_cents + 300_000
    );
}

#[test]
fn the_current_month_counts_only_what_is_still_expected() {
    let mut months = months_between((2026, 8), 3);
    months.push((2026, 11)); // payroll + rent + groceries all already in November
    let app = app_with(&months, false, d(2026, 11, 16));
    let f = app.forecast(2).unwrap();
    assert_eq!(f.months[0].income_cents, 0);
    assert_eq!(f.months[0].spending_cents, 0);
    assert_eq!(f.months[0].net_cents, 0);
    assert_eq!(f.months[1].net_cents, 300_000);
}

#[test]
fn a_year_of_history_puts_the_bonus_in_december() {
    // Nov 2025 through Oct 2026 = 12 complete months, with a December 2025 bonus.
    let months = months_between((2025, 11), 12);
    let app = app_with(&months, true, d(2026, 11, 16));
    let f = app.forecast(2).unwrap();
    assert!(f.seasonal_income);
    assert_eq!(f.history_months, 12);
    // average income = (12*5000 + 10000)/12 = 5833.33; Dec blends with Dec 2025 (15000).
    let avg = (12 * 500_000 + 1_000_000 + 6) / 12;
    let dec = f.months.iter().find(|m| m.month == "2026-12").unwrap();
    assert_eq!(dec.income_cents, (avg + 1_500_000 + 1) / 2);
    let nov = f.months.iter().find(|m| m.month == "2026-11").unwrap();
    // November blends with Nov 2025 (5000), and nothing has happened yet this November.
    assert_eq!(nov.income_cents, (avg + 500_000 + 1) / 2);
    assert!(dec.income_cents > nov.income_cents + 400_000);
}

#[test]
fn leading_empty_months_do_not_drag_the_average_down() {
    // Only Sep and Oct 2026 have data, though the window looks back 24 months.
    let app = app_with(&months_between((2026, 9), 2), false, d(2026, 11, 16));
    let f = app.forecast(1).unwrap();
    assert_eq!(f.history_months, 2);
    assert_eq!(f.months[0].income_cents, 500_000);
}

#[test]
fn the_month_boundary_is_handled() {
    // On the 1st the new month has no activity yet, so a full month is expected.
    let app = app_with(&months_between((2026, 8), 3), false, d(2026, 11, 1));
    let f = app.forecast(1).unwrap();
    assert_eq!(f.months[0].month, "2026-11");
    assert_eq!(f.months[0].income_cents, 500_000);
}

#[test]
fn works_after_the_trial_because_reading_is_always_allowed() {
    let app = app_with(&months_between((2026, 8), 3), false, d(2026, 11, 16));
    app.db().set_setting("first_run", "2026-01-01").unwrap();
    assert!(app.forecast(3).is_ok());
}
