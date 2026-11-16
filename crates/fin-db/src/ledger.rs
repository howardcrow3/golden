use crate::{Db, DbError, Result};
use chrono::NaiveDate;
use fin_core::{Grain, Money, PeriodKey};
use rusqlite::{params, OptionalExtension};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccountId(pub i64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewTxn {
    pub account: AccountId,
    pub day: NaiveDate,
    pub amount: Money,
    pub payee: String,
    pub memo: String,
    pub category: Option<String>,
    pub stream: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Txn {
    pub id: i64,
    pub account: AccountId,
    pub day: NaiveDate,
    pub amount: Money,
    pub payee: String,
    pub memo: String,
    pub category: Option<String>,
    pub stream: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeriodTotal {
    pub period: String,
    pub income: Money,
    pub spending: Money,
    pub net: Money,
    pub count: i64,
}

/// What to group category totals by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Metric {
    Spending,
    Income,
}

fn grain_col(g: Grain) -> &'static str {
    match g {
        Grain::Week => "week_key",
        Grain::Month => "month_key",
        Grain::Quarter => "quarter_key",
        Grain::Year => "year_key",
    }
}

impl Db {
    pub fn add_account(
        &self,
        name: &str,
        kind: &str,
        institution: Option<&str>,
        last4: Option<&str>,
    ) -> Result<AccountId> {
        self.conn.execute(
            "INSERT INTO account(name, kind, institution, last4) VALUES (?1,?2,?3,?4)",
            params![name, kind, institution, last4],
        )?;
        Ok(AccountId(self.conn.last_insert_rowid()))
    }

    /// Find an account by exact name, creating it when missing.
    pub fn find_or_add_account(&self, name: &str, kind: &str) -> Result<AccountId> {
        if let Some(id) = self
            .conn
            .query_row("SELECT id FROM account WHERE name=?1", [name], |r| r.get(0))
            .optional()?
        {
            return Ok(AccountId(id));
        }
        self.add_account(name, kind, None, None)
    }

    /// Set what an account held before its first imported statement.
    pub fn set_opening(&self, id: AccountId, cents: i64) -> Result<()> {
        let n = self.conn.execute(
            "UPDATE account SET opening_cents=?1 WHERE id=?2",
            params![cents, id.0],
        )?;
        if n == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows.into());
        }
        Ok(())
    }

    /// Opening balance per open account.
    pub fn openings(&self) -> Result<Vec<(AccountId, i64)>> {
        let mut st = self
            .conn
            .prepare("SELECT id, opening_cents FROM account WHERE closed=0 ORDER BY id")?;
        let rows = st.query_map([], |r| Ok((AccountId(r.get(0)?), r.get::<_, i64>(1)?)))?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    /// All open accounts with their current balance, oldest first.
    pub fn accounts(&self) -> Result<Vec<(AccountId, String, String, Money)>> {
        let mut st = self.conn.prepare(
            "SELECT a.id, a.name, a.kind, a.opening_cents + coalesce(sum(t.cents),0) FROM account a
             LEFT JOIN txn t ON t.account_id=a.id WHERE a.closed=0 GROUP BY a.id ORDER BY a.id",
        )?;
        let rows = st.query_map([], |r| {
            Ok((
                AccountId(r.get(0)?),
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                Money::from_cents(r.get(3)?),
            ))
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    fn category_id(&self, name: &str) -> Result<i64> {
        if let Some(id) = self
            .conn
            .query_row("SELECT id FROM category WHERE name=?1", [name], |r| {
                r.get(0)
            })
            .optional()?
        {
            return Ok(id);
        }
        self.conn.execute(
            "INSERT INTO category(name, kind) VALUES (?1,'expense')",
            [name],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Insert a transaction. Returns `Ok(None)` when an identical one (same account, day,
    /// amount, payee, memo) was already imported, so re-importing a file adds nothing.
    pub fn add_txn(&self, t: &NewTxn) -> Result<Option<i64>> {
        let cat = match &t.category {
            Some(c) => Some(self.category_id(c)?),
            None => None,
        };
        let key = format!(
            "{}|{}|{}|{}|{}",
            t.account.0,
            t.day,
            t.amount.cents(),
            t.payee,
            t.memo
        );
        let n = self.conn.execute(
            "INSERT OR IGNORE INTO txn(account_id, day, week_key, month_key, quarter_key, year_key, cents, payee, memo, category_id, stream, dedupe_key)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
            params![
                t.account.0,
                t.day,
                PeriodKey::containing(Grain::Week, t.day).to_string(),
                PeriodKey::containing(Grain::Month, t.day).to_string(),
                PeriodKey::containing(Grain::Quarter, t.day).to_string(),
                PeriodKey::containing(Grain::Year, t.day).to_string(),
                t.amount.cents(),
                t.payee,
                t.memo,
                cat,
                t.stream,
                format!("{key}#0"),
            ],
        )?;
        Ok(if n == 0 {
            None
        } else {
            Some(self.conn.last_insert_rowid())
        })
    }

    /// Import a whole file's rows atomically. Identical rows inside the batch get distinct
    /// sequence numbers (two $4 coffees on one day), and re-importing the same batch adds nothing.
    pub fn import_batch(&mut self, rows: &[NewTxn]) -> Result<ImportSummary> {
        let tx = self.conn.transaction()?;
        let mut seen: std::collections::HashMap<String, usize> = Default::default();
        let mut summary = ImportSummary::default();
        for t in rows {
            let key = format!(
                "{}|{}|{}|{}|{}",
                t.account.0,
                t.day,
                t.amount.cents(),
                t.payee,
                t.memo
            );
            let seq = seen.entry(key.clone()).or_insert(0);
            let dedupe = format!("{key}#{seq}");
            *seq += 1;
            let cat: Option<i64> = match &t.category {
                Some(c) => {
                    let existing: Option<i64> = tx
                        .query_row("SELECT id FROM category WHERE name=?1", [c], |r| r.get(0))
                        .optional()?;
                    Some(match existing {
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
            let n = tx.execute(
                "INSERT OR IGNORE INTO txn(account_id, day, week_key, month_key, quarter_key, year_key, cents, payee, memo, category_id, stream, dedupe_key)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
                params![
                    t.account.0,
                    t.day,
                    PeriodKey::containing(Grain::Week, t.day).to_string(),
                    PeriodKey::containing(Grain::Month, t.day).to_string(),
                    PeriodKey::containing(Grain::Quarter, t.day).to_string(),
                    PeriodKey::containing(Grain::Year, t.day).to_string(),
                    t.amount.cents(),
                    t.payee,
                    t.memo,
                    cat,
                    t.stream,
                    dedupe,
                ],
            )?;
            if n == 0 {
                summary.skipped += 1;
            } else {
                summary.added += 1;
            }
        }
        tx.commit()?;
        Ok(summary)
    }

    pub fn get_txn(&self, id: i64) -> Result<Txn> {
        self.conn
            .query_row(
                "SELECT t.id, t.account_id, t.day, t.cents, t.payee, t.memo, c.name, t.stream
                 FROM txn t LEFT JOIN category c ON c.id=t.category_id WHERE t.id=?1",
                [id],
                |r| {
                    Ok(Txn {
                        id: r.get(0)?,
                        account: AccountId(r.get(1)?),
                        day: r.get(2)?,
                        amount: Money::from_cents(r.get(3)?),
                        payee: r.get(4)?,
                        memo: r.get(5)?,
                        category: r.get(6)?,
                        stream: r.get(7)?,
                    })
                },
            )
            .optional()?
            .ok_or_else(|| DbError::NotFound(format!("transaction {id}")))
    }

    /// Set (or clear, with `None`) a transaction's category.
    pub fn set_txn_category(&self, id: i64, category: Option<&str>) -> Result<()> {
        let cat = match category {
            Some(c) if !c.trim().is_empty() => Some(self.category_id(c.trim())?),
            _ => None,
        };
        let n = self.conn.execute(
            "UPDATE txn SET category_id=?1 WHERE id=?2",
            params![cat, id],
        )?;
        if n == 0 {
            return Err(DbError::NotFound(format!("transaction {id}")));
        }
        Ok(())
    }

    /// Transactions with no category yet, oldest first.
    pub fn uncategorized_txns(&self) -> Result<Vec<Txn>> {
        let mut st = self.conn.prepare(
            "SELECT t.id, t.account_id, t.day, t.cents, t.payee, t.memo, c.name, t.stream
             FROM txn t LEFT JOIN category c ON c.id=t.category_id
             WHERE t.category_id IS NULL ORDER BY t.day, t.id",
        )?;
        let rows = st.query_map([], |r| {
            Ok(Txn {
                id: r.get(0)?,
                account: AccountId(r.get(1)?),
                day: r.get(2)?,
                amount: Money::from_cents(r.get(3)?),
                payee: r.get(4)?,
                memo: r.get(5)?,
                category: r.get(6)?,
                stream: r.get(7)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    /// Names of every category in the ledger, sorted.
    pub fn category_names(&self) -> Result<Vec<String>> {
        let mut st = self
            .conn
            .prepare("SELECT name FROM category ORDER BY name")?;
        let rows = st.query_map([], |r| r.get::<_, String>(0))?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    pub fn txn_count(&self) -> Result<i64> {
        Ok(self
            .conn
            .query_row("SELECT count(*) FROM txn", [], |r| r.get(0))?)
    }

    pub fn balance(&self, account: AccountId) -> Result<Money> {
        let c: i64 = self.conn.query_row(
            "SELECT coalesce(sum(cents),0) FROM txn WHERE account_id=?1",
            [account.0],
            |r| r.get(0),
        )?;
        Ok(Money::from_cents(c))
    }

    pub fn txns_between(&self, from: NaiveDate, to: NaiveDate) -> Result<Vec<Txn>> {
        let mut st = self.conn.prepare(
            "SELECT t.id, t.account_id, t.day, t.cents, t.payee, t.memo, c.name, t.stream
             FROM txn t LEFT JOIN category c ON c.id=t.category_id
             WHERE t.day BETWEEN ?1 AND ?2 ORDER BY t.day, t.id",
        )?;
        let rows = st.query_map(params![from, to], |r| {
            Ok(Txn {
                id: r.get(0)?,
                account: AccountId(r.get(1)?),
                day: r.get(2)?,
                amount: Money::from_cents(r.get(3)?),
                payee: r.get(4)?,
                memo: r.get(5)?,
                category: r.get(6)?,
                stream: r.get(7)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    /// Income, spending and net per period of `grain` for periods overlapping [from, to].
    /// Periods with no activity are returned as zeros so charts have no gaps.
    pub fn totals(&self, from: NaiveDate, to: NaiveDate, grain: Grain) -> Result<Vec<PeriodTotal>> {
        let col = grain_col(grain);
        let sql = format!(
            "SELECT {col}, coalesce(sum(CASE WHEN cents>0 THEN cents END),0), coalesce(sum(CASE WHEN cents<0 THEN cents END),0), count(*)
             FROM txn WHERE day BETWEEN ?1 AND ?2 GROUP BY {col}"
        );
        let mut st = self.conn.prepare(&sql)?;
        let mut found: std::collections::HashMap<String, (i64, i64, i64)> = Default::default();
        let first = PeriodKey::containing(grain, from);
        let last = PeriodKey::containing(grain, to);
        // Query whole periods so partial first/last periods are complete.
        let rows = st.query_map(params![first.start, last.end()], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, i64>(3)?,
            ))
        })?;
        for r in rows {
            let (k, inc, sp, n) = r?;
            found.insert(k, (inc, sp, n));
        }
        Ok(PeriodKey::range(grain, from, to)
            .into_iter()
            .map(|p| {
                let k = p.to_string();
                let (inc, sp, n) = found.get(&k).copied().unwrap_or((0, 0, 0));
                PeriodTotal {
                    period: k,
                    income: Money::from_cents(inc),
                    spending: Money::from_cents(-sp),
                    net: Money::from_cents(inc + sp),
                    count: n,
                }
            })
            .collect())
    }

    /// Totals by category for one period.
    pub fn category_totals(
        &self,
        period: PeriodKey,
        metric: Metric,
    ) -> Result<Vec<(String, Money)>> {
        let col = grain_col(period.grain);
        let (cmp, sign) = match metric {
            Metric::Spending => ("<", -1),
            Metric::Income => (">", 1),
        };
        let sql = format!(
            "SELECT coalesce(c.name,'Uncategorized'), sum(t.cents) FROM txn t LEFT JOIN category c ON c.id=t.category_id
             WHERE t.{col}=?1 AND t.cents {cmp} 0 GROUP BY 1 ORDER BY sum(t.cents) * {sign} DESC, 1"
        );
        let mut st = self.conn.prepare(&sql)?;
        let rows = st.query_map([period.to_string()], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
        })?;
        let mut out = vec![];
        for r in rows {
            let (n, c) = r?;
            out.push((n, Money::from_cents(c * sign)));
        }
        Ok(out)
    }

    pub fn add_person(&self, name: &str, birth: NaiveDate, dependent: bool) -> Result<i64> {
        if name.trim().is_empty() {
            return Err(DbError::Invalid("name is empty".into()));
        }
        self.conn.execute(
            "INSERT INTO person(name, birth_date, dependent) VALUES (?1,?2,?3)",
            params![name, birth, dependent],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn people(&self) -> Result<Vec<fin_core::Person>> {
        let mut st = self
            .conn
            .prepare("SELECT id, name, birth_date, dependent FROM person ORDER BY id")?;
        let rows = st.query_map([], |r| {
            Ok(fin_core::Person {
                id: r.get::<_, i64>(0)?.to_string(),
                name: r.get(1)?,
                birth_date: r.get(2)?,
                is_dependent: r.get(3)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ImportSummary {
    pub added: usize,
    pub skipped: usize,
}
