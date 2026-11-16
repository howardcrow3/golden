//! Recurring payments and paychecks, found from the ledger's own history.
//!
//! A payee counts as recurring when it shows up at least three times on a steady rhythm
//! (weekly, every two weeks, monthly, quarterly or yearly) with amounts that stay in the same
//! neighborhood. Groceries and restaurants don't qualify: their gaps are irregular. The
//! point is the list of things that will happen again, with what they cost per year.
use crate::{parse_day, App};
use chrono::Duration;
use fin_core::NaiveDate;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Recurring {
    pub payee: String,
    pub cadence: String,
    /// Typical amount in cents; negative for money out.
    pub typical_cents: i64,
    pub times_seen: usize,
    pub last_seen: String,
    pub next_expected: String,
    /// What it adds up to over a year, same sign as `typical_cents`.
    pub yearly_cents: i64,
}

type Seen = (NaiveDate, i64, String);

const CADENCES: [(&str, i64, i64); 5] = [
    ("weekly", 7, 52),
    ("every two weeks", 14, 26),
    ("monthly", 30, 12),
    ("quarterly", 91, 4),
    ("yearly", 365, 1),
];

/// "NETFLIX.COM 8005551212" and "Netflix.com 800-555" group together; store numbers are dropped.
fn key_of(payee: &str) -> String {
    payee
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty() && !w.chars().any(|c| c.is_ascii_digit()))
        .take(3)
        .collect::<Vec<_>>()
        .join(" ")
}

fn median(v: &mut [i64]) -> i64 {
    v.sort_unstable();
    v[v.len() / 2]
}

fn classify(days: &[(NaiveDate, i64)]) -> Option<(&'static str, i64)> {
    let mut gaps: Vec<i64> = days
        .windows(2)
        .map(|w| (w[1].0 - w[0].0).num_days())
        .collect();
    let m = median(&mut gaps.clone());
    let (name, nominal, per_year) = CADENCES.iter().min_by_key(|(_, n, _)| (n - m).abs())?;
    let tol = (nominal / 8).max(3);
    if (nominal - m).abs() > tol {
        return None;
    }
    let steady = gaps.iter().filter(|g| (**g - m).abs() <= tol).count();
    gaps.sort_unstable();
    // At least three in four gaps must match the rhythm (one skipped month is forgiven).
    (steady * 4 >= gaps.len() * 3).then_some((*name, *per_year))
}

impl App {
    /// Recurring payments and income within the window, biggest yearly amount first.
    pub fn recurring(&self, from: &str, to: &str) -> Result<Vec<Recurring>, String> {
        let rows = self
            .db
            .txns_between(parse_day(from)?, parse_day(to)?)
            .map_err(|e| e.to_string())?;
        let mut groups: BTreeMap<(String, bool), Vec<Seen>> = BTreeMap::new();
        for t in rows {
            let k = key_of(&t.payee);
            if k.is_empty() || t.amount.cents() == 0 {
                continue;
            }
            groups.entry((k, t.amount.cents() > 0)).or_default().push((
                t.day,
                t.amount.cents(),
                t.payee,
            ));
        }
        let mut out = vec![];
        for (_, mut g) in groups {
            if g.len() < 3 {
                continue;
            }
            g.sort_by_key(|x| x.0);
            let days: Vec<(NaiveDate, i64)> = g.iter().map(|x| (x.0, x.1)).collect();
            let Some((cadence, per_year)) = classify(&days) else {
                continue;
            };
            let mut amounts: Vec<i64> = g.iter().map(|x| x.1).collect();
            let typical = median(&mut amounts);
            // Amounts must stay within 30% of the typical one (utilities drift; a one-off doesn't).
            let close = g
                .iter()
                .filter(|x| (x.1 - typical).abs() * 10 <= typical.abs() * 3)
                .count();
            if close * 4 < g.len() * 3 {
                continue;
            }
            let last = g.last().unwrap();
            let step = CADENCES.iter().find(|c| c.0 == cadence).unwrap().1;
            out.push(Recurring {
                payee: last.2.clone(),
                cadence: cadence.into(),
                typical_cents: typical,
                times_seen: g.len(),
                last_seen: last.0.format("%Y-%m-%d").to_string(),
                next_expected: (last.0 + Duration::days(step))
                    .format("%Y-%m-%d")
                    .to_string(),
                yearly_cents: typical * per_year,
            });
        }
        out.sort_by(|a, b| {
            b.yearly_cents
                .abs()
                .cmp(&a.yearly_cents.abs())
                .then(a.payee.cmp(&b.payee))
        });
        Ok(out)
    }
}
