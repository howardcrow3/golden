//! A monthly projection of the household's position from its own history.
//!
//! The method is deliberately simple and explained on screen:
//! - Baseline spending and income are the average of the last (up to) twelve completed months.
//! - With a full year of history, each month's income blends that average with what came in
//!   during the same calendar month a year earlier, so bonuses, vests and dividends show up on
//!   their real months instead of being smeared across the year.
//! - The current month projects only what is still expected beyond what has already happened.
//! - The starting position is the net of everything imported so far.
use crate::App;
use chrono::{Datelike, NaiveDate};
use serde::Serialize;

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ForecastMonth {
    pub month: String,
    pub income_cents: i64,
    pub spending_cents: i64,
    pub net_cents: i64,
    /// Position at the end of this month.
    pub balance_cents: i64,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Forecast {
    pub start_balance_cents: i64,
    pub history_months: usize,
    /// True when each month's income uses the same month a year earlier.
    pub seasonal_income: bool,
    pub months: Vec<ForecastMonth>,
}

fn first_of(d: NaiveDate) -> NaiveDate {
    d.with_day(1).expect("day 1 exists")
}

fn add_months(d: NaiveDate, n: i32) -> NaiveDate {
    let idx = d.year() * 12 + d.month0() as i32 + n;
    NaiveDate::from_ymd_opt(idx.div_euclid(12), idx.rem_euclid(12) as u32 + 1, 1).expect("valid")
}

fn mean(v: &[i64]) -> i64 {
    if v.is_empty() {
        return 0;
    }
    let n = v.len() as i64;
    let s: i64 = v.iter().sum();
    (s + n / 2).div_euclid(n)
}

impl App {
    /// Project `months` months starting with the current one (1 to 60).
    pub fn forecast(&self, months: u32) -> Result<Forecast, String> {
        if !(1..=60).contains(&months) {
            return Err("Choose between 1 and 60 months.".into());
        }
        let today = (self.clock)();
        let this_month = first_of(today);
        let history_from = add_months(this_month, -24);
        let rows = self.cash_flow(history_from, add_months(this_month, -1), "month")?;
        // Start at the first month with any activity; leading empty months aren't history.
        let first_active = rows.iter().position(|r| r.count > 0);
        let hist: Vec<_> = match first_active {
            Some(i) => rows[i..].iter().collect(),
            None => vec![],
        };
        if hist.is_empty() {
            return Err("Import at least one full month of statements to see a projection.".into());
        }
        let recent: Vec<_> = hist.iter().rev().take(12).collect();
        let avg_income = mean(&recent.iter().map(|r| r.income_cents).collect::<Vec<_>>());
        let avg_spending = mean(&recent.iter().map(|r| r.spending_cents).collect::<Vec<_>>());
        let seasonal = hist.len() >= 12;
        let income_by_month: std::collections::HashMap<String, i64> = hist
            .iter()
            .map(|r| (r.period.clone(), r.income_cents))
            .collect();

        let start: i64 = self.accounts()?.iter().map(|a| a.balance_cents).sum();
        let so_far = self
            .cash_flow(this_month, this_month, "month")?
            .into_iter()
            .next();
        let (actual_inc, actual_sp) = so_far
            .map(|r| (r.income_cents, r.spending_cents))
            .unwrap_or((0, 0));

        let mut balance = start;
        let mut out = vec![];
        for i in 0..months as i32 {
            let m = add_months(this_month, i);
            let mut income = avg_income;
            if seasonal {
                let last_year = add_months(m, -12).format("%Y-%m").to_string();
                if let Some(v) = income_by_month.get(&last_year) {
                    income = (avg_income + v + 1).div_euclid(2);
                }
            }
            let mut spending = avg_spending;
            if i == 0 {
                income = (income - actual_inc).max(0);
                spending = (spending - actual_sp).max(0);
            }
            let net = income - spending;
            balance += net;
            out.push(ForecastMonth {
                month: m.format("%Y-%m").to_string(),
                income_cents: income,
                spending_cents: spending,
                net_cents: net,
                balance_cents: balance,
            });
        }
        Ok(Forecast {
            start_balance_cents: start,
            history_months: hist.len(),
            seasonal_income: seasonal,
            months: out,
        })
    }
}
