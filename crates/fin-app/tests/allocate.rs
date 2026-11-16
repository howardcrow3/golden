use fin_app::allocate::*;
use fin_app::App;

fn app_with(cash: i64, spending: i64, contributed: i64, people: usize) -> App {
    let app = App::in_memory().unwrap();
    let mut h = app.household().unwrap();
    h.cash_cents = cash;
    h.spending_cents = spending;
    h.streams[0].contribution_cents = contributed;
    h.people.truncate(1);
    while h.people.len() < people {
        let mut p = h.people[0].clone();
        p.name = "Partner".into();
        h.people.push(p);
    }
    app.save_household(&h).unwrap();
    app
}

fn names(a: &Allocation) -> Vec<&str> {
    a.steps.iter().map(|s| s.name.as_str()).collect()
}

#[test]
fn emergency_cash_comes_first() {
    // $60k a year spending: target $30k. Has $10k, so $20k to fill.
    let a = app_with(1_000_000, 6_000_000, 0, 1)
        .suggest_allocation(1_500_000)
        .unwrap();
    assert_eq!(names(&a), ["Emergency cash"]);
    assert_eq!(a.steps[0].cents, 1_500_000);
}

#[test]
fn flows_through_the_order_and_adds_up() {
    // Cash is full; $10k already in the 401(k): room is 24,500 - 10,000 = 14,500.
    let a = app_with(9_000_000, 6_000_000, 1_000_000, 1)
        .suggest_allocation(5_000_000)
        .unwrap();
    assert_eq!(
        names(&a),
        ["Workplace retirement plan", "IRA", "Taxable account"]
    );
    assert_eq!(a.steps[0].cents, 1_450_000);
    assert_eq!(a.steps[1].cents, IRA_LIMIT_CENTS);
    assert_eq!(a.steps[2].cents, 5_000_000 - 1_450_000 - IRA_LIMIT_CENTS);
    assert_eq!(a.steps.iter().map(|s| s.cents).sum::<i64>(), 5_000_000);
}

#[test]
fn a_second_person_doubles_the_room() {
    let a = app_with(9_000_000, 6_000_000, 0, 2)
        .suggest_allocation(10_000_000)
        .unwrap();
    assert_eq!(a.steps[0].cents, WORKPLACE_LIMIT_CENTS * 2);
    assert_eq!(a.steps[1].cents, IRA_LIMIT_CENTS * 2);
}

#[test]
fn full_workplace_plan_skips_to_the_ira() {
    let a = app_with(9_000_000, 6_000_000, WORKPLACE_LIMIT_CENTS, 1)
        .suggest_allocation(500_000)
        .unwrap();
    assert_eq!(names(&a), ["IRA"]);
}

#[test]
fn zero_savings_is_empty_and_negative_is_refused() {
    let app = app_with(0, 6_000_000, 0, 1);
    assert!(app.suggest_allocation(0).unwrap().steps.is_empty());
    assert!(app.suggest_allocation(-1).is_err());
}

#[test]
fn every_step_explains_itself() {
    let a = app_with(0, 6_000_000, 0, 1)
        .suggest_allocation(50_000_000)
        .unwrap();
    assert!(a.steps.iter().all(|s| !s.why.is_empty() && s.cents > 0));
}
