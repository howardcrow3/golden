//! A refund cancels the purchase it returns: counting the purchase as spending and the refund as
//! income would overstate both. After each import or bank sync, a refund that equals an earlier
//! purchase from the same shop in the same account is paired with it, and the pair is left out
//! of income and spending totals. Both stay in the list, and balances are unchanged.
use crate::transfers::CATEGORY as TRANSFER;
use crate::App;
use chrono::NaiveDate;
use fin_db::dupes::same_payee;

/// How long after the purchase a refund may arrive.
const WINDOW_DAYS: i64 = 120;

impl App {
    /// Pair full refunds with their purchases. Returns how many transactions were newly marked.
    pub fn mark_refunds(&self) -> Result<usize, String> {
        let from = NaiveDate::from_ymd_opt(1900, 1, 1).unwrap();
        let to = NaiveDate::from_ymd_opt(2999, 12, 31).unwrap();
        let txns = self.db.txns_between(from, to).map_err(|e| e.to_string())?;
        let mut done: std::collections::HashSet<i64> = self
            .db
            .netted_ids()
            .map_err(|e| e.to_string())?
            .into_iter()
            .collect();
        let mut marked = vec![];
        for refund in txns.iter().filter(|t| t.amount.cents() > 0) {
            if done.contains(&refund.id) || refund.category.as_deref() == Some(TRANSFER) {
                continue;
            }
            // The nearest earlier purchase of exactly this amount from the same shop.
            let hit = txns
                .iter()
                .filter(|p| {
                    p.account == refund.account
                        && p.amount.cents() == -refund.amount.cents()
                        && !done.contains(&p.id)
                        && p.category.as_deref() != Some(TRANSFER)
                        && p.day <= refund.day
                        && (refund.day - p.day).num_days() <= WINDOW_DAYS
                        && !p.payee.trim().is_empty()
                        && same_payee(&p.payee, &refund.payee)
                })
                .max_by_key(|p| (p.day, p.id));
            if let Some(p) = hit {
                done.insert(p.id);
                done.insert(refund.id);
                marked.extend([p.id, refund.id]);
            }
        }
        self.db
            .set_netted(&marked, true)
            .map_err(|e| e.to_string())?;
        Ok(marked.len())
    }
}
