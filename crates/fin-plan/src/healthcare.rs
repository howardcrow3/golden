//! Healthcare costs the regular spending number tends to leave out: insurance and bills before
//! Medicare, out-of-pocket costs after it, and a long-term care spell late in life.
//!
//! Everything is in today's dollars like the rest of the plan; healthcare is assumed to rise
//! faster than prices, so each year's amount is scaled up by `excess_inflation` a year.
//! Medicare premiums and the surcharge are worked out separately in `medicare`.
use crate::projection::OneTimeCost;
use fin_core::{Money, Person};

#[derive(Debug, Clone, PartialEq)]
pub struct HealthCosts {
    /// Per person per year before 65: insurance premiums and out-of-pocket bills.
    pub pre_65: Money,
    /// Per person per year from 65: Medigap or Advantage, drugs, dental, vision, hearing.
    pub out_of_pocket_65_plus: Money,
    /// Extra yearly growth above inflation, such as 0.015.
    pub excess_inflation: f64,
    pub ltc: Option<LongTermCare>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LongTermCare {
    /// Index into the people list: who needs care.
    pub person: usize,
    pub start_age: i32,
    pub years: i32,
    /// Cost of care per year in today's dollars.
    pub annual_cost: Money,
    /// Chance care is needed, 0 to 1. The plan carries the expected cost, cost times chance.
    pub chance: f64,
}

impl HealthCosts {
    /// 2026 planning estimates. They are starting points, not quotes: premiums and care costs
    /// vary a lot by place and plan.
    pub fn defaults() -> Self {
        Self {
            pre_65: Money::from_dollars(9_600),
            out_of_pocket_65_plus: Money::from_dollars(5_500),
            excess_inflation: 0.015,
            ltc: Some(LongTermCare {
                person: 0,
                start_age: 82,
                years: 3,
                annual_cost: Money::from_dollars(110_000),
                chance: 0.5,
            }),
        }
    }
}

/// Yearly costs from `start_year` to the year the first person reaches `horizon_age`, as extra
/// spending the projection adds to each year. A person's costs stop when they pass `horizon_age`.
pub fn yearly_costs(
    h: &HealthCosts,
    people: &[Person],
    start_year: i32,
    horizon_age: i32,
) -> Vec<OneTimeCost> {
    let Some(first) = people.first() else {
        return vec![];
    };
    let birth_year = |p: &Person| {
        p.birth_date
            .format("%Y")
            .to_string()
            .parse::<i32>()
            .unwrap_or(0)
    };
    let last_year = birth_year(first) + horizon_age;
    let mut out = Vec::new();
    for year in start_year..=last_year {
        let grow = (1.0 + h.excess_inflation).powi(year - start_year);
        let mut total = Money::ZERO;
        for (i, p) in people.iter().enumerate() {
            if p.is_dependent {
                continue;
            }
            let age = year - birth_year(p);
            if age > horizon_age {
                continue;
            }
            total += if age < 65 {
                h.pre_65
            } else {
                h.out_of_pocket_65_plus
            };
            if let Some(l) = &h.ltc {
                if l.person == i && age >= l.start_age && age < l.start_age + l.years {
                    total += l.annual_cost.mul_rate(l.chance.clamp(0.0, 1.0));
                }
            }
        }
        if total > Money::ZERO {
            out.push(OneTimeCost {
                year,
                amount: total.mul_rate(grow),
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use fin_core::NaiveDate;

    fn person(year: i32) -> Person {
        Person {
            id: "a".into(),
            name: "A".into(),
            birth_date: NaiveDate::from_ymd_opt(year, 6, 1).unwrap(),
            is_dependent: false,
        }
    }

    fn at(v: &[OneTimeCost], y: i32) -> Money {
        v.iter()
            .find(|c| c.year == y)
            .map_or(Money::ZERO, |c| c.amount)
    }

    #[test]
    fn premiums_before_65_and_out_of_pocket_after() {
        let mut h = HealthCosts::defaults();
        h.excess_inflation = 0.0;
        h.ltc = None;
        let v = yearly_costs(&h, &[person(1976)], 2026, 95);
        assert_eq!(at(&v, 2026), Money::from_dollars(9_600));
        assert_eq!(at(&v, 2041), Money::from_dollars(5_500)); // age 65
        assert_eq!(v.last().unwrap().year, 2071);
    }

    #[test]
    fn couple_pays_for_both() {
        let mut h = HealthCosts::defaults();
        h.excess_inflation = 0.0;
        h.ltc = None;
        let v = yearly_costs(&h, &[person(1976), person(1976)], 2026, 95);
        assert_eq!(at(&v, 2026), Money::from_dollars(19_200));
    }

    #[test]
    fn long_term_care_is_the_expected_cost_for_the_years_needed() {
        let mut h = HealthCosts::defaults();
        h.excess_inflation = 0.0;
        h.out_of_pocket_65_plus = Money::ZERO;
        let v = yearly_costs(&h, &[person(1960)], 2026, 95);
        assert_eq!(at(&v, 1960 + 81), Money::ZERO);
        assert_eq!(at(&v, 1960 + 82), Money::from_dollars(55_000));
        assert_eq!(at(&v, 1960 + 84), Money::from_dollars(55_000));
        assert_eq!(at(&v, 1960 + 85), Money::ZERO);
    }

    #[test]
    fn healthcare_outgrows_prices() {
        let h = HealthCosts::defaults();
        let v = yearly_costs(&h, &[person(1990)], 2026, 95);
        assert!(at(&v, 2036) > Money::from_dollars(9_600));
    }

    #[test]
    fn no_people_no_costs_and_chance_is_clamped() {
        assert!(yearly_costs(&HealthCosts::defaults(), &[], 2026, 95).is_empty());
        let mut h = HealthCosts::defaults();
        h.excess_inflation = 0.0;
        h.out_of_pocket_65_plus = Money::ZERO;
        h.ltc.as_mut().unwrap().chance = 7.0;
        let v = yearly_costs(&h, &[person(1960)], 2026, 95);
        assert_eq!(at(&v, 1960 + 82), Money::from_dollars(110_000));
    }
}
