//! Month close (BK-5): check a finished month, write a short summary, then lock it so later
//! imports and recategorizing can't quietly change numbers you have signed off.
use crate::{App, PeriodRow};
use chrono::{Datelike, NaiveDate};
use serde::Serialize;
use std::collections::BTreeMap;

const KEY: &str = "closed_months";
/// A line is an anomaly when it is at least this large and several times the account's norm.
const MIN_ANOMALY_CENTS: i64 = 20_000;
const ANOMALY_FACTOR: i64 = 4;

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CloseCheck {
    /// uncategorized, quiet_account, unusual_amount
    pub kind: String,
    pub text: String,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CloseReport {
    /// "YYYY-MM"
    pub month: String,
    pub closed: bool,
    /// Past its last day, so it can be closed.
    pub finished: bool,
    pub income_cents: i64,
    pub spending_cents: i64,
    pub net_cents: i64,
    pub count: i64,
    pub issues: Vec<CloseCheck>,
    pub summary: String,
}

fn parse_month(m: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(&format!("{}-01", m.trim()), "%Y-%m-%d")
        .map_err(|_| "Use a month like 2026-10.".to_string())
}

fn key(d: NaiveDate) -> String {
    d.format("%Y-%m").to_string()
}

fn next_month(d: NaiveDate) -> NaiveDate {
    let idx = d.year() * 12 + d.month0() as i32 + 1;
    NaiveDate::from_ymd_opt(idx.div_euclid(12), idx.rem_euclid(12) as u32 + 1, 1).expect("valid")
}

fn prev_month(d: NaiveDate) -> NaiveDate {
    d.pred_opt().expect("day").with_day(1).expect("day 1")
}

fn dollars(cents: i64) -> String {
    let n = cents.abs() / 100;
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    format!("${out}")
}

impl App {
    fn closed_list(&self) -> Result<Vec<String>, String> {
        match self.db.get_setting(KEY).map_err(|e| e.to_string())? {
            Some(j) => serde_json::from_str(&j).map_err(|e| e.to_string()),
            None => Ok(vec![]),
        }
    }

    /// Months that have been closed, oldest first.
    pub fn closed_months(&self) -> Result<Vec<String>, String> {
        let mut v = self.closed_list()?;
        v.sort();
        Ok(v)
    }

    pub(crate) fn is_month_closed(&self, day: NaiveDate) -> Result<bool, String> {
        Ok(self.closed_list()?.contains(&key(day)))
    }

    /// Refuse a change that would touch a closed month.
    pub(crate) fn ensure_open(&self, day: NaiveDate) -> Result<(), String> {
        if self.is_month_closed(day)? {
            Err(format!(
                "{} is closed. Reopen it on the Close page to change it.",
                day.format("%B %Y")
            ))
        } else {
            Ok(())
        }
    }

    /// What a close of `month` ("YYYY-MM") would find. Changes nothing.
    pub fn month_close_check(&self, month: &str) -> Result<CloseReport, String> {
        let first = parse_month(month)?;
        let last = next_month(first).pred_opt().expect("day");
        let rows = self
            .db
            .txns_between(first, last)
            .map_err(|e| e.to_string())?;
        let flow: PeriodRow = self
            .cash_flow(first, last, "month")?
            .into_iter()
            .next()
            .unwrap_or(PeriodRow {
                period: key(first),
                income_cents: 0,
                spending_cents: 0,
                net_cents: 0,
                count: 0,
            });
        let names: BTreeMap<i64, String> = self
            .db
            .accounts()
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|(id, n, _, _)| (id.0, n))
            .collect();
        let mut issues = vec![];

        let uncategorized = rows.iter().filter(|t| t.category.is_none()).count();
        if uncategorized > 0 {
            issues.push(CloseCheck {
                kind: "uncategorized".into(),
                text: format!(
                    "{uncategorized} transaction{} still need{} a category.",
                    if uncategorized == 1 { "" } else { "s" },
                    if uncategorized == 1 { "s" } else { "" }
                ),
            });
        }

        let prev = self
            .db
            .txns_between(prev_month(first), first.pred_opt().expect("day"))
            .map_err(|e| e.to_string())?;
        for (id, name) in &names {
            let before = prev.iter().any(|t| t.account.0 == *id);
            let now = rows.iter().any(|t| t.account.0 == *id);
            if before && !now {
                issues.push(CloseCheck {
                    kind: "quiet_account".into(),
                    text: format!("{name} had activity last month but none in this one. Is a statement missing?"),
                });
            }
        }

        for (id, name) in &names {
            let mut amounts: Vec<i64> = rows
                .iter()
                .filter(|t| t.account.0 == *id)
                .map(|t| t.amount.cents().abs())
                .collect();
            if amounts.len() < 5 {
                continue;
            }
            amounts.sort_unstable();
            let median = amounts[amounts.len() / 2];
            for t in rows.iter().filter(|t| t.account.0 == *id) {
                let a = t.amount.cents().abs();
                if a >= MIN_ANOMALY_CENTS && a >= median * ANOMALY_FACTOR {
                    issues.push(CloseCheck {
                        kind: "unusual_amount".into(),
                        text: format!(
                            "{} on {} in {name} is much larger than usual ({}).",
                            t.payee,
                            t.day.format("%b %-d"),
                            dollars(a)
                        ),
                    });
                }
            }
        }

        let label = first.format("%B %Y");
        let summary = if flow.count == 0 {
            format!("Nothing was recorded for {label}.")
        } else if flow.net_cents >= 0 {
            format!(
                "{label}: {} came in, {} went out, and you kept {}.",
                dollars(flow.income_cents),
                dollars(flow.spending_cents),
                dollars(flow.net_cents)
            )
        } else {
            format!(
                "{label}: {} came in and {} went out, so you spent {} more than you earned.",
                dollars(flow.income_cents),
                dollars(flow.spending_cents),
                dollars(-flow.net_cents)
            )
        };
        Ok(CloseReport {
            month: key(first),
            closed: self.is_month_closed(first)?,
            finished: (self.clock)() > last,
            income_cents: flow.income_cents,
            spending_cents: flow.spending_cents,
            net_cents: flow.net_cents,
            count: flow.count,
            issues,
            summary,
        })
    }

    /// Lock a finished month. Open issues are reported but do not stop the close.
    pub fn close_month(&self, month: &str) -> Result<CloseReport, String> {
        self.require_edit()?;
        let report = self.month_close_check(month)?;
        if !report.finished {
            return Err("That month isn't over yet.".into());
        }
        let mut list = self.closed_list()?;
        if !list.contains(&report.month) {
            list.push(report.month.clone());
            self.db
                .set_setting(
                    KEY,
                    &serde_json::to_string(&list).map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())?;
        }
        self.month_close_check(month)
    }

    pub fn reopen_month(&self, month: &str) -> Result<CloseReport, String> {
        self.require_edit()?;
        let m = key(parse_month(month)?);
        let mut list = self.closed_list()?;
        list.retain(|x| *x != m);
        self.db
            .set_setting(
                KEY,
                &serde_json::to_string(&list).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
        self.month_close_check(month)
    }
}
