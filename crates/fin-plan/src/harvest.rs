//! Two income-planning limits for years before Medicare: how much long-term gain you can realize
//! at a 0% federal rate, and how far your income is from the ACA subsidy cliff.
use crate::tax::federal::{FederalInput, FederalTaxYear};
use crate::tax::FilingStatus;
use fin_core::Money;
use serde::Serialize;

/// 2025 federal poverty guideline for one person, which sets 2026 marketplace subsidies, and
/// the step for each extra person (48 contiguous states). Source: HHS 2025 poverty guidelines.
pub const FPL_ONE_PERSON: i64 = 15_650;
pub const FPL_EXTRA_PERSON: i64 = 5_500;
/// With the enhanced credits gone after 2025, subsidies end above 400% of the poverty line.
pub const ACA_CLIFF_PERCENT: i64 = 400;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HarvestPlan {
    /// Long-term gain that still faces a 0% federal rate on top of your ordinary income.
    pub zero_rate_room_cents: i64,
    /// Federal tax avoided by realizing that much gain now rather than at 15%.
    pub tax_saved_cents: i64,
    /// Marketplace income limit for this household size.
    pub aca_cliff_cents: i64,
    /// How far your income (with the gain realized) is under the cliff; negative when over it.
    pub aca_room_cents: i64,
    /// Whether realizing the full 0% room would push you over the ACA cliff.
    pub gain_crosses_aca_cliff: bool,
}

pub fn aca_cliff(household_size: i64) -> Money {
    let size = household_size.max(1);
    let fpl = FPL_ONE_PERSON + FPL_EXTRA_PERSON * (size - 1);
    Money::from_dollars(fpl * ACA_CLIFF_PERCENT / 100)
}

/// `ordinary_income` is wages, interest, IRA withdrawals and the like before deductions;
/// `existing_gains` are long-term gains and qualified dividends already counted this year.
pub fn plan(
    fs: FilingStatus,
    ordinary_income: Money,
    existing_gains: Money,
    household_size: i64,
) -> Result<HarvestPlan, String> {
    if ordinary_income < Money::ZERO || existing_gains < Money::ZERO {
        return Err("Income and gains cannot be negative.".into());
    }
    if !(1..=10).contains(&household_size) {
        return Err("Household size should be between 1 and 10.".into());
    }
    let fed = FederalTaxYear::y2026();
    let tax = |gains: Money| {
        fed.income_tax(
            fs,
            FederalInput {
                ordinary_income,
                long_term_gains: gains,
                ..Default::default()
            },
        )
    };
    // Largest extra gain with no gains tax, by bisection (tax is monotone in gains).
    let base = tax(existing_gains).ltcg_tax;
    let (mut lo, mut hi) = (0i64, 500_000_000_i64);
    while hi - lo > 100 {
        let mid = (lo + hi) / 2;
        if tax(existing_gains + Money::from_cents(mid)).ltcg_tax <= base {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let room = Money::from_cents(lo / 100 * 100);
    let saved = room.mul_rate(0.15);
    let cliff = aca_cliff(household_size);
    let agi_with = tax(existing_gains + room).agi;
    Ok(HarvestPlan {
        zero_rate_room_cents: room.cents(),
        tax_saved_cents: saved.cents(),
        aca_cliff_cents: cliff.cents(),
        aca_room_cents: (cliff - agi_with).cents(),
        gain_crosses_aca_cliff: agi_with > cliff,
    })
}

/// The same in cents, for the app's commands.
pub fn plan_cents(
    fs: FilingStatus,
    ordinary_income_cents: i64,
    existing_gains_cents: i64,
    household_size: i64,
) -> Result<HarvestPlan, String> {
    plan(
        fs,
        Money::from_cents(ordinary_income_cents),
        Money::from_cents(existing_gains_cents),
        household_size,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(n: i64) -> Money {
        Money::from_dollars(n)
    }

    #[test]
    fn the_zero_rate_room_fills_the_gap_to_the_bracket_top() {
        // Joint, no income: the standard deduction plus the 0% band is the room.
        let p = plan(FilingStatus::MarriedJoint, d(0), d(0), 2).unwrap();
        assert!(p.zero_rate_room_cents > d(90_000).cents(), "{p:?}");
        // More ordinary income leaves less room.
        let q = plan(FilingStatus::MarriedJoint, d(60_000), d(0), 2).unwrap();
        assert!(q.zero_rate_room_cents < p.zero_rate_room_cents);
        assert!(q.zero_rate_room_cents > 0);
    }

    #[test]
    fn no_room_when_income_is_already_above_the_zero_band() {
        let p = plan(FilingStatus::Single, d(150_000), d(0), 1).unwrap();
        assert_eq!(p.zero_rate_room_cents, 0);
        assert_eq!(p.tax_saved_cents, 0);
    }

    #[test]
    fn gains_already_realized_use_up_room() {
        let a = plan(FilingStatus::Single, d(20_000), d(0), 1).unwrap();
        let b = plan(FilingStatus::Single, d(20_000), d(10_000), 1).unwrap();
        assert!(b.zero_rate_room_cents < a.zero_rate_room_cents);
    }

    #[test]
    fn the_aca_cliff_scales_with_household_size() {
        assert_eq!(aca_cliff(1), d(62_600));
        assert_eq!(aca_cliff(2), d(84_600));
        assert_eq!(aca_cliff(0), aca_cliff(1));
    }

    #[test]
    fn harvesting_can_push_income_over_the_aca_cliff() {
        let p = plan(FilingStatus::Single, d(40_000), d(0), 1).unwrap();
        // Single 0% band tops out near $49k of taxable income, plus the deduction: ~$65k AGI.
        assert!(p.gain_crosses_aca_cliff, "{p:?}");
        let q = plan(FilingStatus::Single, d(10_000), d(0), 1).unwrap();
        assert!(q.aca_room_cents != 0);
    }

    #[test]
    fn bad_input_is_a_readable_error() {
        assert!(plan(FilingStatus::Single, d(-1), d(0), 1).is_err());
        assert!(plan(FilingStatus::Single, d(1), d(0), 0).is_err());
    }
}
