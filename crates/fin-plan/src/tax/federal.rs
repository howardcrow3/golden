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

mod extras;
pub use extras::{
    early_withdrawal_penalty, qbi_deduction, salt_deduction, sep_ira_limit, solo_401k_limit,
    SelfEmploymentTax,
};

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
    /// Extra standard deduction for each filer aged 65 or over. 2026: $2,050 if unmarried,
    /// $1,650 if married (IRS Rev. Proc. 2025-32 section 4.14(3), checked November 2026).
    #[serde(default)]
    aged_extra_deduction: HashMap<FilingStatus, Money>,
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
    /// Interest and other investment income not already counted as long-term gains; it only
    /// matters for the net investment income tax.
    pub other_investment_income: Money,
    /// How many of the filers (0, 1 or 2) are 65 or older.
    pub seniors: u8,
    /// Apply the temporary extra deduction for filers 65 and older (tax years 2025 to 2028).
    pub senior_bonus: bool,
    /// State and local tax inside `itemized`; AMT does not allow it.
    pub salt_in_itemized: Money,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FederalResult {
    pub agi: Money,
    pub deduction: Money,
    pub taxable_income: Money,
    pub ordinary_tax: Money,
    pub ltcg_tax: Money,
    /// Net investment income tax (3.8%), included in `income_tax`.
    pub niit: Money,
    /// Alternative minimum tax on top of the regular tax; already inside `income_tax`.
    pub amt: Money,
    pub income_tax: Money,
    pub marginal_rate: f64,
}

/// Statutory MAGI thresholds for the 3.8% net investment income tax (not inflation-indexed).
/// Source: IRS Topic no. 559, https://www.irs.gov/taxtopics/tc559 (checked November 2026).
fn niit_threshold(fs: FilingStatus) -> Money {
    Money::from_dollars(match fs {
        FilingStatus::MarriedJoint => 250_000,
        FilingStatus::MarriedSeparate => 125_000,
        FilingStatus::Single | FilingStatus::HeadOfHousehold => 200_000,
    })
}

/// The 2025-2028 extra deduction for filers 65+: $6,000 each, with each person's $6,000 reduced
/// by 6% of income over $75,000 ($150,000 joint), so a joint return with two seniors loses it
/// all at $250,000. Not available to married people filing separately (the IRS says
/// to file jointly if married). Source: IRS fact sheet FS-2025-03,
/// https://www.irs.gov/newsroom/one-big-beautiful-bill-act-tax-deductions-for-working-americans-and-seniors
/// for the amount, years and thresholds; the 6% rate is from the statute (the fact sheet omits it).
fn senior_bonus_deduction(fs: FilingStatus, seniors: u8, magi: Money) -> Money {
    let (start, ok) = match fs {
        FilingStatus::MarriedJoint => (150_000, true),
        FilingStatus::MarriedSeparate => (0, false),
        FilingStatus::Single | FilingStatus::HeadOfHousehold => (75_000, true),
    };
    if !ok {
        return Money::ZERO;
    }
    let cut = (magi - Money::from_dollars(start))
        .max(Money::ZERO)
        .mul_rate(0.06);
    let each = (Money::from_dollars(6_000) - cut).max(Money::ZERO);
    Money::from_cents(each.cents() * seniors as i64)
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

    /// How many of `seniors` the return can count: two on a joint return, otherwise one.
    fn seniors_on_return(fs: FilingStatus, seniors: u8) -> u8 {
        seniors.min(if fs == FilingStatus::MarriedJoint {
            2
        } else {
            1
        })
    }

    /// The standard deduction including the extra amount for each filer 65 or older.
    fn standard_with_aged(&self, fs: FilingStatus, seniors: u8) -> Money {
        let aged = self
            .aged_extra_deduction
            .get(&fs)
            .copied()
            .unwrap_or(Money::ZERO);
        let seniors = Self::seniors_on_return(fs, seniors);
        self.standard_deduction(fs) + Money::from_cents(aged.cents() * seniors as i64)
    }

    pub fn income_tax(&self, fs: FilingStatus, i: FederalInput) -> FederalResult {
        let agi = (i.ordinary_income + i.long_term_gains - i.adjustments).max(Money::ZERO);
        let seniors = Self::seniors_on_return(fs, i.seniors);
        let standard = self.standard_with_aged(fs, seniors);
        let mut deduction = standard.max(i.itemized);
        if i.senior_bonus {
            deduction += senior_bonus_deduction(fs, seniors, agi);
        }
        let taxable = (agi - deduction).max(Money::ZERO);
        // Gains sit on top of ordinary income; deduction first absorbs ordinary income.
        let ordinary_taxable = (i.ordinary_income - i.adjustments - deduction).max(Money::ZERO);
        let gains_taxable = i
            .long_term_gains
            .min(taxable - ordinary_taxable.min(taxable))
            .max(Money::ZERO);
        let ordinary_tax = self.brackets[&fs].tax(ordinary_taxable);
        let ltcg_tax = self.ltcg[&fs].tax_stacked(ordinary_taxable, gains_taxable);
        let net_investment = i.long_term_gains + i.other_investment_income;
        let niit = net_investment
            .min((agi - niit_threshold(fs)).max(Money::ZERO))
            .max(Money::ZERO)
            .mul_rate(0.038);
        let mut result = FederalResult {
            agi,
            deduction,
            taxable_income: taxable,
            ordinary_tax,
            ltcg_tax,
            niit,
            amt: Money::ZERO,
            income_tax: ordinary_tax + ltcg_tax + niit,
            marginal_rate: self.brackets[&fs].marginal_rate(ordinary_taxable),
        };
        result.amt = self.amt(fs, &i, &result, Money::ZERO);
        result.income_tax += result.amt;
        result
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

    #[test]
    fn niit_applies_to_investment_income_over_the_threshold() {
        let y = FederalTaxYear::y2026();
        // Single, $180k wages + $50k gains: AGI 230k is $30k over, so NIIT is 3.8% of $30k.
        let r = y.income_tax(
            FilingStatus::Single,
            FederalInput {
                ordinary_income: m(180_000),
                long_term_gains: m(50_000),
                ..Default::default()
            },
        );
        assert_eq!(r.niit, Money::from_cents(114_000));
        assert_eq!(r.income_tax, r.ordinary_tax + r.ltcg_tax + r.niit);
        // Only investment income counts: $20k of gains on $300k wages is taxed in full.
        let r = y.income_tax(
            FilingStatus::Single,
            FederalInput {
                ordinary_income: m(300_000),
                long_term_gains: m(20_000),
                ..Default::default()
            },
        );
        assert_eq!(r.niit, Money::from_cents(76_000));
        // Below the threshold nothing is owed.
        let r = y.income_tax(
            FilingStatus::MarriedJoint,
            FederalInput {
                ordinary_income: m(150_000),
                long_term_gains: m(50_000),
                ..Default::default()
            },
        );
        assert_eq!(r.niit, Money::ZERO);
        // Interest counts too, via the other-investment field.
        let r = y.income_tax(
            FilingStatus::Single,
            FederalInput {
                ordinary_income: m(250_000),
                other_investment_income: m(10_000),
                ..Default::default()
            },
        );
        assert_eq!(r.niit, Money::from_cents(38_000));
    }

    #[test]
    fn people_65_and_over_get_a_bigger_deduction() {
        let y = FederalTaxYear::y2026();
        let base = FederalInput {
            ordinary_income: m(60_000),
            ..Default::default()
        };
        let plain = y.income_tax(FilingStatus::Single, base);
        let aged = y.income_tax(FilingStatus::Single, FederalInput { seniors: 1, ..base });
        assert_eq!(aged.deduction, plain.deduction + m(2_050));
        let couple = y.income_tax(
            FilingStatus::MarriedJoint,
            FederalInput { seniors: 2, ..base },
        );
        assert_eq!(
            couple.deduction,
            y.standard_deduction(FilingStatus::MarriedJoint) + m(3_300)
        );
        // A second senior on a single return is ignored.
        let odd = y.income_tax(FilingStatus::Single, FederalInput { seniors: 2, ..base });
        assert_eq!(odd.deduction, aged.deduction);
    }

    #[test]
    fn the_temporary_senior_deduction_phases_out() {
        let y = FederalTaxYear::y2026();
        let with = |income: i64, fs| {
            y.income_tax(
                fs,
                FederalInput {
                    ordinary_income: m(income),
                    seniors: 1,
                    senior_bonus: true,
                    ..Default::default()
                },
            )
            .deduction
        };
        let std_aged = y.standard_deduction(FilingStatus::Single) + m(2_050);
        assert_eq!(with(60_000, FilingStatus::Single), std_aged + m(6_000));
        // $100k is $25k over the $75k start: 6% of that is $1,500.
        assert_eq!(with(100_000, FilingStatus::Single), std_aged + m(4_500));
        assert_eq!(with(200_000, FilingStatus::Single), std_aged);
        // Not available when married filing separately.
        assert_eq!(
            with(60_000, FilingStatus::MarriedSeparate),
            y.standard_deduction(FilingStatus::MarriedSeparate) + m(1_650)
        );
    }

    #[test]
    fn each_seniors_6000_phases_out_separately_on_a_joint_return() {
        // The 6% reduction applies to each person's $6,000, so a couple who are both 65+ lose
        // it all at $250,000 of MAGI, not $350,000.
        let y = FederalTaxYear::y2026();
        let fs = FilingStatus::MarriedJoint;
        let ded = |income: i64| {
            y.income_tax(
                fs,
                FederalInput {
                    ordinary_income: m(income),
                    seniors: 2,
                    senior_bonus: true,
                    ..Default::default()
                },
            )
            .deduction
        };
        let std_aged = y.standard_deduction(fs) + m(3_300);
        assert_eq!(ded(150_000), std_aged + m(12_000));
        // $50k over: each $6,000 loses $3,000.
        assert_eq!(ded(200_000), std_aged + m(6_000));
        assert_eq!(ded(250_000), std_aged);
        assert_eq!(ded(300_000), std_aged);
    }
}
