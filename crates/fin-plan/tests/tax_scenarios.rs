//! Integration: whole-household tax scenarios using only the public API.
use fin_core::Money;
use fin_plan::tax::{FederalInput, FederalTaxYear, FilingStatus};

fn m(d: i64) -> Money {
    Money::from_dollars(d)
}

#[test]
fn marriage_bonus_for_one_earner_household() {
    let y = FederalTaxYear::y2026();
    let wages = FederalInput {
        ordinary_income: m(150_000),
        ..Default::default()
    };
    let single = y.income_tax(FilingStatus::Single, wages).income_tax;
    let joint = y.income_tax(FilingStatus::MarriedJoint, wages).income_tax;
    assert!(
        joint < single,
        "joint {joint} should be lower than single {single}"
    );
}

#[test]
fn every_status_is_monotonic_in_income() {
    let y = FederalTaxYear::y2026();
    for fs in FilingStatus::ALL {
        let mut prev = Money::ZERO;
        for k in 0..60 {
            let r = y.income_tax(
                fs,
                FederalInput {
                    ordinary_income: m(k * 10_000),
                    ..Default::default()
                },
            );
            assert!(r.income_tax >= prev, "{fs:?} at {k}");
            prev = r.income_tax;
        }
    }
}

#[test]
fn effective_rate_stays_below_marginal() {
    let y = FederalTaxYear::y2026();
    let r = y.income_tax(
        FilingStatus::MarriedJoint,
        FederalInput {
            ordinary_income: m(400_000),
            ..Default::default()
        },
    );
    let eff = r.income_tax.cents() as f64 / 40_000_000.0;
    assert!(eff < r.marginal_rate);
    assert_eq!(r.marginal_rate, 0.24);
}

#[test]
fn retirement_withdrawal_plus_gains_fill_brackets() {
    let y = FederalTaxYear::y2026();
    // Retiree couple: 60k IRA withdrawals + 40k gains.
    let r = y.income_tax(
        FilingStatus::MarriedJoint,
        FederalInput {
            ordinary_income: m(60_000),
            long_term_gains: m(40_000),
            ..Default::default()
        },
    );
    // Taxable = 100k - 32.2k = 67.8k; ordinary taxable 27.8k; gains stack to 67.8k < 98.9k → 0%.
    assert_eq!(r.taxable_income, m(67_800));
    assert_eq!(r.ltcg_tax, Money::ZERO);
}

#[test]
fn custom_year_json_can_replace_bundled_data() {
    let json =
        include_str!("../data/federal_2026.json").replace("\"year\": 2026", "\"year\": 2099");
    let y = FederalTaxYear::from_json(&json).unwrap();
    assert_eq!(y.year, 2099);
}
