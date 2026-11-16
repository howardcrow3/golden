//! Net worth at the end of each past month, rebuilt from the ledger and starting balances.
//!
//! Because debts are accounts whose balance is negative, net worth is simply the sum of every
//! account's balance, so the history is the running total of all transactions plus the starting
//! balances. Months before the first statement show only the starting balances.
use crate::App;
use chrono::{Datelike, NaiveDate};
use serde::Serialize;

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NetWorthPoint {
    pub month: String,
    pub net_worth_cents: i64,
}

fn month_start(d: NaiveDate, back: i32) -> NaiveDate {
    let idx = d.year() * 12 + d.month0() as i32 - back;
    NaiveDate::from_ymd_opt(idx.div_euclid(12), idx.rem_euclid(12) as u32 + 1, 1).expect("valid")
}

impl App {
    /// Month-end net worth for the last `months` months (1 to 120), oldest first. The current
    /// month's point is as of today.
    pub fn net_worth_history(&self, months: u32) -> Result<Vec<NetWorthPoint>, String> {
        if !(1..=120).contains(&months) {
            return Err("Choose between 1 and 120 months.".into());
        }
        let today = (self.clock)();
        let first = month_start(today, months as i32 - 1);
        let opening: i64 = self
            .db
            .openings()
            .map_err(|e| e.to_string())?
            .iter()
            .map(|(_, c)| *c)
            .sum();
        let mut txns = self
            .db
            .txns_between(
                NaiveDate::from_ymd_opt(1900, 1, 1).expect("valid"),
                NaiveDate::from_ymd_opt(2200, 1, 1).expect("valid"),
            )
            .map_err(|e| e.to_string())?;
        txns.sort_by_key(|t| t.day);
        let mut i = 0;
        let mut running = opening;
        let mut out = vec![];
        for k in 0..months as i32 {
            let start = month_start(first, -k);
            let end = month_start(first, -(k + 1))
                .pred_opt()
                .expect("day")
                .min(today);
            while i < txns.len() && txns[i].day <= end {
                running += txns[i].amount.cents();
                i += 1;
            }
            out.push(NetWorthPoint {
                month: start.format("%Y-%m").to_string(),
                net_worth_cents: running,
            });
        }
        Ok(out)
    }
}
