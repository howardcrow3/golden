//! Year-by-year retirement projection in today's dollars (real terms): tax brackets and
//! spending are held constant in real terms and portfolios grow at a real return.

use crate::tax::{
    taxable_social_security, FederalInput, FederalTaxYear, FilingStatus, StateTaxYear,
};
use fin_core::{Money, Person};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Accounts {
    pub cash: Money,
    pub taxable: Money,
    /// Share of the taxable balance that is unrealized gain (taxed as long-term gains on sale).
    pub taxable_gain_share: f64,
    pub traditional: Money,
    pub roth: Money,
}

impl Accounts {
    pub fn total(&self) -> Money {
        self.cash + self.taxable + self.traditional + self.roth
    }
}

/// A source of earned income that stops at `end_year` (exclusive of years after).
#[derive(Debug, Clone, PartialEq)]
pub struct IncomeStream {
    pub label: String,
    /// Index into `PlanInput::people`.
    pub person: usize,
    pub annual_gross: Money,
    pub start_year: i32,
    pub end_year: i32,
    /// Pre-tax 401(k)-style contribution per year while earning.
    pub traditional_contribution: Money,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SocialSecurity {
    pub person: usize,
    /// Annual benefit in today's dollars if claimed at `claim_age`.
    pub annual_benefit: Money,
    pub claim_age: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OneTimeCost {
    pub year: i32,
    pub amount: Money,
}

#[derive(Debug, Clone)]
pub struct PlanInput {
    pub start_year: i32,
    pub people: Vec<Person>,
    pub filing: FilingStatus,
    pub state: String,
    pub accounts: Accounts,
    pub streams: Vec<IncomeStream>,
    pub social_security: Vec<SocialSecurity>,
    /// Household spending in today's dollars.
    pub annual_spending: Money,
    /// One-off outflows on top of spending, in today's dollars: a house, tuition, a wedding.
    /// A negative amount is money coming in that isn't taxed as income (an inheritance, a
    /// home sale within the exclusion).
    pub one_time_costs: Vec<OneTimeCost>,
    /// Plan until the primary person reaches this age.
    pub horizon_age: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct YearRow {
    pub year: i32,
    pub age: i32,
    pub earned: Money,
    pub social_security: Money,
    pub spending: Money,
    pub taxes: Money,
    pub withdrawals: Money,
    pub end_balance: Money,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Projection {
    pub rows: Vec<YearRow>,
    /// First year the household could not cover spending, if ever.
    pub depleted_year: Option<i32>,
}

impl Projection {
    pub fn succeeded(&self) -> bool {
        self.depleted_year.is_none()
    }
    pub fn final_balance(&self) -> Money {
        self.rows.last().map_or(Money::ZERO, |r| r.end_balance)
    }
}

/// Run the projection with `real_return(i)` giving the real return for the i-th year.
pub fn project(input: &PlanInput, real_return: &dyn Fn(usize) -> f64) -> Projection {
    let fed = FederalTaxYear::y2026();
    let states = StateTaxYear::y2026();
    let primary = &input.people[0];
    let years =
        (input.horizon_age - primary.age_on(chrono_start(input.start_year))).max(0) as usize + 1;

    let mut acc = input.accounts;
    let mut rows = Vec::with_capacity(years);
    let mut depleted = None;

    for i in 0..years {
        let year = input.start_year + i as i32;
        let r = real_return(i);

        let mut wages = Money::ZERO;
        let mut contrib = Money::ZERO;
        for s in &input.streams {
            if year >= s.start_year && year <= s.end_year {
                wages += s.annual_gross;
                contrib += s.traditional_contribution;
            }
        }
        let mut ss = Money::ZERO;
        for b in &input.social_security {
            let age = input.people[b.person].age_on(chrono_start(year));
            if age >= b.claim_age {
                ss += b.annual_benefit;
            }
        }

        // Tax on income that does not depend on withdrawals.
        let fica = fed.fica_for_wages(input.filing, wages);
        let base_ordinary = (wages - contrib).max(Money::ZERO);
        let seniors = input
            .people
            .iter()
            .filter(|p| !p.is_dependent && p.age_on(chrono_start(year)) >= 65)
            .count()
            .min(2) as u8;
        // How much of Social Security is taxed depends on all other income, so it is worked
        // out again whenever withdrawals change that income.
        let base_tax = |ord: Money, gains: Money| -> Money {
            let taxable_ss = taxable_social_security(input.filing, ss, ord + gains);
            let f = fed.income_tax(
                input.filing,
                FederalInput {
                    ordinary_income: ord + taxable_ss,
                    long_term_gains: gains,
                    seniors,
                    senior_bonus: year <= 2028,
                    ..Default::default()
                },
            );
            let st = states
                .income_tax(&input.state, input.filing, f.agi)
                .unwrap_or(Money::ZERO);
            f.income_tax + st
        };

        // Money needed after tax; withdrawals are grossed up by iterating (tax depends on withdrawals).
        let spending = input.annual_spending
            + input
                .one_time_costs
                .iter()
                .filter(|c| c.year == year)
                .map(|c| c.amount)
                .sum::<Money>();
        let income_tax_base = base_tax(base_ordinary, Money::ZERO);
        let mut cash_in = wages - contrib + ss - fica - income_tax_base;
        let mut taxes = fica + income_tax_base;
        let mut withdrawn = Money::ZERO;
        let mut gap = spending - cash_in;

        if gap > Money::ZERO {
            // Draw cash, then taxable, then traditional, then Roth, grossing up for tax.
            let mut need = gap;
            let mut a = acc;
            for _ in 0..12 {
                a = acc;
                let mut ord = base_ordinary;
                let mut gains = Money::ZERO;
                withdrawn = Money::ZERO;
                let mut left = need;
                let take = |from: &mut Money, left: &mut Money, w: &mut Money| {
                    let t = (*left).min(*from).max(Money::ZERO);
                    *from -= t;
                    *left -= t;
                    *w += t;
                    t
                };
                take(&mut a.cash, &mut left, &mut withdrawn);
                let t = take(&mut a.taxable, &mut left, &mut withdrawn);
                gains += t.mul_rate(acc.taxable_gain_share);
                let t = take(&mut a.traditional, &mut left, &mut withdrawn);
                ord += t;
                take(&mut a.roth, &mut left, &mut withdrawn);
                let new_tax = base_tax(ord, gains);
                let extra_tax = new_tax - income_tax_base;
                let target = gap + extra_tax;
                if (target - need).abs() <= Money::from_cents(100) {
                    taxes = fica + new_tax;
                    break;
                }
                need = target;
                taxes = fica + new_tax;
            }
            acc = a;
            cash_in += withdrawn;
            gap = spending + (taxes - fica - income_tax_base) - cash_in;
            if gap > Money::from_dollars(1) && depleted.is_none() {
                depleted = Some(year);
            }
        } else {
            // Surplus is saved: pre-tax contributions plus leftover cash to the taxable account.
            acc.traditional += contrib;
            acc.taxable += -gap;
        }
        if contrib > Money::ZERO && gap > Money::ZERO {
            acc.traditional += contrib;
        }

        // Grow what is left.
        acc.cash = acc.cash.mul_rate(1.0);
        acc.taxable = acc.taxable.mul_rate(1.0 + r);
        acc.traditional = acc.traditional.mul_rate(1.0 + r);
        acc.roth = acc.roth.mul_rate(1.0 + r);

        rows.push(YearRow {
            year,
            age: primary.age_on(chrono_start(year)),
            earned: wages,
            social_security: ss,
            spending,
            taxes,
            withdrawals: withdrawn,
            end_balance: acc.total(),
        });
        if depleted.is_some() {
            // Stop: the plan failed; remaining years would all be shortfalls.
            for j in (i + 1)..years {
                let last = input.start_year + j as i32;
                rows.push(YearRow {
                    year: last,
                    age: primary.age_on(chrono_start(last)),
                    earned: Money::ZERO,
                    social_security: Money::ZERO,
                    spending,
                    taxes: Money::ZERO,
                    withdrawals: Money::ZERO,
                    end_balance: Money::ZERO,
                });
            }
            break;
        }
    }
    Projection {
        rows,
        depleted_year: depleted,
    }
}

/// December 31 of `year`: ages in a year row are the age reached by year end.
fn chrono_start(year: i32) -> fin_core::NaiveDate {
    fin_core::NaiveDate::from_ymd_opt(year, 12, 31).unwrap()
}

/// Deterministic projection at a constant real return.
pub fn project_constant(input: &PlanInput, real_return: f64) -> Projection {
    project(input, &|_| real_return)
}

#[cfg(test)]
pub(crate) mod fixtures {
    use super::*;
    use fin_core::NaiveDate;

    pub fn couple() -> PlanInput {
        let p = |n: &str, y: i32| Person {
            id: n.into(),
            name: n.into(),
            birth_date: NaiveDate::from_ymd_opt(y, 6, 15).unwrap(),
            is_dependent: false,
        };
        PlanInput {
            start_year: 2026,
            people: vec![p("A", 1976), p("B", 1978)],
            filing: FilingStatus::MarriedJoint,
            state: "TX".into(),
            accounts: Accounts {
                cash: Money::from_dollars(30_000),
                taxable: Money::from_dollars(150_000),
                taxable_gain_share: 0.3,
                traditional: Money::from_dollars(600_000),
                roth: Money::from_dollars(120_000),
            },
            streams: vec![IncomeStream {
                label: "A salary".into(),
                person: 0,
                annual_gross: Money::from_dollars(180_000),
                start_year: 2026,
                end_year: 2041,
                traditional_contribution: Money::from_dollars(23_500),
            }],
            social_security: vec![SocialSecurity {
                person: 0,
                annual_benefit: Money::from_dollars(38_000),
                claim_age: 67,
            }],
            annual_spending: Money::from_dollars(110_000),
            one_time_costs: vec![],
            horizon_age: 95,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fixtures::couple;
    use super::*;

    #[test]
    fn rows_cover_start_to_horizon() {
        let p = project_constant(&couple(), 0.04);
        assert_eq!(p.rows.first().unwrap().year, 2026);
        assert_eq!(p.rows.last().unwrap().age, 95);
        assert_eq!(p.rows.len(), 95 - 49);
    }

    #[test]
    fn a_comfortable_household_succeeds() {
        let p = project_constant(&couple(), 0.04);
        assert!(p.succeeded(), "depleted in {:?}", p.depleted_year);
        assert!(p.final_balance() > Money::ZERO);
    }

    #[test]
    fn high_spending_and_low_return_fails() {
        let mut i = couple();
        i.annual_spending = Money::from_dollars(260_000);
        let p = project_constant(&i, 0.01);
        assert!(!p.succeeded());
        let year = p.depleted_year.unwrap();
        assert!(year > 2026);
        assert_eq!(p.final_balance(), Money::ZERO);
    }

    #[test]
    fn working_years_save_retired_years_withdraw() {
        let p = project_constant(&couple(), 0.04);
        let working = &p.rows[0];
        assert!(working.earned > Money::ZERO);
        assert_eq!(working.withdrawals, Money::ZERO);
        let retired = p.rows.iter().find(|r| r.year == 2050).unwrap();
        assert_eq!(retired.earned, Money::ZERO);
        assert!(retired.withdrawals > Money::ZERO || retired.social_security >= retired.spending);
    }

    #[test]
    fn social_security_starts_at_claim_age() {
        let p = project_constant(&couple(), 0.04);
        let before = p.rows.iter().find(|r| r.age == 66).unwrap();
        let after = p.rows.iter().find(|r| r.age == 67).unwrap();
        assert_eq!(before.social_security, Money::ZERO);
        assert_eq!(after.social_security, Money::from_dollars(38_000));
    }

    #[test]
    fn a_retiree_living_on_social_security_and_roth_pays_no_income_tax() {
        let mut i = couple();
        i.streams.clear();
        i.accounts = Accounts {
            cash: Money::ZERO,
            taxable: Money::ZERO,
            taxable_gain_share: 0.0,
            traditional: Money::ZERO,
            roth: Money::from_dollars(800_000),
        };
        i.social_security = vec![SocialSecurity {
            person: 0,
            annual_benefit: Money::from_dollars(30_000),
            claim_age: 50,
        }];
        i.annual_spending = Money::from_dollars(50_000);
        let p = project_constant(&i, 0.04);
        // $30k of benefits and nothing else is under the joint threshold: nothing is taxed,
        // so the only gap is spending less benefits, paid from Roth.
        let row = &p.rows[0];
        assert_eq!(row.taxes, Money::ZERO);
        assert_eq!(row.withdrawals, Money::from_dollars(20_000));
    }

    #[test]
    fn one_time_costs_land_in_their_year_and_inflows_are_untaxed() {
        let mut i = couple();
        i.one_time_costs = vec![OneTimeCost {
            year: 2030,
            amount: Money::from_dollars(200_000),
        }];
        let base = project_constant(&couple(), 0.04);
        let with = project_constant(&i, 0.04);
        let row = |p: &Projection, y| p.rows.iter().find(|r| r.year == y).unwrap().spending;
        assert_eq!(
            row(&with, 2030),
            row(&base, 2030) + Money::from_dollars(200_000)
        );
        assert_eq!(row(&with, 2031), row(&base, 2031));
        assert!(with.final_balance() < base.final_balance());
        i.one_time_costs[0].amount = Money::from_dollars(-200_000);
        let inflow = project_constant(&i, 0.04);
        assert!(inflow.final_balance() > base.final_balance());
        // Nothing is taxed on the way in.
        let t = |p: &Projection, y| p.rows.iter().find(|r| r.year == y).unwrap().taxes;
        assert_eq!(t(&inflow, 2030), t(&base, 2030));
    }

    #[test]
    fn higher_returns_never_hurt() {
        let lo = project_constant(&couple(), 0.02).final_balance();
        let hi = project_constant(&couple(), 0.05).final_balance();
        assert!(hi >= lo);
    }

    #[test]
    fn more_spending_never_helps() {
        let mut i = couple();
        let base = project_constant(&i, 0.03).final_balance();
        i.annual_spending = Money::from_dollars(130_000);
        assert!(project_constant(&i, 0.03).final_balance() <= base);
    }

    #[test]
    fn taxes_are_charged_in_a_high_tax_state() {
        let mut i = couple();
        let tx = project_constant(&i, 0.04).rows[0].taxes;
        i.state = "CA".into();
        let ca = project_constant(&i, 0.04).rows[0].taxes;
        assert!(ca > tx);
    }
}
