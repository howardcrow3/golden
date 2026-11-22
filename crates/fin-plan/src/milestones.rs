//! Milestones along the projection: the years things start, stop or become possible.
use crate::projection::{PlanInput, Projection};
use fin_core::Money;
use serde::Serialize;

/// Portfolio multiple of yearly spending that counts as financially independent (a 4% rule).
pub const FI_MULTIPLE: i64 = 25;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Milestone {
    pub key: String,
    pub label: String,
    pub year: i32,
    pub age: i32,
}

fn m(key: &str, label: &str, year: i32, age: i32) -> Milestone {
    Milestone {
        key: key.into(),
        label: label.into(),
        year,
        age,
    }
}

/// Find the milestones the projection reaches, in date order. `real_return` is the yearly
/// return after inflation, used for Coast FI.
pub fn find(plan: &PlanInput, p: &Projection, real_return: f64) -> Vec<Milestone> {
    let mut out = Vec::new();
    let rows = &p.rows;
    let need = Money::from_cents(plan.annual_spending.cents() * FI_MULTIPLE);

    // Retirement: the first year without earnings after a year with some.
    let retire = rows
        .windows(2)
        .find(|w| w[0].earned.cents() > 0 && w[1].earned.cents() == 0)
        .map(|w| &w[1]);
    if let Some(r) = retire {
        out.push(m("retire", "Work ends", r.year, r.age));
    }
    if let Some(r) = rows
        .iter()
        .find(|r| r.end_balance >= Money::from_cents(100_000_000))
    {
        out.push(m("million", "Savings pass $1 million", r.year, r.age));
    }
    if let Some(r) = rows.iter().find(|r| r.end_balance >= need) {
        out.push(m(
            "fi",
            "Financially independent (25 times a year of spending)",
            r.year,
            r.age,
        ));
    }
    // Coast FI: savings left alone, with no more contributions, reach the FI number by the
    // year work ends.
    if let Some(r) = retire {
        let hit = rows.iter().take_while(|x| x.year < r.year).find(|x| {
            let years = r.year - x.year;
            let grown = x.end_balance.cents() as f64 * (1.0 + real_return).powi(years);
            grown >= need.cents() as f64
        });
        if let Some(c) = hit {
            out.push(m(
                "coast",
                "Coast FI: savings can grow to your number without more saving",
                c.year,
                c.age,
            ));
        }
    }
    if let Some(r) = rows.iter().find(|r| r.social_security.cents() > 0) {
        out.push(m("ss", "Social Security starts", r.year, r.age));
    }
    if let Some(r) = rows.iter().find(|r| r.medicare.cents() > 0) {
        out.push(m("medicare", "Medicare premiums begin", r.year, r.age));
    }
    if let Some(r) = rows.iter().find(|r| r.rmd.cents() > 0) {
        out.push(m("rmd", "Required withdrawals begin", r.year, r.age));
    }
    if let Some(y) = p.depleted_year {
        if let Some(r) = rows.iter().find(|r| r.year == y) {
            out.push(m("depleted", "Savings run out", r.year, r.age));
        }
    }
    out.sort_by_key(|x| (x.year, x.key.clone()));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::{Assumptions, Picks};
    use crate::projection::fixtures::couple;
    use crate::scenario;

    fn run() -> (PlanInput, Projection, f64) {
        let plan = couple();
        let r = scenario::run(&plan, &Assumptions::defaults(), &Picks::new(), 10, 1).unwrap();
        (plan, r.projection, r.inputs.real_return)
    }

    #[test]
    fn finds_milestones_in_date_order() {
        let (plan, p, rr) = run();
        let ms = find(&plan, &p, rr);
        assert!(!ms.is_empty());
        assert!(ms.windows(2).all(|w| w[0].year <= w[1].year));
        assert!(ms.iter().any(|x| x.key == "ss"));
    }

    #[test]
    fn coast_fi_comes_before_work_ends() {
        let (plan, p, rr) = run();
        let ms = find(&plan, &p, rr);
        let retire = ms.iter().find(|x| x.key == "retire");
        let coast = ms.iter().find(|x| x.key == "coast");
        if let (Some(r), Some(c)) = (retire, coast) {
            assert!(c.year < r.year);
        }
    }

    #[test]
    fn nothing_is_invented_for_an_empty_projection() {
        let (plan, mut p, _) = run();
        p.rows.clear();
        p.depleted_year = None;
        assert!(find(&plan, &p, 0.03).is_empty());
    }

    #[test]
    fn higher_return_never_delays_coast_fi() {
        let (plan, p, _) = run();
        let lo = find(&plan, &p, 0.01).into_iter().find(|x| x.key == "coast");
        let hi = find(&plan, &p, 0.06).into_iter().find(|x| x.key == "coast");
        if let (Some(l), Some(h)) = (&lo, &hi) {
            assert!(h.year <= l.year);
        } else {
            assert!(lo.is_none() || hi.is_some());
        }
    }
}
