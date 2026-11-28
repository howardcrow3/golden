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
    /// The bank's id for a pending line this one replaces (it posted). That line is deleted and the
    /// category you chose for it carries over.
    pub replaces: Option<String>,
    pub category: Option<String>,
    /// The bank connection (Plaid item) that sent the line; empty when unknown. A second
    /// connection to the same bank sends the same history under new ids, and this is how
    /// those copies are told apart from genuine repeats.
    pub item: String,
}

/// The stored key of a bank line: `bank|<item>|<id>`, or `bank|<id>` for lines saved before
/// the connection was recorded.
fn bank_key(item: &str, external_id: &str) -> String {
    if item.is_empty() {
        format!("bank|{external_id}")
    } else {
        format!("bank|{item}|{external_id}")
    }
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

    /// The Golden accounts a bank item feeds.
    pub fn bank_item_accounts(&self, item_id: &str) -> Result<Vec<AccountId>> {
        let mut st = self.conn.prepare(
            "SELECT DISTINCT account_id FROM bank_account WHERE item_id=?1 ORDER BY account_id",
        )?;
        let it = st.query_map([item_id], |r| r.get::<_, i64>(0).map(AccountId))?;
        Ok(it.collect::<std::result::Result<_, rusqlite::Error>>()?)
    }

    /// Forget a bank item's account links and delete the lines it synced (unlinking a bank).
    pub fn unlink_bank_item(&self, item_id: &str) -> Result<usize> {
        // All or nothing: a still-linked bank with its lines gone would sync them back in.
        self.tag_bank_lines()?;
        let tx = self.conn.unchecked_transaction()?;
        // This connection's own lines, and older lines of unknown origin in its accounts. Lines
        // another connection to the same bank now reports stay.
        let n = tx.execute(
            "DELETE FROM txn WHERE external_id IS NOT NULL AND (
               substr(dedupe_key, 1, length(?2)) = ?2
               OR (dedupe_key = 'bank|' || external_id AND account_id IN
                    (SELECT account_id FROM bank_account WHERE item_id=?1)))",
            params![item_id, format!("bank|{item_id}|")],
        )?;
        tx.execute("DELETE FROM bank_account WHERE item_id=?1", [item_id])?;
        tx.commit()?;
        Ok(n)
    }

    /// Record which connection older bank lines came from, where that is certain: the line's
    /// account is fed by exactly one connection.
    pub fn tag_bank_lines(&self) -> Result<usize> {
        Ok(self.conn.execute(
            "UPDATE txn SET dedupe_key = 'bank|' ||
                 (SELECT min(item_id) FROM bank_account b WHERE b.account_id = txn.account_id)
                 || '|' || external_id
             WHERE external_id IS NOT NULL AND dedupe_key = 'bank|' || external_id
               AND (SELECT count(DISTINCT item_id) FROM bank_account b
                    WHERE b.account_id = txn.account_id) = 1",
            [],
        )?)
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
            // A statement already covers this line (or the pending line it posts as): adding it
            // again would count it twice.
            let reconciled = |id: &str| -> Result<bool> {
                Ok(tx
                    .query_row(
                        "SELECT 1 FROM bank_reconciled WHERE external_id=?1",
                        [id],
                        |_| Ok(()),
                    )
                    .optional()?
                    .is_some())
            };
            if reconciled(&t.external_id)? || t.replaces.as_deref().map_or(Ok(false), reconciled)? {
                tx.execute(
                    "INSERT OR IGNORE INTO bank_reconciled(external_id, account_id) VALUES (?1,?2)",
                    params![t.external_id, t.account.0],
                )?;
                continue;
            }
            // The same line already came in through another connection to this bank (the bank
            // was connected twice): take that copy over instead of counting it again. Each copy
            // is taken once, and lines from the same connection never match, so genuine repeats
            // (two identical coffees) all stay.
            let replaces_present = match &t.replaces {
                Some(old) => tx
                    .query_row("SELECT 1 FROM txn WHERE external_id=?1", [old], |_| Ok(()))
                    .optional()?
                    .is_some(),
                None => false,
            };
            if !t.item.is_empty() && !replaces_present {
                let own = format!("bank|{}|", t.item);
                let copy: Option<i64> = tx
                    .query_row(
                        "SELECT id FROM txn WHERE account_id=?1 AND day=?2 AND cents=?3 AND payee=?4
                           AND external_id IS NOT NULL AND dedupe_key LIKE 'bank|%|%'
                           AND substr(dedupe_key, 1, length(?5)) != ?5
                         ORDER BY id LIMIT 1",
                        params![t.account.0, t.day, t.amount.cents(), t.payee, own],
                        |r| r.get(0),
                    )
                    .optional()?;
                if let Some(id) = copy {
                    tx.execute(
                        "UPDATE txn SET external_id=?2, dedupe_key=?3, memo=?4, pending=?5 WHERE id=?1",
                        params![
                            id,
                            t.external_id,
                            bank_key(&t.item, &t.external_id),
                            t.memo,
                            t.pending
                        ],
                    )?;
                    sum.updated += 1;
                    continue;
                }
            }
            // A pending line that has posted: take over its category, then drop it.
            let inherited: Option<i64> = match &t.replaces {
                Some(old) => {
                    let c = tx
                        .query_row(
                            "SELECT category_id FROM txn WHERE external_id=?1",
                            [old],
                            |r| r.get::<_, Option<i64>>(0),
                        )
                        .optional()?
                        .flatten();
                    tx.execute("DELETE FROM txn WHERE external_id=?1", [old])?;
                    c
                }
                None => None,
            };
            let cat: Option<i64> = match &t.category {
                _ if inherited.is_some() => inherited,
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
                    bank_key(&t.item, &t.external_id),
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
        let rows: Vec<(i64, NaiveDate, i64, bool, Option<String>)> = {
            let mut st = tx.prepare(
                "SELECT id, day, cents, external_id IS NOT NULL, external_id FROM txn
                 WHERE account_id=?1 ORDER BY day, id",
            )?;
            let it = st.query_map([account.0], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
            })?;
            it.collect::<std::result::Result<_, rusqlite::Error>>()?
        };
        let statement: Vec<_> = rows.iter().filter(|r| !r.3).collect();
        let mut used = vec![false; statement.len()];
        let mut drop_ids = vec![];
        for b in rows.iter().filter(|r| r.3) {
            let hit = statement.iter().enumerate().find(|(i, s)| {
                !used[*i] && s.2 == b.2 && (s.1 - b.1).num_days().abs() <= MATCH_DAYS
            });
            if let Some((i, _)) = hit {
                used[i] = true;
                drop_ids.push((b.0, b.4.clone()));
            }
        }
        for (id, external) in &drop_ids {
            tx.execute("DELETE FROM txn WHERE id=?1", [id])?;
            tx.execute(
                "INSERT OR IGNORE INTO bank_reconciled(external_id, account_id) VALUES (?1,?2)",
                params![external, account.0],
            )?;
        }
        tx.commit()?;
        Ok(drop_ids.len())
    }
}
