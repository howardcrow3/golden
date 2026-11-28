//! Integration: properties the federal calculation must keep for every filing status.
use fin_core::Money;
use fin_plan::tax::{taxable_social_security, FederalInput, FederalTaxYear, FilingStatus};

fn m(d: i64) -> Money {
    Money::from_dollars(d)
}

fn input(ordinary: i64, gains: i64, seniors: u8) -> FederalInput {
    FederalInput {
        ordinary_income: m(ordinary),
        long_term_gains: m(gains),
        other_investment_income: m(gains / 10),
        seniors,
        senior_bonus: seniors > 0,
        ..Default::default()
    }
}

#[test]
fn more_ordinary_income_or_gains_never_lowers_federal_tax() {
    let y = FederalTaxYear::y2026();
    for fs in FilingStatus::ALL {
        for seniors in [0u8, 1, 2] {
            for gains in [0, 40_000, 300_000] {
                let mut last = Money::ZERO;
                for ord in (0..800_000).step_by(2_500) {
                    let t = y.income_tax(fs, input(ord, gains, seniors)).income_tax;
                    assert!(
                        t >= last,
                        "{fs:?} seniors {seniors} gains {gains}: fell at {ord}"
                    );
                    last = t;
                }
                let mut last = Money::ZERO;
                for g in (0..800_000).step_by(5_000) {
                    let t = y.income_tax(fs, input(80_000, g, seniors)).income_tax;
                    assert!(t >= last, "{fs:?} seniors {seniors}: fell at gains {g}");
                    last = t;
                }
            }
        }
    }
}

#[test]
fn federal_tax_is_never_more_than_the_income() {
    let y = FederalTaxYear::y2026();
    for fs in FilingStatus::ALL {
        for ord in [0, 10_000, 100_000, 1_000_000, 10_000_000] {
            let r = y.income_tax(fs, input(ord, ord / 2, 0));
            assert!(r.income_tax >= Money::ZERO);
            assert!(r.income_tax <= m(ord + ord / 2), "{fs:?} {ord}");
            assert!(
                (0.0..=0.5).contains(&r.marginal_rate),
                "{fs:?} {ord}: {}",
                r.marginal_rate
            );
        }
    }
}

#[test]
fn taxable_social_security_is_at_most_85_percent_and_grows_with_income() {
    for fs in FilingStatus::ALL {
        for ss in [0, 12_000, 30_000, 60_000] {
            let mut last = Money::ZERO;
            for other in (0..300_000).step_by(1_000) {
                let t = taxable_social_security(fs, m(ss), m(other));
                assert!(
                    t <= m(ss).mul_rate(0.85),
                    "{fs:?} ss {ss} other {other}: {t}"
                );
                assert!(t >= last, "{fs:?} ss {ss}: fell at {other}");
                last = t;
            }
        }
    }
}
