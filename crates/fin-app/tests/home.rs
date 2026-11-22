use fin_app::household::{HomeRow, Household};
use fin_core::NaiveDate;

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 16).unwrap()
}

fn home() -> HomeRow {
    HomeRow {
        value_cents: 60_000_000,
        basis_cents: 30_000_000,
        mortgage_balance_cents: 20_000_000,
        mortgage_rate_bp: 400,
        mortgage_years_left: 15,
        sell_year: None,
        selling_cost_bp: 600,
        replacement_cost_cents: 0,
        rent_after_cents: 0,
    }
}

#[test]
fn no_home_adds_no_costs() {
    assert!(Household::sample()
        .plan_input(today())
        .unwrap()
        .one_time_costs
        .is_empty());
}

#[test]
fn a_mortgage_releases_spending_once_it_is_paid_off() {
    let mut h = Household::sample();
    h.home = Some(home());
    let plan = h.plan_input(today()).unwrap();
    let after: Vec<_> = plan
        .one_time_costs
        .iter()
        .filter(|c| c.year > 2041)
        .collect();
    assert!(!after.is_empty() && after.iter().all(|c| c.amount.cents() < 0));
    assert!(plan.one_time_costs.iter().all(|c| c.year > 2040));
}

#[test]
fn selling_adds_proceeds_in_the_sale_year() {
    let mut h = Household::sample();
    let mut r = home();
    r.sell_year = Some(2032);
    h.home = Some(r);
    let plan = h.plan_input(today()).unwrap();
    let c = plan.one_time_costs.iter().find(|c| c.year == 2032).unwrap();
    assert!(c.amount.cents() < -30_000_000, "{}", c.amount.cents());
}

#[test]
fn selling_the_home_helps_the_plan() {
    let base = Household::sample();
    let mut sold = Household::sample();
    let mut r = home();
    r.sell_year = Some(2040);
    r.replacement_cost_cents = 20_000_000;
    sold.home = Some(r);
    let run = |h: &Household| {
        fin_plan::projection::project_constant(&h.plan_input(today()).unwrap(), 0.04)
            .final_balance()
    };
    assert!(run(&sold) > run(&base));
}

#[test]
fn bad_home_numbers_are_refused() {
    let mut h = Household::sample();
    let mut r = home();
    r.value_cents = -1;
    h.home = Some(r);
    assert!(h.plan_input(today()).is_err());
    let mut r = home();
    r.mortgage_rate_bp = 9_000;
    h.home = Some(r);
    assert!(h.plan_input(today()).is_err());
}

#[test]
fn old_saves_without_a_home_still_load() {
    let json = serde_json::to_value(Household::sample()).unwrap();
    let mut obj = json.as_object().unwrap().clone();
    obj.remove("home");
    let h: Household = serde_json::from_value(obj.into()).unwrap();
    assert!(h.home.is_none());
}
