//! A whole first-month journey through the service layer: import, plan, back up, restore.
use fin_app::goals::Goal;
use fin_app::App;
use fin_core::NaiveDate;

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}
fn today() -> NaiveDate {
    d(2026, 11, 16)
}

const CHECKING: &str = "Date,Description,Amount\n\
09/01/2026,ACME PAYROLL,5000.00\n09/03/2026,RENT PAYMENT LANDLORD,-1500.00\n09/10/2026,GROCERY MART,-300.00\n\
10/01/2026,ACME PAYROLL,5000.00\n10/03/2026,RENT PAYMENT LANDLORD,-1500.00\n10/10/2026,GROCERY MART,-320.00\n\
11/01/2026,ACME PAYROLL,5000.00\n11/03/2026,RENT PAYMENT LANDLORD,-1500.00\n11/05/2026,CARD PAYMENT,-500.00\n11/10/2026,GROCERY MART,-310.00\n";
const CARD: &str = "Date,Description,Amount\n10/05/2026,BIG PURCHASE,-2000.00\n11/05/2026,PAYMENT THANK YOU,500.00\n";

fn journey() -> App {
    let mut app = App::in_memory().unwrap().with_clock(today);
    app.import_statement(CHECKING.as_bytes(), "Checking", "checking")
        .unwrap();
    app.import_statement(CARD.as_bytes(), "Visa card", "credit")
        .unwrap();
    app
}

#[test]
fn importing_the_same_file_twice_changes_nothing() {
    let mut app = journey();
    let before = app.balance_sheet().unwrap();
    let r = app
        .import_statement(CHECKING.as_bytes(), "Checking", "checking")
        .unwrap();
    assert_eq!((r.added, r.skipped_duplicates), (0, 10));
    assert_eq!(app.balance_sheet().unwrap(), before);
}

#[test]
fn net_worth_equals_what_the_statements_add_up_to() {
    let app = journey();
    let sheet = app.balance_sheet().unwrap();
    // Checking: 3 x (5000 - 1500) - (300 + 320 + 310) - 500 payment = 9,070. Card owes 2000 - 500 = 1,500.
    assert_eq!(sheet.assets_cents, 907_000);
    assert_eq!(sheet.debts_cents, 150_000);
    assert_eq!(sheet.net_worth_cents, 757_000);
    assert_eq!(
        sheet.net_worth_cents,
        sheet.assets_cents - sheet.debts_cents
    );
}

#[test]
fn the_card_is_a_debt_and_can_be_planned() {
    let app = journey();
    app.set_debt_terms("Visa card", 1999, 5_000).unwrap();
    let plan = app.debt_plan(0).unwrap();
    let a = plan.avalanche.unwrap();
    assert!(a.feasible);
    assert!(a.total_interest_cents > 0);
    assert_eq!(a.total_paid_cents, 150_000 + a.total_interest_cents);
    // Extra money must make it faster.
    let faster = app.debt_plan(10_000).unwrap().avalanche.unwrap();
    assert!(faster.months < a.months);
}

#[test]
fn a_backup_restores_the_same_picture_in_a_fresh_app() {
    let app = journey();
    app.set_budget("Food", 40_000).unwrap();
    app.set_debt_terms("Visa card", 1999, 5_000).unwrap();
    app.set_goal(Goal {
        name: "Trip".into(),
        target_cents: 300_000,
        saved_cents: 50_000,
        by_date: "2027-06-01".into(),
    })
    .unwrap();
    let id = app.accounts().unwrap()[0].id;
    app.set_opening_balance(id, 100_000).unwrap();
    let text = app.backup_json().unwrap();

    let mut fresh = App::in_memory().unwrap().with_clock(today);
    let report = fresh.restore_backup(&text).unwrap();
    assert_eq!(report.added, 12);
    assert_eq!(fresh.balance_sheet().unwrap(), app.balance_sheet().unwrap());
    assert_eq!(fresh.goals().unwrap(), app.goals().unwrap());
    assert_eq!(fresh.budgets().unwrap(), app.budgets().unwrap());
    assert_eq!(
        fresh.cash_flow(d(2026, 9, 1), today(), "month").unwrap(),
        app.cash_flow(d(2026, 9, 1), today(), "month").unwrap()
    );
    assert_eq!(
        fresh.debt_plan(0).unwrap().debts,
        app.debt_plan(0).unwrap().debts
    );

    // Restoring again must not double anything.
    let again = fresh.restore_backup(&text).unwrap();
    assert_eq!(again.added, 0);
    assert_eq!(fresh.balance_sheet().unwrap(), app.balance_sheet().unwrap());
}

#[test]
fn the_months_add_up_in_the_cash_flow() {
    let app = journey();
    let rows = app.cash_flow(d(2026, 9, 1), today(), "month").unwrap();
    let income: i64 = rows.iter().map(|r| r.income_cents).sum();
    let spending: i64 = rows.iter().map(|r| r.spending_cents).sum();
    assert_eq!(income, 1_500_000);
    // The card payment shows on both statements and must not count as income or spending.
    assert_eq!(income, 1_500_000);
    assert_eq!(spending, 450_000 + 93_000 + 200_000);
    assert_eq!(
        rows.iter().map(|r| r.net_cents).sum::<i64>(),
        income - spending
    );
}
