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

/// How yearly spending responds to the portfolio once nobody is earning a wage.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum SpendingRule {
    /// Spend `annual_spending` every year (today's dollars).
    #[default]
    Fixed,
    /// Spend `rate` of the portfolio at the start of each retired year, but at least `floor`.
    PercentOfPortfolio { rate: f64, floor: Money },
    /// Variable percentage withdrawal: spend what an annuity over the years left would pay at an
    /// assumed real `rate`, so spending tracks the portfolio and the horizon.
    Vpw { rate: f64 },
    /// Guyton-Klinger style guardrails: start at `annual_spending`; if the withdrawal rate drifts
    /// more than `band` (e.g. 0.2 = 20%) above the starting rate, cut spending by `step`; if it
    /// drifts that far below, raise spending by `step`.
    Guardrails { band: f64, step: f64 },
}

/// The order accounts are drawn on when income does not cover spending.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WithdrawalOrder {
    /// Cash, taxable, traditional, then Roth.
    #[default]
    Standard,
    /// Cash, traditional, taxable, then Roth (fills low brackets early).
    TraditionalFirst,
    /// Cash, Roth, taxable, then traditional.
    RothFirst,
    /// Cash, then taxable, traditional and Roth in proportion to their balances.
    Blended,
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
    pub spending_rule: SpendingRule,
    pub withdrawal_order: WithdrawalOrder,
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
    let mut spend_now = input.annual_spending;
    let mut start_rate: Option<f64> = None;

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
        let retired = wages == Money::ZERO;
        let portfolio = acc.total();
        if retired {
            match input.spending_rule {
                SpendingRule::Fixed => {}
                SpendingRule::PercentOfPortfolio { rate, floor } => {
                    spend_now = portfolio.mul_rate(rate).max(floor);
                }
                SpendingRule::Vpw { rate } => {
                    // One extra year so the last year never plans to spend the whole portfolio, which
                    // leaves nothing to cover its own tax.
                    let left = (years - i) as f64 + 1.0;
                    let factor = if rate.abs() < 1e-9 {
                        1.0 / left
                    } else {
                        rate / (1.0 - (1.0 + rate).powf(-left))
                    };
                    spend_now = portfolio.mul_rate(factor);
                }
                SpendingRule::Guardrails { band, step } => {
                    if portfolio > Money::ZERO {
                        let start = *start_rate
                            .get_or_insert(spend_now.cents() as f64 / portfolio.cents() as f64);
                        let now = spend_now.cents() as f64 / portfolio.cents() as f64;
                        if now > start * (1.0 + band) {
                            spend_now = spend_now.mul_rate(1.0 - step);
                        } else if now < start * (1.0 - band) {
                            spend_now = spend_now.mul_rate(1.0 + step);
                        }
                    }
                }
            }
        }
        let spending = spend_now
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
                if input.withdrawal_order == WithdrawalOrder::Blended {
                    let (t0, r0, o0) = (a.taxable, a.traditional, a.roth);
                    let pool = t0 + r0 + o0;
                    if pool > Money::ZERO && left > Money::ZERO {
                        let want = left.min(pool);
                        let t = take(
                            &mut a.taxable,
                            &mut Money::from_cents(want.cents() * t0.cents() / pool.cents()),
                            &mut withdrawn,
                        );
                        gains += t.mul_rate(acc.taxable_gain_share);
                        let r = take(
                            &mut a.traditional,
                            &mut Money::from_cents(want.cents() * r0.cents() / pool.cents()),
                            &mut withdrawn,
                        );
                        ord += r;
                        let o = take(
                            &mut a.roth,
                            &mut Money::from_cents(want.cents() * o0.cents() / pool.cents()),
                            &mut withdrawn,
                        );
                        left -= t + r + o;
                        // Rounding dust (and nothing else) comes off the largest pot.
                        if left > Money::ZERO && left <= Money::from_dollars(1) {
                            take(&mut a.taxable, &mut left, &mut withdrawn);
                        }
                    }
                } else {
                    let seq = match input.withdrawal_order {
                        WithdrawalOrder::TraditionalFirst => ['r', 't', 'o'],
                        WithdrawalOrder::RothFirst => ['o', 't', 'r'],
                        _ => ['t', 'r', 'o'],
                    };
                    for k in seq {
                        match k {
                            't' => {
                                let t = take(&mut a.taxable, &mut left, &mut withdrawn);
                                gains += t.mul_rate(acc.taxable_gain_share);
                            }
                            'r' => {
                                let t = take(&mut a.traditional, &mut left, &mut withdrawn);
                                ord += t;
                            }
                            _ => {
                                take(&mut a.roth, &mut left, &mut withdrawn);
                            }
                        }
                    }
                }
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
            spending_rule: SpendingRule::Fixed,
            withdrawal_order: WithdrawalOrder::Standard,
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

    /// A retired household: no wages, so the spending rules apply from the first year.
    fn retired() -> PlanInput {
        let mut i = couple();
        i.streams.clear();
        i.social_security.clear();
        i.people[0] = {
            let mut p = i.people[0].clone();
            p.birth_date = fin_core::NaiveDate::from_ymd_opt(1961, 3, 1).unwrap();
            p
        };
        i.accounts = Accounts {
            cash: Money::ZERO,
            taxable: Money::ZERO,
            taxable_gain_share: 0.0,
            traditional: Money::from_dollars(500_000),
            roth: Money::from_dollars(500_000),
        };
        i.annual_spending = Money::from_dollars(40_000);
        i
    }

    #[test]
    fn fixed_spending_is_the_default_and_never_changes() {
        let p = project_constant(&retired(), 0.03);
        assert!(p
            .rows
            .iter()
            .all(|r| r.spending == Money::from_dollars(40_000)));
    }

    #[test]
    fn percent_of_portfolio_tracks_the_balance_and_never_runs_out() {
        let mut i = retired();
        i.spending_rule = SpendingRule::PercentOfPortfolio {
            rate: 0.04,
            floor: Money::ZERO,
        };
        let p = project_constant(&i, 0.0);
        assert!(p.succeeded());
        assert!(p.final_balance() > Money::ZERO);
        assert!(p.rows[0].spending == Money::from_dollars(40_000));
        assert!(p.rows[10].spending < p.rows[0].spending);
    }

    #[test]
    fn a_floor_keeps_spending_up_even_as_the_portfolio_shrinks() {
        let mut i = retired();
        i.spending_rule = SpendingRule::PercentOfPortfolio {
            rate: 0.04,
            floor: Money::from_dollars(35_000),
        };
        let p = project_constant(&i, 0.0);
        assert!(p
            .rows
            .iter()
            .take_while(|r| r.end_balance > Money::ZERO)
            .all(|r| r.spending >= Money::from_dollars(35_000)));
    }

    #[test]
    fn vpw_spends_the_portfolio_down_by_the_horizon() {
        let mut i = retired();
        i.spending_rule = SpendingRule::Vpw { rate: 0.03 };
        let p = project_constant(&i, 0.03);
        assert!(p.succeeded());
        assert!(
            p.final_balance() < Money::from_dollars(50_000),
            "left {:?}",
            p.final_balance()
        );
        // Spending follows returns: better than assumed means more to spend later on.
        let good = project(&i, &|_| 0.08);
        assert!(good.rows.last().unwrap().spending > good.rows[0].spending);
    }

    #[test]
    fn guardrails_cut_after_poor_returns_and_raise_after_good_ones() {
        let mut i = retired();
        i.spending_rule = SpendingRule::Guardrails {
            band: 0.2,
            step: 0.1,
        };
        let bad = project(&i, &|y| if y < 3 { -0.25 } else { 0.0 });
        assert!(bad.rows.last().unwrap().spending < Money::from_dollars(40_000));
        let good = project(&i, &|_| 0.08);
        assert!(good.rows.last().unwrap().spending > Money::from_dollars(40_000));
        // Steps are 10% at a time, never a collapse in one year.
        for w in bad.rows.windows(2) {
            if w[1].spending > Money::ZERO && w[1].end_balance > Money::ZERO {
                assert!(w[1].spending.cents() >= w[0].spending.cents() * 85 / 100);
            }
        }
    }

    #[test]
    fn guardrails_help_a_plan_that_fixed_spending_cannot_carry() {
        let mut fixed = retired();
        fixed.annual_spending = Money::from_dollars(55_000);
        let mut guard = fixed.clone();
        guard.spending_rule = SpendingRule::Guardrails {
            band: 0.2,
            step: 0.1,
        };
        let f = project_constant(&fixed, 0.0);
        let g = project_constant(&guard, 0.0);
        assert!(!f.succeeded());
        assert!(g.depleted_year.is_none() || g.depleted_year > f.depleted_year);
    }

    #[test]
    fn withdrawal_order_changes_who_pays_tax_first() {
        let mut roth_first = retired();
        roth_first.withdrawal_order = WithdrawalOrder::RothFirst;
        let mut trad_first = retired();
        trad_first.withdrawal_order = WithdrawalOrder::TraditionalFirst;
        let r = project_constant(&roth_first, 0.0);
        let t = project_constant(&trad_first, 0.0);
        // Spending $40k out of a Roth costs no income tax; out of a traditional account it does.
        assert_eq!(r.rows[0].taxes, Money::ZERO);
        assert!(t.rows[0].taxes > Money::ZERO);
    }

    #[test]
    fn blended_draws_from_both_kinds_at_once() {
        let mut b = retired();
        b.annual_spending = Money::from_dollars(60_000);
        b.withdrawal_order = WithdrawalOrder::Blended;
        let p = project_constant(&b, 0.03);
        // Half comes from the Roth, so the tax is lower than all-traditional but above zero.
        let mut t = retired();
        t.annual_spending = Money::from_dollars(60_000);
        t.withdrawal_order = WithdrawalOrder::TraditionalFirst;
        let tp = project_constant(&t, 0.03);
        assert!(p.rows[0].taxes < tp.rows[0].taxes);
        assert!(p.rows[0].withdrawals > Money::ZERO);
    }
}
