//! Money moved between the household's own accounts (a card payment, a savings sweep) is not
//! income on one side and spending on the other. After each import, equal and opposite amounts
//! in different accounts a few days apart are filed as "Transfer", which the totals leave out.
//! Net worth is unaffected: it adds up every transaction regardless of category.
use crate::App;
use chrono::NaiveDate;
use fin_core::Money;
use fin_ledger::transfers::{match_transfers, Leg};

/// How many days apart the two sides may post.
const WINDOW_DAYS: i64 = 4;
pub const CATEGORY: &str = "Transfer";

impl App {
    /// Pair up unclassified transactions that are the two sides of one transfer and file both as
    /// "Transfer". Anything the person or a rule already categorized is left alone. Returns how
    /// many transactions were marked.
    pub fn mark_transfers(&self) -> Result<usize, String> {
        let from = NaiveDate::from_ymd_opt(1900, 1, 1).unwrap();
        let to = NaiveDate::from_ymd_opt(2999, 12, 31).unwrap();
        // A purchase or refund already paired with each other is not also half of a transfer.
        let netted: std::collections::HashSet<i64> = self
            .db
            .netted_ids()
            .map_err(|e| e.to_string())?
            .into_iter()
            .collect();
        let legs: Vec<Leg> = self
            .db
            .txns_between(from, to)
            .map_err(|e| e.to_string())?
            .into_iter()
            .filter(|t| t.category.is_none() && !netted.contains(&t.id))
            .map(|t| Leg {
                id: t.id,
                account: t.account.0,
                day: t.day,
                amount: Money::from_cents(t.amount.cents()),
            })
            .collect();
        let pairs = match_transfers(&legs, WINDOW_DAYS);
        for (a, b) in &pairs {
            self.db
                .set_txn_category(*a, Some(CATEGORY))
                .map_err(|e| e.to_string())?;
            self.db
                .set_txn_category(*b, Some(CATEGORY))
                .map_err(|e| e.to_string())?;
        }
        Ok(pairs.len() * 2)
    }
}
