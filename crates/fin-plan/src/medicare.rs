//! Medicare Part B and Part D premiums, including the income-related surcharge (IRMAA).
//!
//! Amounts are the 2026 monthly figures from CMS. The plan runs in today's dollars, so they
//! are held flat in real terms. IRMAA looks at income from two years earlier; the projection
//! passes in the modified AGI it has for that year.

use crate::tax::FilingStatus;
use fin_core::Money;

/// 2026 standard Part B premium per month.
const PART_B_BASE: i64 = 20_290;
/// Average Part D base premium per month (the national base beneficiary premium).
const PART_D_BASE: i64 = 3_899;

/// `(MAGI above (single), MAGI above (joint), Part B per month, Part D surcharge per month)`
/// in cents, from the lowest surcharge tier up.
const TIERS: [(i64, i64, i64, i64); 5] = [
    (10_900_000, 21_800_000, 28_410, 1_450),
    (13_700_000, 27_400_000, 40_580, 3_750),
    (17_100_000, 34_200_000, 52_750, 6_040),
    (20_500_000, 41_000_000, 64_920, 8_330),
    (50_000_000, 75_000_000, 68_990, 9_100),
];

/// Which IRMAA tier (0 = none, 1..=5) a modified AGI falls in.
pub fn irmaa_tier(filing: FilingStatus, magi: Money) -> usize {
    let joint = filing == FilingStatus::MarriedJoint;
    TIERS
        .iter()
        .filter(|t| magi.cents() > if joint { t.1 } else { t.0 })
        .count()
}

/// Yearly Part B plus Part D cost for one person on Medicare at this income.
pub fn annual_premium(filing: FilingStatus, magi: Money) -> Money {
    let (b, d) = match irmaa_tier(filing, magi) {
        0 => (PART_B_BASE, 0),
        n => (TIERS[n - 1].2, TIERS[n - 1].3),
    };
    Money::from_cents((b + PART_D_BASE + d) * 12)
}

/// Income at which the next IRMAA tier starts, if there is one.
pub fn next_cliff(filing: FilingStatus, magi: Money) -> Option<Money> {
    let joint = filing == FilingStatus::MarriedJoint;
    TIERS
        .get(irmaa_tier(filing, magi))
        .map(|t| Money::from_cents(if joint { t.1 } else { t.0 }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_premium_below_first_threshold() {
        let m = annual_premium(FilingStatus::Single, Money::from_dollars(100_000));
        assert_eq!(m, Money::from_cents((20_290 + 3_899) * 12));
    }

    #[test]
    fn crossing_a_cliff_by_a_dollar_raises_the_premium() {
        let at = annual_premium(FilingStatus::Single, Money::from_dollars(109_000));
        let over = annual_premium(FilingStatus::Single, Money::from_dollars(109_001));
        assert_eq!(over - at, Money::from_cents((28_410 - 20_290 + 1_450) * 12));
    }

    #[test]
    fn joint_thresholds_are_double() {
        assert_eq!(
            irmaa_tier(FilingStatus::MarriedJoint, Money::from_dollars(200_000)),
            0
        );
        assert_eq!(
            irmaa_tier(FilingStatus::Single, Money::from_dollars(200_000)),
            3
        );
        assert_eq!(
            irmaa_tier(FilingStatus::MarriedJoint, Money::from_dollars(800_000)),
            5
        );
    }

    #[test]
    fn top_tier_has_no_next_cliff() {
        assert!(next_cliff(FilingStatus::Single, Money::from_dollars(600_000)).is_none());
        assert_eq!(
            next_cliff(FilingStatus::Single, Money::from_dollars(50_000)),
            Some(Money::from_dollars(109_000))
        );
    }
}
