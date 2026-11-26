//! Plain arithmetic for equity pay: what an RSU vest or an ESPP purchase is worth and how many
//! shares are left after taxes are covered. Figures are estimates, not tax advice.
use fin_core::Money;
use serde::Serialize;

/// Federal supplemental withholding rate for vests under $1 million.
pub const SUPPLEMENTAL_RATE: f64 = 0.22;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VestResult {
    /// Shares times the price on the vest date (ordinary income).
    pub value_cents: i64,
    /// Tax held back at vest.
    pub withheld_cents: i64,
    /// Whole shares sold to cover the withholding.
    pub shares_sold: u64,
    pub shares_kept: u64,
    /// What the sale of those shares raised beyond the withholding (kept as cash).
    pub cash_back_cents: i64,
}

/// A vest taxed at `federal_rate` plus `other_rate` (Social Security, Medicare, state), with
/// enough whole shares sold at `price` to cover it. Rates are fractions such as 0.22.
pub fn rsu_vest(
    shares: u64,
    price: Money,
    federal_rate: f64,
    other_rate: f64,
) -> Result<VestResult, String> {
    if price.is_negative() || price.is_zero() {
        return Err("The share price must be more than zero.".into());
    }
    let rate = federal_rate + other_rate;
    if !(0.0..1.0).contains(&rate) {
        return Err("Tax rates must add up to between 0% and 100%.".into());
    }
    let value = price.mul_ratio(shares as i64, 1);
    let withheld = value.mul_rate(rate);
    let per = price.cents();
    let sold = if withheld.is_zero() {
        0
    } else {
        ((withheld.cents() + per - 1) / per) as u64
    }
    .min(shares);
    let cash_back = (price.mul_ratio(sold as i64, 1) - withheld).cents().max(0);
    Ok(VestResult {
        value_cents: value.cents(),
        withheld_cents: withheld.cents(),
        shares_sold: sold,
        shares_kept: shares - sold,
        cash_back_cents: cash_back,
    })
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EsppResult {
    pub purchase_price_cents: i64,
    pub shares: u64,
    /// Cash left over because only whole shares are bought.
    pub leftover_cents: i64,
    /// Market value of the shares minus what was paid, the day they are bought.
    pub discount_gain_cents: i64,
}

/// `discount` is a fraction such as 0.15. With `lookback` the price is the lower of the offering
/// date and purchase date prices, before the discount.
pub fn espp_purchase(
    contributions: Money,
    offering_price: Money,
    purchase_price: Money,
    discount: f64,
    lookback: bool,
) -> Result<EsppResult, String> {
    if !(0.0..=0.15).contains(&discount) {
        return Err("The ESPP discount can be at most 15%.".into());
    }
    if offering_price.cents() <= 0 || purchase_price.cents() <= 0 {
        return Err("Share prices must be more than zero.".into());
    }
    if contributions.is_negative() {
        return Err("Contributions can't be negative.".into());
    }
    let basis = if lookback && offering_price < purchase_price {
        offering_price
    } else {
        purchase_price
    };
    let pay = basis.mul_rate(1.0 - discount);
    let shares = (contributions.cents() / pay.cents()) as u64;
    let spent = pay.mul_ratio(shares as i64, 1);
    Ok(EsppResult {
        purchase_price_cents: pay.cents(),
        shares,
        leftover_cents: (contributions - spent).cents(),
        discount_gain_cents: (purchase_price.mul_ratio(shares as i64, 1) - spent).cents(),
    })
}

/// A sale is a "qualifying disposition" when it is more than two years after the offering date
/// and more than one year after the purchase date. Dates are `(year, month, day)`.
pub fn espp_qualifying(
    offering: (i32, u32, u32),
    purchase: (i32, u32, u32),
    sale: (i32, u32, u32),
) -> bool {
    let after = |from: (i32, u32, u32), years: i32| sale > (from.0 + years, from.1, from.2);
    after(offering, 2) && after(purchase, 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(n: i64) -> Money {
        Money::from_dollars(n)
    }

    #[test]
    fn a_vest_sells_just_enough_shares_to_cover_tax() {
        // 100 shares at $50 = $5,000; 22% + 7.65% + 5% = 34.65% = $1,732.50 -> 35 shares ($1,750).
        let v = rsu_vest(100, d(50), SUPPLEMENTAL_RATE, 0.1265).unwrap();
        assert_eq!(v.value_cents, 500_000);
        assert_eq!(v.withheld_cents, 173_250);
        assert_eq!(v.shares_sold, 35);
        assert_eq!(v.shares_kept, 65);
        assert_eq!(v.cash_back_cents, 1_750);
    }

    #[test]
    fn rsu_edge_cases() {
        let zero = rsu_vest(0, d(10), 0.22, 0.1).unwrap();
        assert_eq!((zero.shares_sold, zero.shares_kept), (0, 0));
        let none = rsu_vest(10, d(10), 0.0, 0.0).unwrap();
        assert_eq!((none.shares_sold, none.shares_kept), (0, 10));
        assert!(rsu_vest(10, d(0), 0.2, 0.1).is_err());
        assert!(rsu_vest(10, d(10), 0.7, 0.4).is_err());
    }

    #[test]
    fn espp_with_lookback_uses_the_lower_price() {
        // $10,000 saved, $40 at offering, $50 at purchase, 15% off the lower: $34 a share.
        let e = espp_purchase(d(10_000), d(40), d(50), 0.15, true).unwrap();
        assert_eq!(e.purchase_price_cents, 3_400);
        assert_eq!(e.shares, 294);
        assert_eq!(e.leftover_cents, 1_000_000 - 294 * 3_400);
        assert_eq!(e.discount_gain_cents, 294 * 5_000 - 294 * 3_400);
        // Without lookback the discount comes off the purchase price.
        let n = espp_purchase(d(10_000), d(40), d(50), 0.15, false).unwrap();
        assert_eq!(n.purchase_price_cents, 4_250);
        // A falling price: lookback does not help.
        let f = espp_purchase(d(1_000), d(60), d(50), 0.15, true).unwrap();
        assert_eq!(f.purchase_price_cents, 4_250);
    }

    #[test]
    fn espp_rejects_bad_input() {
        assert!(espp_purchase(d(100), d(10), d(10), 0.2, true).is_err());
        assert!(espp_purchase(d(100), d(0), d(10), 0.1, true).is_err());
        assert!(espp_purchase(d(-1), d(10), d(10), 0.1, true).is_err());
    }

    #[test]
    fn qualifying_needs_both_holding_periods() {
        assert!(espp_qualifying((2023, 1, 1), (2023, 7, 1), (2025, 7, 2)));
        assert!(!espp_qualifying((2023, 7, 1), (2023, 12, 1), (2025, 6, 30)));
        assert!(!espp_qualifying((2023, 1, 1), (2024, 7, 1), (2025, 7, 1)));
    }
}
