//! Integration: properties every state's retirement rules must keep, across all states.
use fin_core::Money;
use fin_plan::tax::state::RetirementIncome;
use fin_plan::tax::{FilingStatus, StateTaxYear};

fn m(d: i64) -> Money {
    Money::from_dollars(d)
}

/// A retired couple's mix: a private pension, a government pension, military pay, IRA money and
/// Social Security, with `extra` added on top as other income.
fn household(ages: &[i32]) -> RetirementIncome {
    RetirementIncome {
        pension: m(60_000),
        public_pension: m(20_000),
        distributions: m(25_000),
        social_security: m(40_000),
        ages: ages.to_vec(),
        birth_years: ages.iter().map(|a| 2026 - a).collect(),
        military: ages
            .iter()
            .enumerate()
            .map(|(i, _)| m(if i == 0 { 15_000 } else { 0 }))
            .collect(),
        earned: ages.iter().map(|_| Money::ZERO).collect(),
        ..Default::default()
    }
}

const STATUSES: [FilingStatus; 2] = [FilingStatus::Single, FilingStatus::MarriedJoint];

fn ages(fs: FilingStatus, age: i32) -> Vec<i32> {
    if fs == FilingStatus::MarriedJoint {
        vec![age, age - 2]
    } else {
        vec![age]
    }
}

#[test]
fn more_income_never_means_less_state_tax() {
    let y = StateTaxYear::y2026();
    let codes: Vec<String> = y.codes().map(str::to_string).collect();
    assert!(codes.len() >= 50);
    for code in &codes {
        for fs in STATUSES {
            for age in [50, 63, 68, 80] {
                let r = household(&ages(fs, age));
                let base = 105_000 + 34_000; // retirement income plus the taxable part of benefits
                let mut last = Money::ZERO;
                for extra in (0..400_000).step_by(5_000) {
                    let agi = m(base + extra);
                    let tax = y
                        .income_tax_with_retirement(code, fs, agi, m(34_000), &r)
                        .unwrap();
                    assert!(
                        tax >= last,
                        "{code} {fs:?} age {age}: tax fell from {last} to {tax} at AGI {agi}"
                    );
                    last = tax;
                }
            }
        }
    }
}

#[test]
fn retirement_rules_never_raise_tax_above_the_plain_schedule() {
    let y = StateTaxYear::y2026();
    for code in y.codes() {
        for fs in STATUSES {
            for age in [50, 63, 68, 80] {
                let r = household(&ages(fs, age));
                for agi in [60_000, 150_000, 400_000] {
                    let with = y
                        .income_tax_with_retirement(code, fs, m(agi), Money::ZERO, &r)
                        .unwrap();
                    let plain = y.income_tax(code, fs, m(agi)).unwrap();
                    assert!(
                        with <= plain,
                        "{code} {fs:?} age {age} AGI {agi}: {with} with the rules, {plain} without"
                    );
                }
            }
        }
    }
}

#[test]
fn no_household_pays_negative_state_tax() {
    let y = StateTaxYear::y2026();
    for code in y.codes() {
        for fs in STATUSES {
            let r = household(&ages(fs, 70));
            for agi in [0, 1_000, 20_000] {
                let t = y
                    .income_tax_with_retirement(code, fs, m(agi), m(agi.min(10_000)), &r)
                    .unwrap();
                assert!(t >= Money::ZERO, "{code} {fs:?} AGI {agi}: {t}");
            }
        }
    }
}
