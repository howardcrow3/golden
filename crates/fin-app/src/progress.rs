//! Progress tracking: how your real net worth has moved against what the plan projected.
//!
//! "Start tracking" saves the plan's projected savings path and today's net worth. After that
//! the report compares, month by month, how much your net worth has changed against how much
//! the plan said your savings would change. The two are not the same measure (net worth also
//! holds a home and debts), so the comparison is of change, not of level.
use crate::App;
use chrono::{Datelike, NaiveDate};
use fin_plan::quickplan;
use serde::{Deserialize, Serialize};

const KEY: &str = "progress.baseline";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct Baseline {
    /// First day tracked, yyyy-mm-dd.
    started: String,
    /// Savings (cash, taxable, traditional, Roth) when tracking started, in cents.
    start_savings_cents: i64,
    /// Year-end projected savings, from the start year on.
    years: Vec<(i32, i64)>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProgressPoint {
    pub month: String,
    /// Net worth now minus net worth when tracking began.
    pub actual_change_cents: i64,
    /// Projected savings then minus projected savings at the start.
    pub projected_change_cents: i64,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProgressReport {
    pub started: String,
    pub points: Vec<ProgressPoint>,
    /// Latest actual change minus latest projected change; positive is ahead of the plan.
    pub ahead_cents: i64,
}

/// Projected savings on `day`, interpolating between year-ends (the year-end before the first
/// projected year is the starting savings).
fn projected_on(start_savings: i64, years: &[(i32, i64)], day: NaiveDate) -> i64 {
    let Some(&(first, _)) = years.first() else {
        return start_savings;
    };
    let y = day.year();
    if y < first {
        return start_savings;
    }
    let before = if y == first {
        start_savings
    } else {
        years
            .iter()
            .find(|(yr, _)| *yr == y - 1)
            .map_or(start_savings, |(_, b)| *b)
    };
    let Some(&(_, after)) = years.iter().find(|(yr, _)| *yr == y) else {
        return years.last().map_or(start_savings, |(_, b)| *b);
    };
    let frac = (day.ordinal0() as f64 + 1.0) / if day.leap_year() { 366.0 } else { 365.0 };
    before + ((after - before) as f64 * frac).round() as i64
}

fn month_end(d: NaiveDate) -> NaiveDate {
    let next = if d.month() == 12 {
        NaiveDate::from_ymd_opt(d.year() + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(d.year(), d.month() + 1, 1)
    };
    next.and_then(|n| n.pred_opt()).unwrap_or(d)
}

impl App {
    /// Save today's projected path and net worth as the starting point, replacing any earlier one.
    pub fn start_progress_tracking(&self) -> Result<(), String> {
        let today = (self.clock)();
        let h = self.household()?;
        let (plan, picks) = self.plan(today)?;
        let r = quickplan::run_plan(&plan, &picks, 10)?;
        let b = Baseline {
            started: today.format("%Y-%m-%d").to_string(),
            start_savings_cents: h.cash_cents
                + h.taxable_cents
                + h.traditional_cents
                + h.roth_cents,
            years: r.years.iter().map(|y| (y.year, y.balance_cents)).collect(),
        };
        self.db
            .set_setting(KEY, &serde_json::to_string(&b).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())
    }

    pub fn stop_progress_tracking(&self) -> Result<(), String> {
        self.db.set_setting(KEY, "").map_err(|e| e.to_string())
    }

    /// The comparison so far, or None before tracking has been started.
    pub fn progress(&self) -> Result<Option<ProgressReport>, String> {
        let raw = self.db.get_setting(KEY).map_err(|e| e.to_string())?;
        let Some(raw) = raw.filter(|r| !r.is_empty()) else {
            return Ok(None);
        };
        let b: Baseline = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
        let started =
            NaiveDate::parse_from_str(&b.started, "%Y-%m-%d").map_err(|e| e.to_string())?;
        let today = (self.clock)();
        let months = ((today.year() - started.year()) * 12 + today.month() as i32
            - started.month() as i32
            + 1)
        .clamp(1, 120) as u32;
        let hist = self.net_worth_history(months.max(1))?;
        let base_nw = hist.first().map_or(0, |p| p.net_worth_cents);
        // Both measures are zero at the end of the month tracking began in.
        let start_projected = projected_on(
            b.start_savings_cents,
            &b.years,
            month_end(started).min(today),
        );
        let mut points = Vec::new();
        for p in &hist {
            let first = NaiveDate::parse_from_str(&format!("{}-01", p.month), "%Y-%m-%d")
                .map_err(|e| e.to_string())?;
            // Last day of the month, or today for the current one.
            let day = month_end(first).min(today);
            points.push(ProgressPoint {
                month: p.month.clone(),
                actual_change_cents: p.net_worth_cents - base_nw,
                projected_change_cents: projected_on(b.start_savings_cents, &b.years, day)
                    - start_projected,
            });
        }
        let ahead = points
            .last()
            .map_or(0, |p| p.actual_change_cents - p.projected_change_cents);
        Ok(Some(ProgressReport {
            started: b.started,
            points,
            ahead_cents: ahead,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn projection_interpolates_between_year_ends() {
        let years = [(2026, 1_200_000), (2027, 2_400_000)];
        assert_eq!(projected_on(1_000_000, &years, d(2025, 6, 1)), 1_000_000);
        // End of 2026 is the 2026 figure; end of 2027 the 2027 one.
        assert_eq!(projected_on(1_000_000, &years, d(2026, 12, 31)), 1_200_000);
        assert_eq!(projected_on(1_000_000, &years, d(2027, 12, 31)), 2_400_000);
        // Past the end of the plan it stays at the last figure.
        assert_eq!(projected_on(1_000_000, &years, d(2040, 1, 1)), 2_400_000);
        // Part-way through 2027 sits between the two year-ends.
        let mid = projected_on(1_000_000, &years, d(2027, 7, 2));
        assert!(mid > 1_200_000 && mid < 2_400_000);
    }

    #[test]
    fn no_projection_means_no_change() {
        assert_eq!(projected_on(500, &[], d(2026, 1, 1)), 500);
    }
}
