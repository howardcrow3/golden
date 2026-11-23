//! Lines synced from a bank connection. They carry the bank's own id so a later sync can update or
//! remove them, and they are provisional: when the month's statement is imported, [`Db::reconcile_bank`]
//! drops the bank lines the statement now covers, so the statement stays the record.
use crate::ledger::AccountId;
use crate::{Db, Result};
use chrono::NaiveDate;
use fin_core::{Grain, Money, PeriodKey};
use rusqlite::{params, OptionalExtension};

/// How many days apart a bank line and a statement line may be and still count as the same.
pub const MATCH_DAYS: i64 = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BankTxn {
    /// The bank's id for the line, stable across syncs.
    pub external_id: String,
    pub account: AccountId,
    pub day: NaiveDate,
    pub amount: Money,
    pub payee: String,
    pub memo: String,
    pub pending: bool,
    pub category: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BankSummary {
    pub added: usize,
    pub updated: usize,
    pub removed: usize,
}

impl Db {
    /// Remember which Golden account a bank's account feeds.
    pub fn link_bank_account(
        &self,
        bank_account_id: &str,
        item_id: &str,
        account: AccountId,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT INTO bank_account(bank_account_id, item_id, account_id) VALUES (?1,?2,?3)
             ON CONFLICT(bank_account_id) DO UPDATE SET item_id=?2, account_id=?3",
            params![bank_account_id, item_id, account.0],
        )?;
        Ok(())
    }

    pub fn bank_account(&self, bank_account_id: &str) -> Result<Option<AccountId>> {
        Ok(self
            .conn
            .query_row(
                "SELECT account_id FROM bank_account WHERE bank_account_id=?1",
                [bank_account_id],
                |r| r.get::<_, i64>(0),
            )
            .optional()?
            .map(AccountId))
    }

    /// Forget a bank item's account links and delete the lines it synced (unlinking a bank).
    pub fn unlink_bank_item(&self, item_id: &str) -> Result<usize> {
        let n = self.conn.execute(
            "DELETE FROM txn WHERE external_id IS NOT NULL AND account_id IN
               (SELECT account_id FROM bank_account WHERE item_id=?1)",
            [item_id],
        )?;
        self.conn
            .execute("DELETE FROM bank_account WHERE item_id=?1", [item_id])?;
        Ok(n)
    }

    /// Apply one sync's changes atomically. An id seen before is updated (its category, which the
    /// person may have changed, is kept); a new id is inserted; removed ids are deleted.
    pub fn apply_bank_changes(
        &mut self,
        upserts: &[BankTxn],
        removed: &[String],
    ) -> Result<BankSummary> {
        let tx = self.conn.transaction()?;
        let mut sum = BankSummary::default();
        for t in upserts {
            let keys = [
                PeriodKey::containing(Grain::Week, t.day).to_string(),
                PeriodKey::containing(Grain::Month, t.day).to_string(),
                PeriodKey::containing(Grain::Quarter, t.day).to_string(),
                PeriodKey::containing(Grain::Year, t.day).to_string(),
            ];
            let updated = tx.execute(
                "UPDATE txn SET account_id=?2, day=?3, week_key=?4, month_key=?5, quarter_key=?6,
                    year_key=?7, cents=?8, payee=?9, memo=?10, pending=?11 WHERE external_id=?1",
                params![
                    t.external_id,
                    t.account.0,
                    t.day,
                    keys[0],
                    keys[1],
                    keys[2],
                    keys[3],
                    t.amount.cents(),
                    t.payee,
                    t.memo,
                    t.pending
                ],
            )?;
            if updated > 0 {
                sum.updated += 1;
                continue;
            }
            let cat: Option<i64> = match &t.category {
                Some(c) => {
                    let found: Option<i64> = tx
                        .query_row("SELECT id FROM category WHERE name=?1", [c], |r| r.get(0))
                        .optional()?;
                    Some(match found {
                        Some(id) => id,
                        None => {
                            tx.execute(
                                "INSERT INTO category(name, kind) VALUES (?1,'expense')",
                                [c],
                            )?;
                            tx.last_insert_rowid()
                        }
                    })
                }
                None => None,
            };
            tx.execute(
                "INSERT INTO txn(account_id, day, week_key, month_key, quarter_key, year_key, cents,
                    payee, memo, category_id, dedupe_key, external_id, pending)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
                params![
                    t.account.0,
                    t.day,
                    keys[0],
                    keys[1],
                    keys[2],
                    keys[3],
                    t.amount.cents(),
                    t.payee,
                    t.memo,
                    cat,
                    format!("bank|{}", t.external_id),
                    t.external_id,
                    t.pending
                ],
            )?;
            sum.added += 1;
        }
        for id in removed {
            sum.removed += tx.execute("DELETE FROM txn WHERE external_id=?1", [id])?;
        }
        tx.commit()?;
        Ok(sum)
    }

    /// Drop bank lines that a statement now covers: same account, same amount, within
    /// [`MATCH_DAYS`] days. Each statement line covers at most one bank line. Returns how many
    /// bank lines were dropped.
    pub fn reconcile_bank(&mut self, account: AccountId) -> Result<usize> {
        let tx = self.conn.transaction()?;
        let rows: Vec<(i64, NaiveDate, i64, bool)> = {
            let mut st = tx.prepare(
                "SELECT id, day, cents, external_id IS NOT NULL FROM txn WHERE account_id=?1
                 ORDER BY day, id",
            )?;
            let it = st.query_map([account.0], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
            })?;
            it.collect::<std::result::Result<_, rusqlite::Error>>()?
        };
        let statement: Vec<&(i64, NaiveDate, i64, bool)> = rows.iter().filter(|r| !r.3).collect();
        let mut used = vec![false; statement.len()];
        let mut drop_ids = vec![];
        for b in rows.iter().filter(|r| r.3) {
            let hit = statement.iter().enumerate().find(|(i, s)| {
                !used[*i] && s.2 == b.2 && (s.1 - b.1).num_days().abs() <= MATCH_DAYS
            });
            if let Some((i, _)) = hit {
                used[i] = true;
                drop_ids.push(b.0);
            }
        }
        for id in &drop_ids {
            tx.execute("DELETE FROM txn WHERE id=?1", [id])?;
        }
        tx.commit()?;
        Ok(drop_ids.len())
    }
}
