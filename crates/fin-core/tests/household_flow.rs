//! Integration: a household budget flowing through money, grains and ages end to end.
use chrono::NaiveDate;
use fin_core::{age_on, Grain, Money, PeriodKey, Person};

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

#[test]
fn monthly_budget_splits_across_a_quarter_without_losing_cents() {
    let q = PeriodKey::containing(Grain::Quarter, d(2026, 11, 15));
    let quarterly = Money::parse("$3,001.00").unwrap();
    let months = PeriodKey::range(Grain::Month, q.start, q.end());
    assert_eq!(months.len(), 3);
    let weights: Vec<u64> = months.iter().map(|m| m.days() as u64).collect();
    let parts = quarterly.allocate(&weights);
    assert_eq!(parts.iter().copied().sum::<Money>(), quarterly);
    assert!(parts[0] > parts[1]); // Oct (31d) gets more than Nov (30d)
}

#[test]
fn budget_in_any_grain_agrees_for_the_same_period() {
    let weekly = Money::from_dollars(100);
    let nov = PeriodKey::containing(Grain::Month, d(2026, 11, 1));
    let via_period = nov.budget_for(weekly, Grain::Week);
    let via_grain = Grain::convert(weekly, Grain::Week, Grain::Month);
    // Real 30-day November vs average month differ by under 2%.
    let diff = (via_period - via_grain).abs().cents() as f64 / via_grain.cents() as f64;
    assert!(diff < 0.02, "{diff}");
}

#[test]
fn couple_milestones() {
    let a = Person {
        id: "a".into(),
        name: "A".into(),
        birth_date: d(1976, 4, 2),
        is_dependent: false,
    };
    let kid = Person {
        id: "k".into(),
        name: "K".into(),
        birth_date: d(2015, 9, 30),
        is_dependent: true,
    };
    let today = d(2026, 11, 15);
    assert_eq!(a.age_on(today), 50);
    assert_eq!(age_on(kid.birth_date, today), 11);
    assert_eq!(a.reaches_59_half(), d(2035, 10, 2));
    assert_eq!(a.rmd_start_age(), 75);
    assert_eq!(a.full_retirement_age(), (67, 0));
}
