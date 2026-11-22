//! Rules that sit beside the regular income tax: the SALT deduction cap, the alternative
//! minimum tax and the 10% penalty on early retirement withdrawals.

use super::*;
use crate::tax::FilingStatus;

/// OBBBA state-and-local-tax deduction cap for 2026: $40,400 ($20,200 if married filing
/// separately), reduced by 30% of modified AGI over $505,000 ($252,500 separately), never below
/// $10,000 ($5,000 separately). Source: IRC section 164(b)(7) as amended by Public Law 119-21
/// with the 2026 figures from IRS guidance; check each November.
pub fn salt_deduction(fs: FilingStatus, magi: Money, state_local_taxes: Money) -> Money {
    let half = fs == FilingStatus::MarriedSeparate;
    let (cap, start, floor) = if half {
        (20_200, 252_500, 5_000)
    } else {
        (40_400, 505_000, 10_000)
    };
    let cut = (magi - Money::from_dollars(start))
        .max(Money::ZERO)
        .mul_rate(0.30);
    let limit = (Money::from_dollars(cap) - cut).max(Money::from_dollars(floor));
    state_local_taxes.max(Money::ZERO).min(limit)
}

/// Tax on a withdrawal from a pre-tax retirement account before age 59 and a half: 10% of the
/// taxable amount, unless an exception applies (the caller decides that and passes `exempt`).
/// Source: IRC section 72(t).
pub fn early_withdrawal_penalty(
    age_at_withdrawal: f64,
    taxable_withdrawal: Money,
    exempt: bool,
) -> Money {
    if exempt || age_at_withdrawal >= 59.5 {
        Money::ZERO
    } else {
        taxable_withdrawal.max(Money::ZERO).mul_rate(0.10)
    }
}

/// 2026 AMT figures: (exemption, phase-out start, 28% starts at taxable AMT income).
/// Source: IRS Rev. Proc. 2025-32 and the 50% phase-out rate set by Public Law 119-21.
fn amt_figures(fs: FilingStatus) -> (i64, i64, i64) {
    match fs {
        FilingStatus::MarriedJoint => (140_200, 1_000_000, 244_500),
        FilingStatus::MarriedSeparate => (70_100, 500_000, 122_250),
        FilingStatus::Single | FilingStatus::HeadOfHousehold => (90_100, 500_000, 244_500),
    }
}

impl FederalTaxYear {
    /// Alternative minimum tax owed on top of the regular tax. The standard deduction is not
    /// allowed under AMT, and neither is the SALT part of itemized deductions
    /// (`salt_in_itemized`). `preferences` are other add-backs, such as the spread on exercised
    /// incentive stock options. Gains keep their lower rates.
    pub fn amt(
        &self,
        fs: FilingStatus,
        i: &FederalInput,
        r: &FederalResult,
        preferences: Money,
    ) -> Money {
        let standard_used = i.itemized <= self.standard_deduction(fs);
        let addback = if standard_used {
            r.deduction
        } else {
            i.salt_in_itemized.min(r.deduction)
        };
        let amti = r.taxable_income + addback + preferences;
        let (exemption, phase_start, break_28) = amt_figures(fs);
        let lost = (amti - Money::from_dollars(phase_start))
            .max(Money::ZERO)
            .mul_rate(0.50);
        let exemption = (Money::from_dollars(exemption) - lost).max(Money::ZERO);
        let base = (amti - exemption).max(Money::ZERO);
        let gains = i.long_term_gains.min(base);
        let ordinary = base - gains;
        let cut = Money::from_dollars(break_28);
        let low = ordinary.min(cut);
        let high = (ordinary - cut).max(Money::ZERO);
        let tentative =
            low.mul_rate(0.26) + high.mul_rate(0.28) + self.ltcg[&fs].tax_stacked(ordinary, gains);
        (tentative - (r.ordinary_tax + r.ltcg_tax)).max(Money::ZERO)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn salt_cap_is_40_400_and_phases_down_to_10_000() {
        let s = Money::from_dollars;
        assert_eq!(
            salt_deduction(FilingStatus::MarriedJoint, s(200_000), s(60_000)),
            s(40_400)
        );
        assert_eq!(
            salt_deduction(FilingStatus::MarriedJoint, s(200_000), s(12_000)),
            s(12_000)
        );
        // $100k over the start: cap loses 30k.
        assert_eq!(
            salt_deduction(FilingStatus::Single, s(605_000), s(60_000)),
            s(10_400)
        );
        assert_eq!(
            salt_deduction(FilingStatus::Single, s(2_000_000), s(60_000)),
            s(10_000)
        );
        assert_eq!(
            salt_deduction(FilingStatus::MarriedSeparate, s(100_000), s(60_000)),
            s(20_200)
        );
        assert_eq!(
            salt_deduction(FilingStatus::MarriedSeparate, s(2_000_000), s(60_000)),
            s(5_000)
        );
    }

    #[test]
    fn early_withdrawals_cost_ten_percent_unless_exempt() {
        let m = Money::from_dollars;
        assert_eq!(early_withdrawal_penalty(50.0, m(20_000), false), m(2_000));
        assert_eq!(early_withdrawal_penalty(59.4, m(20_000), false), m(2_000));
        assert_eq!(
            early_withdrawal_penalty(59.5, m(20_000), false),
            Money::ZERO
        );
        assert_eq!(early_withdrawal_penalty(40.0, m(20_000), true), Money::ZERO);
        assert_eq!(early_withdrawal_penalty(40.0, m(-5), false), Money::ZERO);
    }

    fn run(fs: FilingStatus, input: FederalInput) -> (FederalResult, Money) {
        let y = FederalTaxYear::y2026();
        let r = y.income_tax(fs, input);
        let a = y.amt(fs, &input, &r, Money::ZERO);
        (r, a)
    }

    #[test]
    fn ordinary_earners_with_the_standard_deduction_owe_no_amt() {
        for wages in [60_000, 150_000, 400_000] {
            let (_, a) = run(
                FilingStatus::Single,
                FederalInput {
                    ordinary_income: Money::from_dollars(wages),
                    ..Default::default()
                },
            );
            assert_eq!(a, Money::ZERO, "wages {wages}");
        }
    }

    #[test]
    fn heavy_itemizing_of_state_tax_can_trigger_amt() {
        let y = FederalTaxYear::y2026();
        let fs = FilingStatus::MarriedJoint;
        let input = FederalInput {
            ordinary_income: Money::from_dollars(450_000),
            itemized: Money::from_dollars(120_000),
            salt_in_itemized: Money::from_dollars(40_400),
            ..Default::default()
        };
        let r = y.income_tax(fs, input);
        assert_eq!(y.amt(fs, &input, &r, Money::ZERO), Money::ZERO);
        // A large incentive-stock-option spread tips it over.
        assert!(y.amt(fs, &input, &r, Money::from_dollars(600_000)) > Money::ZERO);
    }

    #[test]
    fn the_exemption_phases_out_above_the_start() {
        let y = FederalTaxYear::y2026();
        let fs = FilingStatus::Single;
        let input = FederalInput {
            ordinary_income: Money::from_dollars(100_000),
            ..Default::default()
        };
        let r = y.income_tax(fs, input);
        let low = y.amt(fs, &input, &r, Money::from_dollars(0));
        let big = y.amt(fs, &input, &r, Money::from_dollars(900_000));
        assert!(big > low);
    }
}

/// Self-employment tax on one year of net business income.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SelfEmploymentTax {
    pub tax: Money,
    /// The half of the tax that is deducted from income on the return.
    pub deduction: Money,
}

impl FederalTaxYear {
    /// Self-employment tax: 15.3% of 92.35% of net earnings, with the Social Security part
    /// stopping at the wage base (shared with any wages) and the 0.9% additional Medicare tax
    /// over the filing-status threshold. Source: IRC section 1401 and 3101(b)(2).
    pub fn self_employment_tax(
        &self,
        fs: FilingStatus,
        net_earnings: Money,
        other_wages: Money,
    ) -> SelfEmploymentTax {
        let f = &self.fica;
        let base = net_earnings.max(Money::ZERO).mul_rate(0.9235);
        if base <= Money::ZERO {
            return SelfEmploymentTax {
                tax: Money::ZERO,
                deduction: Money::ZERO,
            };
        }
        let room = (f.ss_wage_base - other_wages.max(Money::ZERO)).max(Money::ZERO);
        let ss = base.min(room).mul_rate(f.ss_rate * 2.0);
        let medicare = base.mul_rate(f.medicare_rate * 2.0);
        let over = (base + other_wages.max(Money::ZERO) - f.addl_medicare_threshold[&fs])
            .max(Money::ZERO)
            .min(base);
        let addl = over.mul_rate(f.addl_medicare_rate);
        let regular = ss + medicare;
        SelfEmploymentTax {
            tax: regular + addl,
            deduction: regular.mul_rate(0.5),
        }
    }
}

/// Qualified business income deduction, simplified: 20% of business income after the
/// self-employment tax deduction, at most 20% of taxable income before it, and only while
/// taxable income is under the 2026 threshold ($201,750 single, $403,500 joint) where the
/// wage and specified-service limits do not apply. Above it we allow nothing, which is
/// cautious. Source: IRC section 199A.
pub fn qbi_deduction(
    fs: FilingStatus,
    qualified_income: Money,
    taxable_before_qbi: Money,
) -> Money {
    let threshold = match fs {
        FilingStatus::MarriedJoint => 403_500,
        _ => 201_750,
    };
    if taxable_before_qbi > Money::from_dollars(threshold) {
        return Money::ZERO;
    }
    qualified_income
        .max(Money::ZERO)
        .mul_rate(0.20)
        .min(taxable_before_qbi.max(Money::ZERO).mul_rate(0.20))
}

/// What one person can put into a Solo 401(k) in 2026: the employee deferral ($24,500, plus
/// $8,000 from age 50) and an employer share of 20% of net earnings after half the self-employment
/// tax, all capped at $72,000 (plus catch-up). Source: IRC section 415(c) and 402(g).
pub fn solo_401k_limit(age: i32, net_earnings: Money, se_deduction: Money) -> Money {
    let catch_up = if age >= 50 { 8_000 } else { 0 };
    let employee = Money::from_dollars(24_500 + catch_up);
    let earnings = (net_earnings - se_deduction).max(Money::ZERO);
    let employer = earnings.mul_rate(0.20);
    (employee + employer)
        .min(Money::from_dollars(72_000 + catch_up))
        .min(earnings)
}

/// SEP IRA: 20% of net earnings after half the self-employment tax, capped at $72,000.
pub fn sep_ira_limit(net_earnings: Money, se_deduction: Money) -> Money {
    (net_earnings - se_deduction)
        .max(Money::ZERO)
        .mul_rate(0.20)
        .min(Money::from_dollars(72_000))
}

#[cfg(test)]
mod selfemp_tests {
    use super::*;
    use crate::tax::federal::FederalTaxYear;

    fn fed() -> FederalTaxYear {
        FederalTaxYear::y2026()
    }

    #[test]
    fn se_tax_on_100k_is_about_14k() {
        let t = fed().self_employment_tax(
            FilingStatus::Single,
            Money::from_dollars(100_000),
            Money::ZERO,
        );
        // 100,000 x 0.9235 x 15.3% = 14,130
        let d = t.tax.cents() - 1_413_000;
        assert!(d.abs() < 10_000, "{:?}", t);
        assert!((t.deduction.cents() - t.tax.cents() / 2).abs() <= 1);
    }

    #[test]
    fn wages_use_up_the_social_security_base() {
        let alone = fed().self_employment_tax(
            FilingStatus::Single,
            Money::from_dollars(50_000),
            Money::ZERO,
        );
        let with_wages = fed().self_employment_tax(
            FilingStatus::Single,
            Money::from_dollars(50_000),
            Money::from_dollars(300_000),
        );
        assert!(with_wages.tax < alone.tax);
        // Only Medicare (2.9%) and the additional 0.9% remain.
        let expect = Money::from_dollars(50_000).mul_rate(0.9235 * (0.029 + 0.009));
        assert!((with_wages.tax.cents() - expect.cents()).abs() < 100);
    }

    #[test]
    fn no_income_no_tax_and_losses_do_not_refund() {
        let t = fed().self_employment_tax(
            FilingStatus::Single,
            Money::from_dollars(-5_000),
            Money::ZERO,
        );
        assert_eq!(t.tax, Money::ZERO);
    }

    #[test]
    fn qbi_is_twenty_percent_with_a_taxable_income_cap() {
        let q = qbi_deduction(
            FilingStatus::Single,
            Money::from_dollars(90_000),
            Money::from_dollars(120_000),
        );
        assert_eq!(q, Money::from_dollars(18_000));
        let capped = qbi_deduction(
            FilingStatus::Single,
            Money::from_dollars(90_000),
            Money::from_dollars(50_000),
        );
        assert_eq!(capped, Money::from_dollars(10_000));
    }

    #[test]
    fn qbi_is_withheld_above_the_threshold() {
        assert_eq!(
            qbi_deduction(
                FilingStatus::Single,
                Money::from_dollars(90_000),
                Money::from_dollars(250_000)
            ),
            Money::ZERO
        );
        assert!(
            qbi_deduction(
                FilingStatus::MarriedJoint,
                Money::from_dollars(90_000),
                Money::from_dollars(250_000)
            ) > Money::ZERO
        );
    }

    #[test]
    fn solo_401k_and_sep_limits() {
        let net = Money::from_dollars(100_000);
        let half = Money::from_dollars(7_065);
        let solo = solo_401k_limit(40, net, half);
        assert_eq!(
            solo,
            Money::from_dollars(24_500) + Money::from_dollars(92_935).mul_rate(0.20)
        );
        assert!(solo_401k_limit(55, net, half) > solo);
        assert_eq!(
            sep_ira_limit(net, half),
            Money::from_dollars(92_935).mul_rate(0.20)
        );
        // Tiny earnings: never more than was earned.
        assert_eq!(
            solo_401k_limit(40, Money::from_dollars(10_000), Money::ZERO),
            Money::from_dollars(10_000)
        );
        // Huge earnings hit the cap.
        assert_eq!(
            solo_401k_limit(40, Money::from_dollars(900_000), Money::ZERO),
            Money::from_dollars(72_000)
        );
    }
}
