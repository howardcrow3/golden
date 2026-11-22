//! Donor-advised fund "bunching": give several years of charity in one year to itemize that
//! year, then take the standard deduction in the years between.
use crate::tax::federal::{FederalInput, FederalTaxYear};
use crate::tax::FilingStatus;
use fin_core::Money;
use serde::Serialize;

/// Cash gifts to a donor-advised fund are deductible up to 60% of AGI (IRC 170(b)(1)(G)).
pub const CASH_LIMIT: f64 = 0.60;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Bunching {
    pub years: i32,
    pub tax_each_year_cents: i64,
    pub tax_bunched_cents: i64,
    pub saved_cents: i64,
    /// Whether each year on its own already beats the standard deduction (then bunching helps little).
    pub itemizes_anyway: bool,
}

/// Compare giving `giving` every year with giving `years` years at once into a donor-advised fund
/// in the first year and nothing after. `ordinary_income` is taxable income before deductions and
/// `other_itemized` is everything else you would itemize (mortgage interest, capped state taxes).
pub fn bunching(
    fs: FilingStatus,
    ordinary_income: Money,
    other_itemized: Money,
    giving: Money,
    years: i32,
) -> Result<Bunching, String> {
    if !(2..=10).contains(&years) {
        return Err("Bunching covers two to ten years.".into());
    }
    if giving <= Money::ZERO {
        return Err("Enter what you give each year.".into());
    }
    let fed = FederalTaxYear::y2026();
    let tax = |itemized: Money| {
        fed.income_tax(
            fs,
            FederalInput {
                ordinary_income,
                itemized,
                ..Default::default()
            },
        )
        .income_tax
    };
    let cap = ordinary_income.mul_rate(CASH_LIMIT);
    let each = tax(other_itemized + giving.min(cap));
    let base = tax(other_itemized);
    let big = tax(other_itemized + Money::from_cents(giving.cents() * years as i64).min(cap));
    let every = Money::from_cents(each.cents() * years as i64);
    let bunched = big + Money::from_cents(base.cents() * (years as i64 - 1));
    Ok(Bunching {
        years,
        tax_each_year_cents: every.cents(),
        tax_bunched_cents: bunched.cents(),
        saved_cents: (every - bunched).max(Money::ZERO).cents(),
        itemizes_anyway: other_itemized + giving > fed.standard_deduction(fs),
    })
}

/// The same in cents, for the app's commands.
pub fn bunching_cents(
    fs: FilingStatus,
    income_cents: i64,
    other_itemized_cents: i64,
    giving_cents: i64,
    years: i32,
) -> Result<Bunching, String> {
    bunching(
        fs,
        Money::from_cents(income_cents),
        Money::from_cents(other_itemized_cents),
        Money::from_cents(giving_cents),
        years,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(n: i64) -> Money {
        Money::from_dollars(n)
    }

    #[test]
    fn bunching_beats_even_giving_when_giving_alone_is_below_the_standard_deduction() {
        let b = bunching(
            FilingStatus::MarriedJoint,
            d(150_000),
            d(10_000),
            d(8_000),
            4,
        )
        .unwrap();
        assert!(b.saved_cents > 0, "{b:?}");
        assert!(!b.itemizes_anyway);
    }

    #[test]
    fn no_gain_when_already_itemizing_every_year() {
        let b = bunching(FilingStatus::Single, d(200_000), d(30_000), d(20_000), 2).unwrap();
        assert!(b.itemizes_anyway);
        assert_eq!(b.saved_cents, 0);
    }

    #[test]
    fn savings_never_exceed_the_tax_and_bad_input_is_rejected() {
        let b = bunching(FilingStatus::Single, d(90_000), d(0), d(5_000), 3).unwrap();
        assert!(b.saved_cents <= b.tax_each_year_cents);
        assert!(bunching(FilingStatus::Single, d(90_000), d(0), d(5_000), 1).is_err());
        assert!(bunching(FilingStatus::Single, d(90_000), d(0), d(0), 3).is_err());
    }

    #[test]
    fn the_sixty_percent_limit_caps_the_big_year() {
        // Gift of 10 years of 20k = 200k against 100k income: only 60k is deductible.
        let capped = bunching(FilingStatus::Single, d(100_000), d(0), d(20_000), 10).unwrap();
        let bigger_gift = bunching(FilingStatus::Single, d(100_000), d(0), d(40_000), 10).unwrap();
        // Both big years hit the same 60k cap, so the bunched tax is identical.
        assert_eq!(capped.tax_bunched_cents, bigger_gift.tax_bunched_cents);
    }
}
