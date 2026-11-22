//! What is left for heirs: federal estate tax and the income tax heirs pay on inherited
//! pre-tax accounts. All in today's dollars, where the estate tax exemption is indexed.
use crate::projection::Accounts;
use crate::tax::FilingStatus;
use fin_core::Money;

/// 2026 federal estate tax exemption per person ($15,000,000 under Public Law 119-21), and the
/// flat 40% rate on what is above it. A married couple is assumed to use both exemptions
/// (portability). Source: IRC section 2010 and 2001(c).
pub const EXEMPTION_PER_PERSON: i64 = 15_000_000;
pub const ESTATE_RATE: f64 = 0.40;
/// Heirs must empty an inherited IRA within ten years (SECURE Act), so the income lands on
/// top of their own.
pub const INHERITED_IRA_YEARS: i64 = 10;
/// A middle guess at the heirs' marginal income tax rate.
pub const DEFAULT_HEIR_RATE: f64 = 0.24;

#[derive(Debug, Clone, PartialEq)]
pub struct Legacy {
    pub gross: Money,
    pub exemption: Money,
    pub estate_tax: Money,
    /// Yearly amount heirs must take from the inherited pre-tax accounts for ten years.
    pub inherited_ira_per_year: Money,
    pub heir_income_tax: Money,
    pub net_to_heirs: Money,
}

/// The same figures in cents, for the screen.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyView {
    pub gross_cents: i64,
    pub exemption_cents: i64,
    pub estate_tax_cents: i64,
    pub inherited_ira_per_year_cents: i64,
    pub heir_income_tax_cents: i64,
    pub net_to_heirs_cents: i64,
}

impl From<&Legacy> for LegacyView {
    fn from(l: &Legacy) -> Self {
        LegacyView {
            gross_cents: l.gross.cents(),
            exemption_cents: l.exemption.cents(),
            estate_tax_cents: l.estate_tax.cents(),
            inherited_ira_per_year_cents: l.inherited_ira_per_year.cents(),
            heir_income_tax_cents: l.heir_income_tax.cents(),
            net_to_heirs_cents: l.net_to_heirs.cents(),
        }
    }
}

pub fn exemption(fs: FilingStatus) -> Money {
    let people = if fs == FilingStatus::MarriedJoint {
        2
    } else {
        1
    };
    Money::from_dollars(EXEMPTION_PER_PERSON * people)
}

pub fn estate_tax(fs: FilingStatus, gross: Money) -> Money {
    (gross - exemption(fs))
        .max(Money::ZERO)
        .mul_rate(ESTATE_RATE)
}

/// Settle the estate from the accounts left at the end of the plan.
/// Taxable investments get a stepped-up basis so carry no income tax; Roth is tax free;
/// pre-tax money is taxed to heirs at `heir_rate`.
pub fn settle(fs: FilingStatus, left: &Accounts, extra_assets: Money, heir_rate: f64) -> Legacy {
    let gross = left.cash + left.taxable + left.roth + left.traditional + extra_assets;
    let tax = estate_tax(fs, gross);
    // The estate tax comes out of the whole estate, so pre-tax money bears its share.
    let share = if gross > Money::ZERO {
        tax.cents() as f64 / gross.cents() as f64
    } else {
        0.0
    };
    let traditional_after_estate = left.traditional.mul_rate(1.0 - share);
    let heir_tax = traditional_after_estate.mul_rate(heir_rate.clamp(0.0, 0.5));
    Legacy {
        gross,
        exemption: exemption(fs),
        estate_tax: tax,
        inherited_ira_per_year: Money::from_cents(
            traditional_after_estate.cents() / INHERITED_IRA_YEARS,
        ),
        heir_income_tax: heir_tax,
        net_to_heirs: gross - tax - heir_tax,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn accounts(cash: i64, taxable: i64, roth: i64, trad: i64) -> Accounts {
        Accounts {
            cash: Money::from_dollars(cash),
            taxable: Money::from_dollars(taxable),
            taxable_gain_share: 0.3,
            traditional: Money::from_dollars(trad),
            roth: Money::from_dollars(roth),
        }
    }

    #[test]
    fn small_estates_owe_no_estate_tax() {
        let l = settle(
            FilingStatus::Single,
            &accounts(10_000, 500_000, 100_000, 400_000),
            Money::ZERO,
            0.24,
        );
        assert_eq!(l.estate_tax, Money::ZERO);
        // Only the pre-tax part is taxed to heirs.
        assert_eq!(l.heir_income_tax, Money::from_dollars(96_000));
        assert_eq!(l.net_to_heirs, Money::from_dollars(1_010_000 - 96_000));
        assert_eq!(l.inherited_ira_per_year, Money::from_dollars(40_000));
    }

    #[test]
    fn a_couple_gets_two_exemptions() {
        assert_eq!(
            exemption(FilingStatus::MarriedJoint),
            Money::from_dollars(30_000_000)
        );
        assert_eq!(
            exemption(FilingStatus::Single),
            Money::from_dollars(15_000_000)
        );
        assert_eq!(
            estate_tax(FilingStatus::Single, Money::from_dollars(20_000_000)),
            Money::from_dollars(2_000_000)
        );
        assert_eq!(
            estate_tax(FilingStatus::MarriedJoint, Money::from_dollars(20_000_000)),
            Money::ZERO
        );
    }

    #[test]
    fn the_estate_tax_is_shared_with_the_pre_tax_money() {
        let l = settle(
            FilingStatus::Single,
            &accounts(0, 10_000_000, 0, 10_000_000),
            Money::ZERO,
            0.30,
        );
        // 20M estate: 5M over the exemption at 40% = 2M, a tenth of the estate.
        assert_eq!(l.estate_tax, Money::from_dollars(2_000_000));
        assert_eq!(
            l.heir_income_tax,
            Money::from_dollars(9_000_000).mul_rate(0.30)
        );
        assert!(l.net_to_heirs < l.gross - l.estate_tax);
    }

    #[test]
    fn nothing_left_means_nothing_owed() {
        let l = settle(
            FilingStatus::Single,
            &accounts(0, 0, 0, 0),
            Money::ZERO,
            0.24,
        );
        assert_eq!(l.net_to_heirs, Money::ZERO);
        assert_eq!(l.estate_tax, Money::ZERO);
    }

    #[test]
    fn heir_rate_is_clamped() {
        let a = accounts(0, 0, 0, 100_000);
        let l = settle(FilingStatus::Single, &a, Money::ZERO, 5.0);
        assert_eq!(l.heir_income_tax, Money::from_dollars(50_000));
    }
}
