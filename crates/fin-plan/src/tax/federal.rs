use super::brackets::BracketTable;
use fin_core::Money;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilingStatus {
    Single,
    MarriedJoint,
    MarriedSeparate,
    HeadOfHousehold,
}

impl FilingStatus {
    pub const ALL: [FilingStatus; 4] = [
        FilingStatus::Single,
        FilingStatus::MarriedJoint,
        FilingStatus::MarriedSeparate,
        FilingStatus::HeadOfHousehold,
    ];
}

#[derive(Debug, Clone, Deserialize)]
struct Fica {
    ss_wage_base: Money,
    ss_rate: f64,
    medicare_rate: f64,
    addl_medicare_rate: f64,
    addl_medicare_threshold: HashMap<FilingStatus, Money>,
}

/// One year of federal tax rules loaded from a JSON data file.
#[derive(Debug, Clone, Deserialize)]
pub struct FederalTaxYear {
    pub year: i32,
    pub source: String,
    standard_deduction: HashMap<FilingStatus, Money>,
    brackets: HashMap<FilingStatus, BracketTable>,
    ltcg: HashMap<FilingStatus, BracketTable>,
    fica: Fica,
}

/// Annual figures for a household's federal return (all in cents).
#[derive(Debug, Clone, Copy, Default)]
pub struct FederalInput {
    /// Wages, interest, pensions, IRA withdrawals, taxable Social Security etc.
    pub ordinary_income: Money,
    /// Qualified dividends and long-term capital gains.
    pub long_term_gains: Money,
    /// Above-the-line adjustments (401(k) pre-tax, HSA, IRA).
    pub adjustments: Money,
    /// Itemized deductions; the larger of this and the standard deduction is used.
    pub itemized: Money,
    /// Wages subject to FICA (all earners combined; use `fica_for_wages` per earner for exact SS cap).
    pub wages: Money,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FederalResult {
    pub agi: Money,
    pub deduction: Money,
    pub taxable_income: Money,
    pub ordinary_tax: Money,
    pub ltcg_tax: Money,
    pub income_tax: Money,
    pub marginal_rate: f64,
}

impl FederalTaxYear {
    /// Rules for 2026, bundled with the app.
    pub fn y2026() -> Self {
        serde_json::from_str(include_str!("../../data/federal_2026.json"))
            .expect("bundled 2026 data")
    }

    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }

    pub fn standard_deduction(&self, fs: FilingStatus) -> Money {
        self.standard_deduction[&fs]
    }

    pub fn brackets(&self, fs: FilingStatus) -> &BracketTable {
        &self.brackets[&fs]
    }

    pub fn income_tax(&self, fs: FilingStatus, i: FederalInput) -> FederalResult {
        let agi = (i.ordinary_income + i.long_term_gains - i.adjustments).max(Money::ZERO);
        let deduction = self.standard_deduction(fs).max(i.itemized);
        let taxable = (agi - deduction).max(Money::ZERO);
        // Gains sit on top of ordinary income; deduction first absorbs ordinary income.
        let ordinary_taxable = (i.ordinary_income - i.adjustments - deduction).max(Money::ZERO);
        let gains_taxable = i
            .long_term_gains
            .min(taxable - ordinary_taxable.min(taxable))
            .max(Money::ZERO);
        let ordinary_tax = self.brackets[&fs].tax(ordinary_taxable);
        let ltcg_tax = self.ltcg[&fs].tax_stacked(ordinary_taxable, gains_taxable);
        FederalResult {
            agi,
            deduction,
            taxable_income: taxable,
            ordinary_tax,
            ltcg_tax,
            income_tax: ordinary_tax + ltcg_tax,
            marginal_rate: self.brackets[&fs].marginal_rate(ordinary_taxable),
        }
    }

    /// Employee-side FICA for one earner: Social Security to the wage base plus Medicare,
    /// plus the additional Medicare tax over the filing-status threshold.
    pub fn fica_for_wages(&self, fs: FilingStatus, wages: Money) -> Money {
        let f = &self.fica;
        let wages = wages.max(Money::ZERO);
        let ss = wages.min(f.ss_wage_base).mul_rate(f.ss_rate);
        let medicare = wages.mul_rate(f.medicare_rate);
        let over = (wages - f.addl_medicare_threshold[&fs]).max(Money::ZERO);
        ss + medicare + over.mul_rate(f.addl_medicare_rate)
    }
}

/// The part of Social Security benefits that is taxable, by the IRS provisional-income rule.
/// `other_income` is everything on the return except Social Security (wages, withdrawals,
/// gains, interest). The thresholds are set in law and are not adjusted for inflation.
pub fn taxable_social_security(fs: FilingStatus, ss: Money, other_income: Money) -> Money {
    if ss <= Money::ZERO {
        return Money::ZERO;
    }
    let (lower, upper, tier) = match fs {
        FilingStatus::Single | FilingStatus::HeadOfHousehold => (25_000, 34_000, 4_500),
        FilingStatus::MarriedJoint => (32_000, 44_000, 6_000),
        // Married filing separately (living with a spouse): no base amount at all.
        FilingStatus::MarriedSeparate => (0, 0, 0),
    };
    let (lower, upper, tier) = (
        Money::from_dollars(lower),
        Money::from_dollars(upper),
        Money::from_dollars(tier),
    );
    let provisional = other_income.max(Money::ZERO) + ss.mul_rate(0.5);
    if provisional <= lower {
        return Money::ZERO;
    }
    let cap = ss.mul_rate(0.85);
    let half = ss.mul_rate(0.5);
    if provisional <= upper {
        return (provisional - lower).mul_rate(0.5).min(half);
    }
    ((provisional - upper).mul_rate(0.85) + half.min(tier)).min(cap)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(d: i64) -> Money {
        Money::from_dollars(d)
    }

    #[test]
    fn data_file_loads_for_every_status() {
        let y = FederalTaxYear::y2026();
        assert_eq!(y.year, 2026);
        for fs in FilingStatus::ALL {
            assert!(y.standard_deduction(fs) > Money::ZERO);
            assert_eq!(y.brackets(fs).0.len(), 7);
        }
    }

    #[test]
    fn single_100k_wages() {
        let y = FederalTaxYear::y2026();
        let r = y.income_tax(
            FilingStatus::Single,
            FederalInput {
                ordinary_income: m(100_000),
                ..Default::default()
            },
        );
        assert_eq!(r.taxable_income, m(100_000 - 16_100));
        // 10% of 12,400 + 12% of 38,000 + 22% of 33,500
        assert_eq!(r.income_tax, Money::from_cents(124_000 + 456_000 + 737_000));
        assert_eq!(r.marginal_rate, 0.22);
    }

    #[test]
    fn mfj_200k() {
        let y = FederalTaxYear::y2026();
        let r = y.income_tax(
            FilingStatus::MarriedJoint,
            FederalInput {
                ordinary_income: m(200_000),
                ..Default::default()
            },
        );
        assert_eq!(r.taxable_income, m(167_800));
        // 10% 24,800 + 12% 76,000 + 22% 67,000 = 2,480 + 9,120 + 14,740
        assert_eq!(r.income_tax, m(2_480 + 9_120 + 14_740));
    }

    #[test]
    fn low_income_owes_nothing() {
        let y = FederalTaxYear::y2026();
        let r = y.income_tax(
            FilingStatus::Single,
            FederalInput {
                ordinary_income: m(15_000),
                ..Default::default()
            },
        );
        assert_eq!(r.income_tax, Money::ZERO);
    }

    #[test]
    fn gains_in_zero_bracket() {
        let y = FederalTaxYear::y2026();
        // MFJ with 60k wages and 30k gains: taxable ordinary = 27.8k, gains stack to 57.8k < 98.9k → 0%.
        let r = y.income_tax(
            FilingStatus::MarriedJoint,
            FederalInput {
                ordinary_income: m(60_000),
                long_term_gains: m(30_000),
                ..Default::default()
            },
        );
        assert_eq!(r.ltcg_tax, Money::ZERO);
        assert!(r.ordinary_tax > Money::ZERO);
    }

    #[test]
    fn gains_stack_into_15_percent() {
        let y = FederalTaxYear::y2026();
        let r = y.income_tax(
            FilingStatus::Single,
            FederalInput {
                ordinary_income: m(66_100),
                long_term_gains: m(20_000),
                ..Default::default()
            },
        );
        // ordinary taxable 50,000; gains from 50k: 0% up to 49,450 is already passed → all 15%.
        assert_eq!(r.ltcg_tax, m(3_000));
    }

    #[test]
    fn itemized_beats_standard_when_larger() {
        let y = FederalTaxYear::y2026();
        let r = y.income_tax(
            FilingStatus::Single,
            FederalInput {
                ordinary_income: m(100_000),
                itemized: m(30_000),
                ..Default::default()
            },
        );
        assert_eq!(r.deduction, m(30_000));
    }

    #[test]
    fn adjustments_reduce_agi() {
        let y = FederalTaxYear::y2026();
        let r = y.income_tax(
            FilingStatus::Single,
            FederalInput {
                ordinary_income: m(100_000),
                adjustments: m(23_500),
                ..Default::default()
            },
        );
        assert_eq!(r.agi, m(76_500));
    }

    #[test]
    fn fica_caps_social_security() {
        let y = FederalTaxYear::y2026();
        let low = y.fica_for_wages(FilingStatus::Single, m(100_000));
        assert_eq!(low, Money::from_cents(620_000 + 145_000));
        let high = y.fica_for_wages(FilingStatus::Single, m(300_000));
        // SS: 6.2% * 184,500 = 11,439; Medicare 1.45% * 300k = 4,350; addl 0.9% * 100k = 900
        assert_eq!(high, m(11_439 + 4_350 + 900));
    }

    #[test]
    fn taxable_social_security_follows_the_provisional_income_tiers() {
        let single = FilingStatus::Single;
        // Under $25,000 of provisional income nothing is taxed.
        assert_eq!(
            taxable_social_security(single, m(20_000), m(10_000)),
            Money::ZERO
        );
        // $20k benefits + $20k other: PI 30k, so half of the $5k over 25k.
        assert_eq!(
            taxable_social_security(single, m(20_000), m(20_000)),
            m(2_500)
        );
        // $24k benefits + $40k other: PI 52k. 0.85*18k + 4.5k = 19.8k, under the 20.4k cap.
        assert_eq!(
            taxable_social_security(single, m(24_000), m(40_000)),
            m(19_800)
        );
        // Plenty of other income hits the 85% cap.
        assert_eq!(
            taxable_social_security(single, m(24_000), m(200_000)),
            m(20_400)
        );
        // Married filing jointly uses 32k/44k and a 6k middle tier.
        let joint = FilingStatus::MarriedJoint;
        assert_eq!(
            taxable_social_security(joint, m(30_000), m(15_000)),
            Money::ZERO
        );
        assert_eq!(
            taxable_social_security(joint, m(30_000), m(30_000)),
            m(6_850)
        );
        assert_eq!(
            taxable_social_security(joint, m(30_000), m(60_000)),
            m(25_500)
        );
        // No benefits, no tax.
        assert_eq!(
            taxable_social_security(single, Money::ZERO, m(90_000)),
            Money::ZERO
        );
        // Married filing separately: taxed from the first dollar.
        assert_eq!(
            taxable_social_security(FilingStatus::MarriedSeparate, m(20_000), m(10_000)),
            m(17_000)
        );
    }
}
