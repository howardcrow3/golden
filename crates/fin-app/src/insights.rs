//! Plain-language observations about the most recent month, from numbers Golden already has.
//!
//! Each observation is a fixed rule with a threshold, so it is always explainable ("Dining is
//! up because it was $310 against $190 the month before") and never invented.
use crate::{App, PeriodRow};
use chrono::{Datelike, NaiveDate};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Insight {
    /// savings, overspent, category_up, needs_category
    pub kind: String,
    pub text: String,
}

const MIN_JUMP_CENTS: i64 = 5_000;

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

fn month_name(d: NaiveDate) -> &'static str {
    [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ][d.month0() as usize]
}

fn add_months(d: NaiveDate, n: i32) -> NaiveDate {
    let idx = d.year() * 12 + d.month0() as i32 + n;
    NaiveDate::from_ymd_opt(idx.div_euclid(12), idx.rem_euclid(12) as u32 + 1, 1).expect("valid")
}

impl App {
    /// Observations about the last full month (or this month when it is the only one with data).
    pub fn insights(&self) -> Result<Vec<Insight>, String> {
        let today = (self.clock)();
        let this_month = today.with_day(1).expect("day 1");
        let last = add_months(this_month, -1);
        let flow = |m: NaiveDate| -> Result<Option<PeriodRow>, String> {
            Ok(self
                .cash_flow(m, add_months(m, 1).pred_opt().expect("day"), "month")?
                .into_iter()
                .find(|r| r.count > 0))
        };
        let (month, row) = match flow(last)? {
            Some(r) => (last, r),
            None => match flow(this_month)? {
                Some(r) => (this_month, r),
                None => return Ok(vec![]),
            },
        };
        let name = month_name(month);
        let mut out = vec![];

        if row.income_cents > 0 {
            if row.net_cents >= 0 {
                let pct = row.net_cents * 100 / row.income_cents;
                out.push(Insight {
                    kind: "savings".into(),
                    text: format!(
                        "In {name} you kept {} of the {} that came in ({pct}%).",
                        dollars(row.net_cents),
                        dollars(row.income_cents)
                    ),
                });
            } else {
                out.push(Insight {
                    kind: "savings".into(),
                    text: format!(
                        "In {name} you spent {} more than came in.",
                        dollars(-row.net_cents)
                    ),
                });
            }
        } else if row.spending_cents > 0 {
            out.push(Insight {
                kind: "savings".into(),
                text: format!(
                    "No income was recorded in {name}; spending was {}.",
                    dollars(row.spending_cents)
                ),
            });
        }

        let day = month.format("%Y-%m-%d").to_string();
        let prev_day = add_months(month, -1).format("%Y-%m-%d").to_string();
        let now: HashMap<String, i64> = self
            .categories_str(&day, "month", false)?
            .into_iter()
            .map(|c| (c.category, c.cents))
            .collect();
        let before: HashMap<String, i64> = self
            .categories_str(&prev_day, "month", false)?
            .into_iter()
            .map(|c| (c.category, c.cents))
            .collect();
        let prev_name = month_name(add_months(month, -1));
        let mut ups: Vec<(&String, i64, i64)> = now
            .iter()
            .filter_map(|(c, v)| {
                let b = *before.get(c)?;
                (b > 0 && *v - b >= MIN_JUMP_CENTS && (*v - b) * 4 >= b).then_some((c, *v, b))
            })
            .collect();
        ups.sort_by(|a, b| (b.1 - b.2).cmp(&(a.1 - a.2)).then(a.0.cmp(b.0)));
        if let Some((c, v, b)) = ups.first() {
            out.push(Insight {
                kind: "category_up".into(),
                text: format!(
                    "{c} was {} in {name}, up {} from {prev_name}'s {}.",
                    dollars(*v),
                    dollars(v - b),
                    dollars(*b)
                ),
            });
        }

        for b in self.budget_status(&day, "month")? {
            if b.remaining_cents < 0 && b.category != "Total" {
                out.push(Insight {
                    kind: "overspent".into(),
                    text: format!(
                        "{} went {} over its {} budget in {name}.",
                        b.category,
                        dollars(-b.remaining_cents),
                        dollars(b.budget_cents)
                    ),
                });
            }
        }

        let last_day = add_months(month, 1)
            .pred_opt()
            .expect("day")
            .format("%Y-%m-%d")
            .to_string();
        let todo = self.transactions(&day, &last_day, true)?.len();
        if todo > 0 {
            out.push(Insight {
                kind: "needs_category".into(),
                text: format!(
                    "{todo} transaction{} in {name} still need{} a category.",
                    if todo == 1 { "" } else { "s" },
                    if todo == 1 { "s" } else { "" }
                ),
            });
        }
        Ok(out)
    }
}
