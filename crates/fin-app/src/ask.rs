//! Ask about your money in plain words (AI-1). Every number in an answer comes from the ledger
//! through the same read-only functions the MCP server uses; the words around them are written
//! here, so an answer can never contain a number nobody computed. The questions understood now
//! are about spending and income (in total or by category), savings, net worth and balances, for
//! a week, month, quarter or year.
use crate::transactions::TxnRow;
use crate::App;
use chrono::{Datelike, NaiveDate};
use fin_core::{Grain, Money, PeriodKey};
use serde::Serialize;

const MAX_BEHIND: usize = 8;

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Fact {
    pub label: String,
    pub cents: i64,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Answer {
    pub understood: bool,
    /// The answer in a sentence or two.
    pub text: String,
    /// The numbers behind it.
    pub facts: Vec<Fact>,
    /// The biggest transactions the answer rests on (spending or income questions).
    pub behind: Vec<TxnRow>,
}

impl Answer {
    fn not_understood() -> Answer {
        Answer {
            understood: false,
            text: "I can answer questions like: \"How much did I spend on dining last month?\", \"What did I earn this year?\", \"How much did I save last quarter?\", \"What is my net worth?\" or \"What is the balance of my checking account?\"".into(),
            facts: vec![],
            behind: vec![],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Intent {
    Spending,
    Income,
    Saving,
    NetWorth,
    Balance,
}

/// The period a question is about, and how to say it.
pub fn parse_period(q: &str, today: NaiveDate) -> (PeriodKey, String) {
    let q = q.to_ascii_lowercase();
    let grains = [
        ("week", Grain::Week),
        ("month", Grain::Month),
        ("quarter", Grain::Quarter),
        ("year", Grain::Year),
    ];
    for (word, g) in grains {
        if q.contains(&format!("last {word}")) || q.contains(&format!("past {word}")) {
            return (
                PeriodKey::containing(g, today).prev(),
                format!("last {word}"),
            );
        }
        if q.contains(&format!("this {word}")) {
            return (PeriodKey::containing(g, today), format!("this {word}"));
        }
    }
    const MONTHS: [&str; 12] = [
        "january",
        "february",
        "march",
        "april",
        "may",
        "june",
        "july",
        "august",
        "september",
        "october",
        "november",
        "december",
    ];
    for (i, m) in MONTHS.iter().enumerate() {
        let hit = q
            .split(|c: char| !c.is_ascii_alphabetic())
            .any(|w| w == *m || (m.len() > 3 && w == &m[..3]));
        if hit {
            let month = i as u32 + 1;
            // The most recent such month that is not in the future.
            let year = if month > today.month() {
                today.year() - 1
            } else {
                today.year()
            };
            let d = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
            let label = format!("in {} {}", m_title(m), year);
            return (PeriodKey::containing(Grain::Month, d), label);
        }
    }
    (
        PeriodKey::containing(Grain::Month, today).prev(),
        "last month".into(),
    )
}

fn m_title(m: &str) -> String {
    let mut c = m.chars();
    c.next()
        .map(|f| f.to_ascii_uppercase().to_string() + c.as_str())
        .unwrap_or_default()
}

fn intent(q: &str) -> Option<Intent> {
    let q = q.to_ascii_lowercase();
    let has = |words: &[&str]| words.iter().any(|w| q.contains(w));
    if has(&["net worth", "worth"]) {
        Some(Intent::NetWorth)
    } else if has(&["balance"]) {
        Some(Intent::Balance)
    } else if has(&["save", "saved", "saving", "savings rate"]) {
        Some(Intent::Saving)
    } else if has(&["earn", "income", "made", "paid", "salary", "revenue"]) {
        Some(Intent::Income)
    } else if has(&["spend", "spent", "spending", "cost", "expens"]) {
        Some(Intent::Spending)
    } else {
        None
    }
}

impl App {
    fn period_totals(&self, p: &PeriodKey) -> Result<(i64, i64), String> {
        let grain = match p.grain {
            Grain::Week => "week",
            Grain::Month => "month",
            Grain::Quarter => "quarter",
            Grain::Year => "year",
        };
        let rows = self.cash_flow(p.start, p.end(), grain)?;
        Ok(rows.iter().fold((0, 0), |(i, s), r| {
            (i + r.income_cents, s + r.spending_cents)
        }))
    }

    /// The biggest transactions in a period, optionally only one category.
    fn behind(&self, p: &PeriodKey, category: Option<&str>, income: bool) -> Vec<TxnRow> {
        let from = p.start.format("%Y-%m-%d").to_string();
        let to = p.end().format("%Y-%m-%d").to_string();
        let mut rows = self.transactions(&from, &to, false).unwrap_or_default();
        rows.retain(|t| {
            (if income {
                t.amount_cents > 0
            } else {
                t.amount_cents < 0
            }) && category.is_none_or(|c| t.category.as_deref() == Some(c))
        });
        rows.sort_by_key(|t| -t.amount_cents.abs());
        rows.truncate(MAX_BEHIND);
        rows
    }

    pub fn ask(&self, question: &str) -> Result<Answer, String> {
        let today = (self.clock)();
        let q = question.trim();
        if q.is_empty() {
            return Ok(Answer::not_understood());
        }
        let Some(it) = intent(q) else {
            return Ok(Answer::not_understood());
        };
        let (period, label) = parse_period(q, today);
        let money = |c: i64| Money::from_cents(c).to_string();
        match it {
            Intent::NetWorth => {
                let b = self.balance_sheet()?;
                Ok(Answer {
                    understood: true,
                    text: format!(
                        "Your net worth is {}: {} you hold, less {} you owe.",
                        money(b.net_worth_cents),
                        money(b.assets_cents),
                        money(b.debts_cents.abs())
                    ),
                    facts: vec![
                        Fact {
                            label: "Net worth".into(),
                            cents: b.net_worth_cents,
                        },
                        Fact {
                            label: "Assets".into(),
                            cents: b.assets_cents,
                        },
                        Fact {
                            label: "Debts".into(),
                            cents: b.debts_cents.abs(),
                        },
                    ],
                    behind: vec![],
                })
            }
            Intent::Balance => {
                let lower = q.to_ascii_lowercase();
                let accounts = self.accounts()?;
                let hit = accounts
                    .iter()
                    .filter(|a| lower.contains(&a.name.to_ascii_lowercase()))
                    .max_by_key(|a| a.name.len());
                match hit {
                    Some(a) => Ok(Answer {
                        understood: true,
                        text: format!("{} holds {}.", a.name, money(a.balance_cents)),
                        facts: vec![Fact {
                            label: a.name.clone(),
                            cents: a.balance_cents,
                        }],
                        behind: vec![],
                    }),
                    None if accounts.is_empty() => Ok(Answer {
                        understood: true,
                        text: "There are no accounts yet. Import a statement first.".into(),
                        facts: vec![],
                        behind: vec![],
                    }),
                    None => Ok(Answer {
                        understood: true,
                        text: "Which account? You have: ".to_string()
                            + &accounts
                                .iter()
                                .map(|a| a.name.clone())
                                .collect::<Vec<_>>()
                                .join(", ")
                            + ".",
                        facts: accounts
                            .iter()
                            .map(|a| Fact {
                                label: a.name.clone(),
                                cents: a.balance_cents,
                            })
                            .collect(),
                        behind: vec![],
                    }),
                }
            }
            Intent::Saving => {
                let (inc, spend) = self.period_totals(&period)?;
                let net = inc - spend.abs();
                let rate = if inc > 0 {
                    format!(
                        ", {}% of what you earned",
                        (net as f64 / inc as f64 * 100.0).round() as i64
                    )
                } else {
                    String::new()
                };
                Ok(Answer {
                    understood: true,
                    text: format!(
                        "{}: you earned {} and spent {}, so you {} {}{}.",
                        cap(&label),
                        money(inc),
                        money(spend.abs()),
                        if net >= 0 { "saved" } else { "overspent by" },
                        money(net.abs()),
                        if net >= 0 { rate } else { String::new() }
                    ),
                    facts: vec![
                        Fact {
                            label: "Earned".into(),
                            cents: inc,
                        },
                        Fact {
                            label: "Spent".into(),
                            cents: spend.abs(),
                        },
                        Fact {
                            label: "Saved".into(),
                            cents: net,
                        },
                    ],
                    behind: vec![],
                })
            }
            Intent::Income | Intent::Spending => {
                let income = it == Intent::Income;
                let grain = match period.grain {
                    Grain::Week => "week",
                    Grain::Month => "month",
                    Grain::Quarter => "quarter",
                    Grain::Year => "year",
                };
                let rows = self.categories(period.start, grain, income)?;
                // A category the question names (longest name wins).
                let lower = q.to_ascii_lowercase();
                let names = self.category_choices()?;
                let cat = names
                    .iter()
                    .filter(|n| lower.contains(&n.to_ascii_lowercase()))
                    .max_by_key(|n| n.len())
                    .cloned();
                let verb = if income { "earned" } else { "spent" };
                if let Some(cat) = cat {
                    let cents = rows
                        .iter()
                        .find(|r| r.category == cat)
                        .map(|r| r.cents.abs())
                        .unwrap_or(0);
                    return Ok(Answer {
                        understood: true,
                        text: if cents == 0 {
                            format!(
                                "Nothing {} on {} {}.",
                                if income { "came in" } else { "was spent" },
                                cat,
                                label
                            )
                        } else {
                            format!("You {verb} {} on {} {}.", money(cents), cat, label)
                        },
                        facts: vec![Fact {
                            label: cat.clone(),
                            cents,
                        }],
                        behind: self.behind(&period, Some(&cat), income),
                    });
                }
                let (inc, spend) = self.period_totals(&period)?;
                let total = if income { inc } else { spend.abs() };
                let mut top: Vec<_> = rows
                    .iter()
                    .map(|r| (r.category.clone(), r.cents.abs()))
                    .collect();
                top.sort_by_key(|(_, c)| -c);
                top.truncate(3);
                let detail = if top.is_empty() {
                    String::new()
                } else {
                    format!(
                        " The biggest: {}.",
                        top.iter()
                            .map(|(n, c)| format!("{n} {}", money(*c)))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                };
                let mut facts = vec![Fact {
                    label: if income { "Earned" } else { "Spent" }.into(),
                    cents: total,
                }];
                facts.extend(top.into_iter().map(|(label, cents)| Fact { label, cents }));
                Ok(Answer {
                    understood: true,
                    text: if total == 0 {
                        format!("There is nothing recorded {label}.")
                    } else {
                        format!("{} you {verb} {}.{detail}", cap(&label), money(total))
                    },
                    facts,
                    behind: self.behind(&period, None, income),
                })
            }
        }
    }
}

fn cap(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_ascii_uppercase().to_string() + c.as_str())
        .unwrap_or_default()
}
