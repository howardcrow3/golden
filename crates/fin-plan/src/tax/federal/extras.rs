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
