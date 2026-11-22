//! "Retire when the plan works": try each retirement age for the first person and report the
//! earliest one where the plan succeeds in at least `target` of the simulated futures.
use crate::household::Household;
use fin_core::NaiveDate;
use fin_plan::quickplan;
use serde::Serialize;

/// The oldest retirement age tried.
pub const LAST_AGE: i32 = 75;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Try {
    pub age: i32,
    pub success_rate: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RetireWhen {
    pub target: f64,
    /// The earliest age that meets the target, if any up to 75 does.
    pub age: Option<i32>,
    /// The age the household has set now.
    pub current_age: i32,
    /// Every age tried, so the screen can show the trade-off.
    pub tries: Vec<Try>,
}

/// Scan retirement ages from the first person's age now (or 40, if younger people are not
/// supported) up to 75. `runs` trials each, so keep it modest.
pub fn scan(
    h: &Household,
    today: NaiveDate,
    target: f64,
    runs: usize,
) -> Result<RetireWhen, String> {
    let target = target.clamp(0.5, 0.99);
    let first = h.people.first().ok_or("Add a person first.")?;
    let current_age = first.retire_age;
    let birth = NaiveDate::parse_from_str(&first.birth_date, "%Y-%m-%d")
        .map_err(|_| "Enter a valid birth date.".to_string())?;
    let age_now = fin_core::age_on(birth, today);
    let picks = h.picks.clone();
    let mut tries = Vec::new();
    let mut found = None;
    for age in age_now.max(30)..=LAST_AGE {
        let mut trial = h.clone();
        trial.people[0].retire_age = age;
        let plan = trial.plan_input(today)?;
        let r = quickplan::run_plan(&plan, &picks, runs)?;
        tries.push(Try {
            age,
            success_rate: r.success_rate,
        });
        if found.is_none() && r.success_rate >= target {
            found = Some(age);
            break;
        }
    }
    Ok(RetireWhen {
        target,
        age: found,
        current_age,
        tries,
    })
}
