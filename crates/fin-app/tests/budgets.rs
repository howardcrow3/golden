use fin_app::budget::{convert_monthly, BudgetRow, TOTAL};
use fin_app::App;
use fin_core::NaiveDate;

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

const CSV: &str = "Date,Description,Amount\n10/01/2026,ACME PAYROLL,5000.00\n10/06/2026,NETFLIX.COM,-15.49\n10/09/2026,SPOTIFY USA,-10.99\n10/12/2026,MYSTERY VENDOR 22,-80.00\n";

fn loaded() -> App {
    let mut app = App::in_memory().unwrap().with_clock(|| d(2026, 11, 16));
    app.import_statement(CSV.as_bytes(), "Checking", "checking")
        .unwrap();
    app
}

#[test]
fn conversions_match_the_ui() {
    assert_eq!(convert_monthly(650_000, "month").unwrap(), 650_000);
    assert_eq!(convert_monthly(650_000, "year").unwrap(), 7_800_000);
    assert_eq!(convert_monthly(650_000, "quarter").unwrap(), 1_950_000);
    // 30.4375 days a month, 7 a week.
    assert_eq!(convert_monthly(650_000, "week").unwrap(), 149_490);
    assert!(convert_monthly(1, "decade").is_err());
}

#[test]
fn no_budgets_to_start() {
    let app = loaded();
    assert!(app.budgets().unwrap().monthly.is_empty());
    assert!(app.budget_status("2026-10-15", "month").unwrap().is_empty());
}

#[test]
fn set_replace_and_remove() {
    let app = loaded();
    app.set_budget("Subscriptions", 3_000).unwrap();
    app.set_budget("Subscriptions", 4_000).unwrap();
    assert_eq!(app.budgets().unwrap().monthly["Subscriptions"], 4_000);
    app.set_budget("Subscriptions", 0).unwrap();
    assert!(app.budgets().unwrap().monthly.is_empty());
}

#[test]
fn bad_input_is_a_readable_error() {
    let app = loaded();
    assert!(app.set_budget("  ", 100).unwrap_err().contains("category"));
    assert!(app.set_budget("Food", -5).unwrap_err().contains("negative"));
}

#[test]
fn status_compares_spending_and_orders_overspent_first() {
    let app = loaded();
    app.set_budget("Subscriptions", 2_000).unwrap(); // spent 26.48
    app.set_budget("Food", 50_000).unwrap(); // nothing spent
    app.set_budget(TOTAL, 100_000).unwrap(); // spent 106.48 incl. uncategorized
    let rows = app.budget_status("2026-10-20", "month").unwrap();
    let names: Vec<_> = rows.iter().map(|r| r.category.as_str()).collect();
    assert_eq!(names, ["Total", "Subscriptions", "Food"]);
    let subs = &rows[1];
    assert_eq!(subs.spent_cents, 2_648);
    assert_eq!(subs.remaining_cents, -648);
    assert_eq!(
        rows[2],
        BudgetRow {
            category: "Food".into(),
            budget_cents: 50_000,
            spent_cents: 0,
            remaining_cents: 50_000,
            rollover_cents: 0,
            rolls: false,
        }
    );
    assert_eq!(rows[0].spent_cents, 10_648);
    assert_eq!(rows[0].remaining_cents, 100_000 - 10_648);
}

#[test]
fn other_periods_have_no_spending_yet_budget_scales() {
    let app = loaded();
    app.set_budget("Subscriptions", 3_000).unwrap();
    let nov = app.budget_status("2026-11-10", "month").unwrap();
    assert_eq!(nov[0].spent_cents, 0);
    let year = app.budget_status("2026-11-10", "year").unwrap();
    assert_eq!(year[0].budget_cents, 36_000);
    assert_eq!(year[0].spent_cents, 2_648);
}

#[test]
fn budgets_are_stored_in_the_database() {
    let app = loaded();
    app.set_budget("Food", 12_345).unwrap();
    assert_eq!(app.budgets().unwrap().monthly["Food"], 12_345);
}

#[test]
fn locked_after_trial_but_readable() {
    let app = loaded();
    app.set_budget("Food", 100).unwrap();
    app.db().set_setting("first_run", "2026-01-01").unwrap();
    assert!(app.set_budget("Food", 200).is_err());
    assert_eq!(app.budgets().unwrap().monthly["Food"], 100);
    assert_eq!(app.budget_status("2026-10-01", "month").unwrap().len(), 1);
}

// Rollover: money left over (or overspent) in earlier months carries into this one.
#[test]
fn rollover_carries_unspent_and_overspent_money_forward() {
    let mut app = App::in_memory().unwrap().with_clock(|| d(2026, 11, 16));
    let csv = "Date,Description,Amount\n09/05/2026,CORNER CAFE,-30.00\n10/05/2026,CORNER CAFE,-120.00\n11/05/2026,CORNER CAFE,-40.00\n";
    app.import_statement(csv.as_bytes(), "Checking", "checking")
        .unwrap();
    let cat = app.transactions("2026-09-01", "2026-09-30", false).unwrap()[0].id;
    for r in app.transactions("2026-09-01", "2026-11-30", false).unwrap() {
        app.set_category(r.id, "Food", false).unwrap();
    }
    let _ = cat;
    app.set_budget("Food", 10_000).unwrap(); // $100 a month
    let nov = |app: &App| {
        app.budget_status("2026-11-16", "month")
            .unwrap()
            .into_iter()
            .find(|r| r.category == "Food")
            .unwrap()
    };
    // Off: November is $100 - $40.
    assert_eq!(nov(&app).remaining_cents, 6_000);
    assert!(!nov(&app).rolls);
    app.set_rollover("Food", true).unwrap();
    let r = nov(&app);
    // Sept +$70, Oct -$20 carry in: $50. Remaining = 100 + 50 - 40.
    assert_eq!(r.rollover_cents, 5_000);
    assert_eq!(r.remaining_cents, 11_000);
    assert!(r.rolls);
    // Only the monthly view rolls over.
    let y = app.budget_status("2026-11-16", "year").unwrap();
    assert_eq!(
        y.iter()
            .find(|r| r.category == "Food")
            .unwrap()
            .rollover_cents,
        0
    );
    app.set_rollover("Food", false).unwrap();
    assert_eq!(nov(&app).rollover_cents, 0);
}

#[test]
fn rollover_needs_a_budget_first() {
    let app = loaded();
    assert!(app.set_rollover("Food", true).is_err());
}
