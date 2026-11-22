//! Children in the plan: childcare while they are small, college at 18 to 21, and the child tax
//! credit until 17. Turned into yearly extra costs (credits are untaxed inflows, negative costs)
//! in today's dollars. Spending is assumed to already cover everyday child costs, so enter only
//! what is on top of it.
use crate::projection::OneTimeCost;
use crate::tax::FilingStatus;
use fin_core::Money;

/// 2026 child tax credit per qualifying child under 17 (IRC section 24 as amended by Public
/// Law 119-21), phased down $50 for every $1,000 of income over $200,000 ($400,000 joint).
pub const CHILD_TAX_CREDIT: i64 = 2_200;
pub const CREDIT_UNDER_AGE: i32 = 17;
pub const CHILDCARE_THROUGH_AGE: i32 = 12;
pub const COLLEGE_FIRST_AGE: i32 = 18;
pub const COLLEGE_YEARS: i32 = 4;

#[derive(Debug, Clone, PartialEq)]
pub struct Child {
    pub birth_year: i32,
    /// Yearly childcare cost from birth through age 12.
    pub childcare: Money,
    /// Yearly college cost you will pay, for four years from age 18.
    pub college: Money,
}

/// The credit for `children` under 17, at `income` (modified AGI).
pub fn child_tax_credit(fs: FilingStatus, income: Money, children: usize) -> Money {
    let start = if fs == FilingStatus::MarriedJoint {
        400_000
    } else {
        200_000
    };
    let over = (income - Money::from_dollars(start))
        .max(Money::ZERO)
        .cents();
    // $50 per $1,000 or part of $1,000 over the threshold.
    let steps = (over + 99_999) / 100_000;
    let full = CHILD_TAX_CREDIT * 100 * children as i64;
    Money::from_cents((full - steps * 5_000).max(0))
}

/// Net extra cost per year, `year` to `last_year` inclusive. `income_in(year)` is the
/// household's income that year, for the credit's phase-out.
pub fn yearly_costs(
    children: &[Child],
    fs: FilingStatus,
    year: i32,
    last_year: i32,
    income_in: impl Fn(i32) -> Money,
) -> Vec<OneTimeCost> {
    let mut out = Vec::new();
    for y in year..=last_year {
        let mut cents = 0i64;
        let mut credited = 0usize;
        for c in children {
            let age = y - c.birth_year;
            if (0..=CHILDCARE_THROUGH_AGE).contains(&age) {
                cents += c.childcare.cents();
            }
            if (COLLEGE_FIRST_AGE..COLLEGE_FIRST_AGE + COLLEGE_YEARS).contains(&age) {
                cents += c.college.cents();
            }
            if (0..CREDIT_UNDER_AGE).contains(&age) {
                credited += 1;
            }
        }
        if credited > 0 {
            cents -= child_tax_credit(fs, income_in(y), credited).cents();
        }
        if cents != 0 {
            out.push(OneTimeCost {
                year: y,
                amount: Money::from_cents(cents),
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(n: i64) -> Money {
        Money::from_dollars(n)
    }
    fn child(birth: i32, care: i64, college: i64) -> Child {
        Child {
            birth_year: birth,
            childcare: d(care),
            college: d(college),
        }
    }
    fn at(costs: &[OneTimeCost], year: i32) -> i64 {
        costs
            .iter()
            .find(|c| c.year == year)
            .map_or(0, |c| c.amount.cents())
    }

    #[test]
    fn full_credit_below_the_threshold_and_phased_out_above() {
        let fs = FilingStatus::MarriedJoint;
        assert_eq!(child_tax_credit(fs, d(150_000), 2), d(4_400));
        assert_eq!(child_tax_credit(fs, d(400_000), 1), d(2_200));
        // $10,000 over: $500 off.
        assert_eq!(child_tax_credit(fs, d(410_000), 1), d(1_700));
        // A part of $1,000 counts as a full step.
        assert_eq!(child_tax_credit(fs, d(400_001), 1), d(2_150));
        assert_eq!(
            child_tax_credit(FilingStatus::Single, d(250_000), 1),
            Money::ZERO
        );
        assert_eq!(child_tax_credit(fs, d(100_000), 0), Money::ZERO);
    }

    #[test]
    fn childcare_runs_from_birth_through_twelve() {
        let costs = yearly_costs(
            &[child(2026, 10_000, 0)],
            FilingStatus::Single,
            2026,
            2060,
            |_| d(80_000),
        );
        // Year of birth: childcare 10,000 less the 2,200 credit.
        assert_eq!(at(&costs, 2026), d(7_800).cents());
        assert_eq!(at(&costs, 2038), d(7_800).cents());
        // Age 13 to 16: only the credit.
        assert_eq!(at(&costs, 2039), -d(2_200).cents());
        assert_eq!(at(&costs, 2042), -d(2_200).cents());
        // Age 17: nothing.
        assert_eq!(at(&costs, 2043), 0);
    }

    #[test]
    fn college_is_four_years_from_eighteen() {
        let costs = yearly_costs(
            &[child(2020, 0, 30_000)],
            FilingStatus::Single,
            2026,
            2060,
            |_| d(80_000),
        );
        assert_eq!(at(&costs, 2037), 0);
        for y in 2038..=2041 {
            assert_eq!(at(&costs, y), d(30_000).cents(), "{y}");
        }
        assert_eq!(at(&costs, 2042), 0);
    }

    #[test]
    fn several_children_add_up_and_no_children_is_empty() {
        let two = [child(2024, 5_000, 0), child(2026, 5_000, 0)];
        let costs = yearly_costs(&two, FilingStatus::MarriedJoint, 2026, 2026, |_| d(100_000));
        assert_eq!(at(&costs, 2026), d(10_000 - 4_400).cents());
        assert!(yearly_costs(&[], FilingStatus::Single, 2026, 2060, |_| d(1)).is_empty());
    }

    #[test]
    fn income_over_the_threshold_in_a_later_year_cuts_that_years_credit() {
        let costs = yearly_costs(
            &[child(2026, 0, 0)],
            FilingStatus::Single,
            2026,
            2027,
            |y| {
                if y == 2026 {
                    d(90_000)
                } else {
                    d(210_000)
                }
            },
        );
        assert_eq!(at(&costs, 2026), -d(2_200).cents());
        assert_eq!(at(&costs, 2027), -d(1_700).cents());
    }
}
