//! Mortgage offset account: what a savings balance held against the loan saves.
use fin_core::Money;
use fin_plan::housing::offset_payoff;
use serde::Serialize;

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OffsetEffect {
    pub months_without: i32,
    pub months_with: i32,
    pub interest_without_cents: i64,
    pub interest_with_cents: i64,
    pub interest_saved_cents: i64,
}

/// Compare the loan with and without an offset balance. The rate is in basis points.
pub fn mortgage_offset(
    balance_cents: i64,
    rate_bp: i64,
    years_left: i32,
    offset_cents: i64,
) -> Result<OffsetEffect, String> {
    if balance_cents <= 0 {
        return Err("Enter what is still owed on the mortgage.".into());
    }
    if !(0..=3000).contains(&rate_bp) {
        return Err("Enter the yearly interest rate between 0 and 30 percent.".into());
    }
    if !(1..=50).contains(&years_left) {
        return Err("Enter the years left as a number from 1 to 50.".into());
    }
    let b = Money::from_cents(balance_cents);
    let rate = rate_bp as f64 / 10_000.0;
    let (m0, i0) = offset_payoff(b, rate, years_left, Money::ZERO);
    let (m1, i1) = offset_payoff(b, rate, years_left, Money::from_cents(offset_cents.max(0)));
    Ok(OffsetEffect {
        months_without: m0,
        months_with: m1,
        interest_without_cents: i0.cents(),
        interest_with_cents: i1.cents(),
        interest_saved_cents: (i0.cents() - i1.cents()).max(0),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offset_saves_interest_and_time() {
        let e = mortgage_offset(30_000_000, 450, 25, 6_000_000).unwrap();
        assert!(e.months_with < e.months_without);
        assert!(e.interest_saved_cents > 0);
        assert_eq!(
            e.interest_saved_cents,
            e.interest_without_cents - e.interest_with_cents
        );
    }

    #[test]
    fn zero_offset_saves_nothing() {
        let e = mortgage_offset(10_000_000, 400, 10, 0).unwrap();
        assert_eq!(e.interest_saved_cents, 0);
        assert_eq!(e.months_with, e.months_without);
    }

    #[test]
    fn bad_inputs_are_explained() {
        assert!(mortgage_offset(0, 400, 10, 0).is_err());
        assert!(mortgage_offset(100_000, 5000, 10, 0).is_err());
        assert!(mortgage_offset(100_000, 400, 0, 0).is_err());
    }
}
