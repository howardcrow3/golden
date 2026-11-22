use fin_app::household::Household;
use fin_app::retire_when::scan;
use fin_core::NaiveDate;

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 16).unwrap()
}

#[test]
fn finds_an_age_and_stops_there() {
    let h = Household::sample();
    let r = scan(&h, today(), 0.8, 50).unwrap();
    let age = r.age.expect("the sample household can retire by 75");
    assert_eq!(r.tries.last().unwrap().age, age);
    assert!(r.tries.last().unwrap().success_rate >= 0.8);
    // Every earlier age fell short.
    assert!(r.tries[..r.tries.len() - 1]
        .iter()
        .all(|t| t.success_rate < 0.8));
    assert_eq!(r.current_age, h.people[0].retire_age);
}

#[test]
fn a_household_that_cannot_retire_gets_no_age() {
    let mut h = Household::sample();
    h.spending_cents = 50_000_000;
    h.cash_cents = 0;
    h.taxable_cents = 0;
    h.traditional_cents = 0;
    h.roth_cents = 0;
    let r = scan(&h, today(), 0.9, 20).unwrap();
    assert_eq!(r.age, None);
    assert_eq!(r.tries.last().unwrap().age, fin_app::retire_when::LAST_AGE);
}

#[test]
fn bad_input_is_a_readable_error() {
    let mut h = Household::sample();
    h.people.clear();
    assert!(scan(&h, today(), 0.8, 20).unwrap_err().contains("person"));
}
