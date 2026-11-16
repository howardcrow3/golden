//! Savings goals: a target, a date, what's already set aside, and what that asks of each month.
//!
//! Whether a goal is within reach compares the monthly amount it needs with what the household
//! actually kept per month over the last three completed months. Nothing is reserved or moved;
//! this is arithmetic on the person's own numbers.
use crate::{parse_day, App};
use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};

const KEY: &str = "goals";
const MAX_GOALS: usize = 30;
const MAX_CENTS: i64 = 100_000_000_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Goal {
    pub name: String,
    pub target_cents: i64,
    pub saved_cents: i64,
    /// YYYY-MM-DD
    pub by_date: String,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GoalRow {
    pub name: String,
    pub target_cents: i64,
    pub saved_cents: i64,
    pub by_date: String,
    /// Whole months from today to the date (0 once it has passed).
    pub months_left: i64,
    pub monthly_needed_cents: i64,
    /// 0 to 100, rounded down.
    pub percent: i64,
    /// done, on_track, behind, past_due, or unknown (no history to compare with).
    pub status: String,
    /// What the household has been keeping per month, when there is history.
    pub typical_monthly_cents: Option<i64>,
}

fn months_between(from: NaiveDate, to: NaiveDate) -> i64 {
    let m = (to.year() - from.year()) as i64 * 12 + to.month() as i64 - from.month() as i64;
    // A part month still has to be saved for: round the remaining days up.
    if to.day() > from.day() {
        m + 1
    } else {
        m
    }
}

impl App {
    fn saved_goals(&self) -> Result<Vec<Goal>, String> {
        match self.db.get_setting(KEY).map_err(|e| e.to_string())? {
            Some(s) => {
                serde_json::from_str(&s).map_err(|e| format!("saved goals are unreadable: {e}"))
            }
            None => Ok(vec![]),
        }
    }

    fn write_goals(&self, g: &[Goal]) -> Result<(), String> {
        let json = serde_json::to_string(g).map_err(|e| e.to_string())?;
        self.db.set_setting(KEY, &json).map_err(|e| e.to_string())
    }

    /// Add a goal, or replace the one with the same name.
    pub fn set_goal(&self, goal: Goal) -> Result<(), String> {
        self.require_edit()?;
        let name = goal.name.trim().to_string();
        if name.is_empty() || name.len() > 60 {
            return Err("Give the goal a short name.".into());
        }
        if !(1..=MAX_CENTS).contains(&goal.target_cents) {
            return Err("The target must be more than zero.".into());
        }
        if !(0..=MAX_CENTS).contains(&goal.saved_cents) {
            return Err("What's saved so far can't be negative.".into());
        }
        parse_day(&goal.by_date).map_err(|_| "Enter the date as YYYY-MM-DD.".to_string())?;
        let mut all = self.saved_goals()?;
        let g = Goal {
            name: name.clone(),
            ..goal
        };
        match all.iter_mut().find(|x| x.name.eq_ignore_ascii_case(&name)) {
            Some(existing) => *existing = g,
            None => {
                if all.len() >= MAX_GOALS {
                    return Err("That is more goals than Golden can track (30).".into());
                }
                all.push(g);
            }
        }
        self.write_goals(&all)
    }

    pub fn remove_goal(&self, name: &str) -> Result<(), String> {
        self.require_edit()?;
        let mut all = self.saved_goals()?;
        let before = all.len();
        all.retain(|g| !g.name.eq_ignore_ascii_case(name));
        if all.len() == before {
            return Err("No such goal.".into());
        }
        self.write_goals(&all)
    }

    /// Goals soonest first, each with what it needs per month.
    pub fn goals(&self) -> Result<Vec<GoalRow>, String> {
        let today = (self.clock)();
        let this_month = today.with_day(1).expect("day 1");
        // What was kept per month over the three completed months before this one.
        let idx = this_month.year() * 12 + this_month.month0() as i32 - 3;
        let from = NaiveDate::from_ymd_opt(idx.div_euclid(12), idx.rem_euclid(12) as u32 + 1, 1)
            .expect("valid");
        let rows = self.cash_flow(from, this_month.pred_opt().expect("day"), "month")?;
        let active: Vec<_> = rows.iter().skip_while(|r| r.count == 0).collect();
        let typical = (!active.is_empty())
            .then(|| active.iter().map(|r| r.net_cents).sum::<i64>() / active.len() as i64);

        let mut out: Vec<GoalRow> = self
            .saved_goals()?
            .into_iter()
            .map(|g| {
                let due = parse_day(&g.by_date).unwrap_or(today);
                let left = (g.target_cents - g.saved_cents).max(0);
                let months = if due > today {
                    months_between(today, due).max(1)
                } else {
                    0
                };
                let needed = if months > 0 {
                    (left + months - 1) / months
                } else {
                    left
                };
                let status = if left == 0 {
                    "done"
                } else if months == 0 {
                    "past_due"
                } else {
                    match typical {
                        None => "unknown",
                        Some(t) if t >= needed => "on_track",
                        Some(_) => "behind",
                    }
                };
                GoalRow {
                    percent: (g.saved_cents * 100 / g.target_cents).clamp(0, 100),
                    name: g.name,
                    target_cents: g.target_cents,
                    saved_cents: g.saved_cents,
                    by_date: g.by_date,
                    months_left: months,
                    monthly_needed_cents: needed,
                    status: status.into(),
                    typical_monthly_cents: typical,
                }
            })
            .collect();
        out.sort_by(|a, b| a.by_date.cmp(&b.by_date).then(a.name.cmp(&b.name)));
        Ok(out)
    }
}
