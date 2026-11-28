//! What-if chat. A person types a sentence; an `Interpreter` turns it into concrete changes;
//! Golden runs the plan with and without them and shows both. Nothing is saved until the
//! person keeps the change. The built-in `RuleInterpreter` understands common phrasings with
//! no network or model; a language model can be plugged in behind the same trait later.
use crate::household::{EventRow, Household, StreamRow};
use crate::App;
use chrono::{Datelike, NaiveDate};
use fin_plan::quickplan::{self, QuickPlanResult};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Change {
    #[serde(rename_all = "camelCase")]
    SetPick { key: String, label: String },
    #[serde(rename_all = "camelCase")]
    RetireAt { person: usize, age: i32 },
    #[serde(rename_all = "camelCase")]
    Spending { cents: i64 },
    #[serde(rename_all = "camelCase")]
    MoveTo { state: String },
    #[serde(rename_all = "camelCase")]
    OneTime {
        label: String,
        person: usize,
        cents: i64,
        year: i32,
    },
    /// A lasting raise: this person's regular income streams grow by `bp` basis points
    /// (100 bp = 1%) in real terms. One-time amounts are left alone.
    #[serde(rename_all = "camelCase")]
    Raise { person: usize, bp: i32 },
    /// A planned one-off cost (positive) or untaxed inflow (negative) in today's dollars.
    #[serde(rename_all = "camelCase")]
    PlannedCost {
        label: String,
        cents: i64,
        year: i32,
    },
    /// Set what is in one kind of account: "cash", "taxable", "traditional" or "roth".
    #[serde(rename_all = "camelCase")]
    SetBalance { account: String, cents: i64 },
    /// How retirement spending is set: "fixed", "guardrails", "percent" or "vpw".
    #[serde(rename_all = "camelCase")]
    SpendingRule { rule: String },
    /// Which accounts are drawn first: "standard", "traditional_first", "roth_first" or "blended".
    #[serde(rename_all = "camelCase")]
    WithdrawalOrder { order: String },
}

pub const ACCOUNTS: [&str; 4] = ["cash", "taxable", "traditional", "roth"];
pub const SPENDING_RULES: [&str; 4] = ["fixed", "guardrails", "percent", "vpw"];
pub const WITHDRAWAL_ORDERS: [&str; 4] = ["standard", "traditional_first", "roth_first", "blended"];

fn spending_rule_words(rule: &str) -> &str {
    match rule {
        "fixed" => "the same spending every year",
        "guardrails" => "guardrails spending",
        "percent" => "spending 4% of the portfolio",
        "vpw" => "variable spending that spends down evenly",
        other => other,
    }
}

fn order_words(order: &str) -> &str {
    match order {
        "standard" => "cash, taxable, traditional, then Roth",
        "traditional_first" => "cash, traditional, taxable, then Roth",
        "roth_first" => "cash, Roth, taxable, then traditional",
        "blended" => "all accounts in proportion",
        other => other,
    }
}

impl Change {
    pub fn describe(&self, h: &Household) -> String {
        match self {
            Change::SetPick { key, label } => format!("Assume {}: {label}", pick_name(key)),
            Change::RetireAt { person, age } => {
                let name = h.people.get(*person).map_or("Someone", |p| p.name.as_str());
                let verb = if name == "You" { "retire" } else { "retires" };
                format!("{name} {verb} at {age}")
            }
            Change::Spending { cents } => {
                format!("Spend ${} a year", group(*cents / 100))
            }
            Change::MoveTo { state } => format!("Live in {state}"),
            Change::OneTime {
                label, cents, year, ..
            } => format!("{label} of ${} in {year}", group(*cents / 100)),
            Change::Raise { person, bp } => format!(
                "{} get{} a {}% raise",
                h.people.get(*person).map_or("Someone", |p| p.name.as_str()),
                if h.people.get(*person).is_some_and(|p| p.name == "You") {
                    ""
                } else {
                    "s"
                },
                *bp as f64 / 100.0
            ),
            Change::SetBalance { account, cents } => {
                format!("Have ${} in {account}", group(*cents / 100))
            }
            Change::SpendingRule { rule } => format!("Use {}", spending_rule_words(rule)),
            Change::WithdrawalOrder { order } => {
                format!("Draw accounts in this order: {}", order_words(order))
            }
            Change::PlannedCost {
                label, cents, year, ..
            } if *cents < 0 => format!("{label}: ${} comes in during {year}", group(-*cents / 100)),
            Change::PlannedCost {
                label, cents, year, ..
            } => format!("{label}: ${} goes out in {year}", group(*cents / 100)),
        }
    }

    pub fn apply(&self, h: &mut Household) -> Result<(), String> {
        match self {
            Change::SetPick { key, label } => {
                h.picks.insert(key.clone(), label.clone());
            }
            Change::RetireAt { person, age } => {
                h.people
                    .get_mut(*person)
                    .ok_or("No such person.")?
                    .retire_age = *age;
            }
            Change::Spending { cents } => h.spending_cents = *cents,
            Change::MoveTo { state } => h.state = state.clone(),
            Change::OneTime {
                label,
                person,
                cents,
                year,
            } => h.streams.push(StreamRow {
                label: label.clone(),
                person: *person,
                annual_cents: *cents,
                self_employed: false,
                kind: None,
                employer_match_cents: 0,
                hsa_cents: 0,
                start_year: Some(*year),
                end_year: Some(*year),
                contribution_cents: 0,
            }),
            Change::Raise { person, bp } => {
                if *person >= h.people.len() {
                    return Err("No such person.".into());
                }
                for s in h.streams.iter_mut().filter(|s| {
                    s.person == *person && !(s.start_year.is_some() && s.start_year == s.end_year)
                }) {
                    s.annual_cents = s.annual_cents * (10_000 + *bp as i64) / 10_000;
                    s.contribution_cents = s.contribution_cents * (10_000 + *bp as i64) / 10_000;
                }
            }
            Change::SetBalance { account, cents } => match account.as_str() {
                "cash" => h.cash_cents = *cents,
                "taxable" => h.taxable_cents = *cents,
                "traditional" => h.traditional_cents = *cents,
                "roth" => h.roth_cents = *cents,
                _ => return Err("Choose cash, taxable, traditional or roth.".into()),
            },
            Change::SpendingRule { rule } => {
                if !SPENDING_RULES.contains(&rule.as_str()) {
                    return Err("Choose a spending rule from the list.".into());
                }
                h.spending_rule = rule.clone();
            }
            Change::WithdrawalOrder { order } => {
                if !WITHDRAWAL_ORDERS.contains(&order.as_str()) {
                    return Err("Choose a withdrawal order from the list.".into());
                }
                h.withdrawal_order = order.clone();
            }
            Change::PlannedCost { label, cents, year } => h.events.push(EventRow {
                label: label.clone(),
                year: *year,
                cents: *cents,
            }),
        }
        Ok(())
    }
}

fn pick_name(key: &str) -> &str {
    match key {
        "real_return" => "investment return",
        "return_volatility" => "market ups and downs",
        "horizon_age" => "plan until",
        "ss_claim_age" => "Social Security claim age",
        "spending_change" => "retirement spending",
        other => other,
    }
}

fn group(n: i64) -> String {
    let s = n.abs().to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    if n < 0 {
        format!("-{out}")
    } else {
        out
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Interpretation {
    pub changes: Vec<Change>,
}

pub trait Interpreter: Send {
    fn interpret(&self, text: &str, h: &Household, today: NaiveDate) -> Interpretation;
}

/// Deterministic phrase understanding: no model, no network.
pub struct RuleInterpreter;

pub(crate) const STATES: &[(&str, &str)] = &[
    ("alabama", "AL"),
    ("alaska", "AK"),
    ("arizona", "AZ"),
    ("arkansas", "AR"),
    ("california", "CA"),
    ("colorado", "CO"),
    ("connecticut", "CT"),
    ("delaware", "DE"),
    ("florida", "FL"),
    ("georgia", "GA"),
    ("hawaii", "HI"),
    ("idaho", "ID"),
    ("illinois", "IL"),
    ("indiana", "IN"),
    ("iowa", "IA"),
    ("kansas", "KS"),
    ("kentucky", "KY"),
    ("louisiana", "LA"),
    ("maine", "ME"),
    ("maryland", "MD"),
    ("massachusetts", "MA"),
    ("michigan", "MI"),
    ("minnesota", "MN"),
    ("mississippi", "MS"),
    ("missouri", "MO"),
    ("montana", "MT"),
    ("nebraska", "NE"),
    ("nevada", "NV"),
    ("new hampshire", "NH"),
    ("new jersey", "NJ"),
    ("new mexico", "NM"),
    ("new york", "NY"),
    ("north carolina", "NC"),
    ("north dakota", "ND"),
    ("ohio", "OH"),
    ("oklahoma", "OK"),
    ("oregon", "OR"),
    ("pennsylvania", "PA"),
    ("rhode island", "RI"),
    ("south carolina", "SC"),
    ("south dakota", "SD"),
    ("tennessee", "TN"),
    ("texas", "TX"),
    ("utah", "UT"),
    ("vermont", "VT"),
    ("virginia", "VA"),
    ("washington", "WA"),
    ("west virginia", "WV"),
    ("wisconsin", "WI"),
    ("wyoming", "WY"),
];

/// "80k", "$80,000", "1.2m", "80000" -> cents.
pub fn parse_amount(token: &str) -> Option<i64> {
    let t = token
        .trim()
        .trim_start_matches('$')
        .replace(',', "")
        .to_lowercase();
    let (num, mult) = if let Some(n) = t.strip_suffix('k') {
        (n.to_string(), 1_000.0)
    } else if let Some(n) = t.strip_suffix('m') {
        (n.to_string(), 1_000_000.0)
    } else {
        (t, 1.0)
    };
    let v: f64 = num.parse().ok()?;
    // Anything over a trillion dollars is a typo (or "1e30"); refusing it keeps the plan's
    // arithmetic far from overflow.
    let dollars = v * mult;
    if !v.is_finite() || v < 0.0 || dollars > 1e12 {
        return None;
    }
    Some((dollars * 100.0).round() as i64)
}

/// First amount-looking token after `marker` in `text`.
fn amount_after(text: &str, marker: &str) -> Option<i64> {
    let rest = &text[text.find(marker)? + marker.len()..];
    rest.split_whitespace()
        .take(4)
        .map(|w| w.trim_matches(|c: char| ",.;:!?()".contains(c) && c != '$'))
        .find_map(|w| {
            if w.chars().any(|c| c.is_ascii_digit()) {
                parse_amount(w)
            } else {
                None
            }
        })
}

/// Amounts written with a dollar sign or a k/m suffix ("$400k", "30k", "$1,200"), in order.
fn money_tokens(text: &str) -> Vec<i64> {
    text.split_whitespace()
        .map(|w| w.trim_matches(|c: char| ",.;:!?()".contains(c) && c != '$'))
        .filter(|w| {
            w.starts_with('$')
                || (w.len() > 1
                    && (w.ends_with('k') || w.ends_with('m'))
                    && w[..w.len() - 1]
                        .chars()
                        .all(|c| c.is_ascii_digit() || c == '.' || c == ','))
        })
        .filter_map(parse_amount)
        .collect()
}

/// Planned one-off costs and untaxed inflows: "buy a house for $400k in 2030", "college $30k
/// a year from 2036 to 2040", "an inheritance of $200k in 2032", "sell the house in 2035 for
/// $500k".
fn planned_costs(t: &str, today: NaiveDate) -> Vec<Change> {
    // "2036-2040" is a span too, so a dash separates years like a space does.
    let years: Vec<i32> = t
        .split(|c: char| c.is_whitespace() || c == '-' || c == '\u{2013}')
        .filter_map(|w| {
            w.trim_matches(|c: char| !c.is_ascii_digit())
                .parse::<i32>()
                .ok()
        })
        .filter(|y| (today.year()..=today.year() + 60).contains(y))
        .collect();
    let Some(&first_year) = years.first() else {
        return vec![];
    };
    let Some(&cents) = money_tokens(t).first() else {
        return vec![];
    };
    if t.contains("bonus") || t.contains("vest") {
        return vec![];
    }
    let periodic = [
        "a year",
        "per year",
        "each year",
        "every year",
        "annually",
        "/year",
    ]
    .iter()
    .any(|p| t.contains(p));
    let monthly = t.contains("a month") || t.contains("per month") || t.contains("monthly");
    let inflow = [
        ("inherit", "Inheritance"),
        ("sell the house", "Home sale"),
        ("sell our house", "Home sale"),
        ("sell the home", "Home sale"),
        ("sell our home", "Home sale"),
        ("sale of the house", "Home sale"),
        ("sale of our house", "Home sale"),
    ]
    .iter()
    .find(|(w, _)| t.contains(w))
    .map(|(_, l)| l.to_string());
    let cost = [
        "house",
        "home",
        "car",
        "wedding",
        "college",
        "tuition",
        "boat",
        "vacation",
        "renovation",
        "remodel",
        "expense",
        "cost",
        "purchase",
        "buy",
        "down payment",
        "medical",
        "roof",
    ]
    .iter()
    .find(|w| t.contains(*w))
    .map(|w| {
        let mut l = if *w == "buy" || *w == "purchase" || *w == "cost" {
            "Expense".to_string()
        } else {
            w.to_string()
        };
        l[..1].make_ascii_uppercase();
        l
    });
    let one_off_spend = t.contains("spend ") && years.len() == 1 && !periodic && !monthly;
    let (label, sign) = match (inflow, cost) {
        (Some(l), _) => (l, -1),
        (None, Some(l)) => (l, 1),
        (None, None) if one_off_spend => ("Expense".to_string(), 1),
        _ => return vec![],
    };
    if monthly {
        return vec![];
    }
    // A span of years: "from 2036 to 2040", "2036-2040" or "for 4 years".
    let span: Vec<i32> = if years.len() >= 2 && years[1] > years[0] {
        (years[0]..=years[1]).take(40).collect()
    } else if let Some(n) = number_after(t, "for ").filter(|n| (2..=40).contains(n)) {
        if t.contains("years") {
            (first_year..first_year + n).collect()
        } else {
            vec![first_year]
        }
    } else {
        vec![first_year]
    };
    let per_year = if span.len() > 1 && !periodic {
        cents / span.len() as i64 // a total spread across the years
    } else {
        cents
    };
    span.into_iter()
        .map(|year| Change::PlannedCost {
            label: label.clone(),
            cents: sign * per_year,
            year,
        })
        .collect()
}

/// "a 5% raise", "raise of 3.5%", "give me a 10 percent raise" as basis points.
fn raise_bp(t: &str) -> Option<i32> {
    if !t.contains("raise") {
        return None;
    }
    let words: Vec<&str> = t.split_whitespace().collect();
    for (i, w) in words.iter().enumerate() {
        let w = w.trim_matches(|c: char| ",.;:!?()".contains(c));
        let pct = if let Some(n) = w.strip_suffix('%') {
            n.parse::<f64>().ok()
        } else if words.get(i + 1).is_some_and(|n| n.starts_with("percent")) {
            w.parse::<f64>().ok()
        } else {
            None
        };
        if let Some(p) = pct.filter(|p| *p > 0.0 && *p <= 100.0) {
            return Some((p * 100.0).round() as i32);
        }
    }
    None
}

fn number_after(text: &str, marker: &str) -> Option<i32> {
    let rest = &text[text.find(marker)? + marker.len()..];
    rest.split_whitespace().take(3).find_map(|w| {
        w.trim_matches(|c: char| !c.is_ascii_digit())
            .parse::<i32>()
            .ok()
    })
}

impl Interpreter for RuleInterpreter {
    fn interpret(&self, text: &str, h: &Household, today: NaiveDate) -> Interpretation {
        let t = text.to_lowercase();
        let mut out = vec![];
        let mut pick = |key: &str, label: &str| {
            out.push(Change::SetPick {
                key: key.into(),
                label: label.into(),
            })
        };

        // Who is being talked about (default: the first person).
        let person = h
            .people
            .iter()
            .position(|p| !p.name.trim().is_empty() && t.contains(&p.name.trim().to_lowercase()))
            .unwrap_or(0);

        let mut changes: Vec<Change> = vec![];

        for marker in [
            "retire at",
            "retires at",
            "retiring at",
            "retire by",
            "retires by",
            "stop working at",
            "quit at",
        ] {
            if let Some(age) = number_after(&t, marker).filter(|a| (30..=100).contains(a)) {
                changes.push(Change::RetireAt { person, age });
                break;
            }
        }

        if let Some(bp) = raise_bp(&t) {
            changes.push(Change::Raise { person, bp });
        }

        let planned = planned_costs(&t, today);
        let has_planned = !planned.is_empty();
        changes.extend(planned);

        for marker in ["spend ", "spending ", "live on "] {
            if has_planned {
                break;
            }
            if let Some(mut cents) = amount_after(&t, marker) {
                if t.contains("a month")
                    || t.contains("per month")
                    || t.contains("/month")
                    || t.contains("monthly")
                {
                    cents *= 12;
                }
                changes.push(Change::Spending { cents });
                break;
            }
        }

        // Read the words right after the moving phrase, not after the first "to " in the
        // sentence ("I want to move to Florida").
        if let Some(after) = ["move to ", "moving to ", "relocate to "]
            .iter()
            .find_map(|m| t.find(m).map(|i| &t[i + m.len()..]))
        {
            let after = after
                .trim_matches(|c: char| !c.is_alphanumeric() && c != ' ')
                .to_string();
            let named = STATES
                .iter()
                .find(|(n, _)| after.starts_with(n))
                .map(|(_, c)| c.to_string());
            let coded = after
                .split_whitespace()
                .next()
                .filter(|w| w.len() == 2 && STATES.iter().any(|(_, c)| c.eq_ignore_ascii_case(w)))
                .map(|w| w.to_uppercase());
            if let Some(state) = named.or(coded) {
                changes.push(Change::MoveTo { state });
            }
        }

        for word in [
            "bonus",
            "stock vest",
            "vest",
            "windfall",
            "sell the business",
            "business sale",
        ] {
            if let Some(i) = t.find(word) {
                let rest = &t[i..];
                let year = rest
                    .split_whitespace()
                    .filter_map(|w| {
                        w.trim_matches(|c: char| !c.is_ascii_digit())
                            .parse::<i32>()
                            .ok()
                    })
                    .find(|y| (today.year()..=today.year() + 60).contains(y));
                // "$50k bonus in 2028" puts the amount before the word; a bare number after
                // it must not be the year itself.
                let cents = money_tokens(rest)
                    .first()
                    .copied()
                    .or_else(|| money_tokens(&t[..i]).last().copied())
                    .or_else(|| {
                        amount_after(rest, word)
                            .filter(|c| Some(*c) != year.map(|y| i64::from(y) * 100))
                    });
                if let (Some(cents), Some(year)) = (cents, year) {
                    let mut label = word.to_string();
                    label[..1].make_ascii_uppercase();
                    changes.push(Change::OneTime {
                        label,
                        person,
                        cents,
                        year,
                    });
                    break;
                }
            }
        }

        // Market and horizon what-ifs map onto the assumptions with options.
        let has = |words: &[&str]| words.iter().any(|w| t.contains(w));
        if has(&[
            "crash",
            "bear market",
            "cautious",
            "pessimistic",
            "bad market",
            "poor returns",
            "low returns",
        ]) {
            pick("real_return", "Cautious");
        } else if has(&[
            "optimistic",
            "bull market",
            "great returns",
            "strong returns",
        ]) {
            pick("real_return", "Optimistic");
        }
        if has(&[
            "volatile",
            "volatility",
            "aggressive portfolio",
            "all stocks",
            "more risk",
        ]) {
            pick("return_volatility", "Aggressive");
        } else if has(&[
            "conservative portfolio",
            "less risk",
            "bond heavy",
            "bond-heavy",
        ]) {
            pick("return_volatility", "Gentle");
        }
        // "to 90" is an age only when nothing amount-like follows ("up to 90k", "to 90,000",
        // "to 90%") and it is not "up to" (a spending cap).
        let age_phrase = |w: &str| {
            t.match_indices(w).any(|(i, _)| {
                let after = t[i + w.len()..].chars().next();
                let amountish = after.is_some_and(|c| c.is_alphanumeric() || c == ',' || c == '%')
                    || t[i + w.len()..].starts_with('.')
                        && t[i + w.len() + 1..].starts_with(|c: char| c.is_ascii_digit());
                !amountish && !t[..i].trim_end().ends_with("up")
            })
        };
        let horizon = |n: u32| {
            [
                format!("live to {n}"),
                format!("to age {n}"),
                format!("until {n}"),
                format!("to {n}"),
            ]
            .iter()
            .any(|w| age_phrase(w))
        };
        if horizon(100) {
            pick("horizon_age", "Age 100");
        } else if horizon(95) {
            pick("horizon_age", "Age 95");
        } else if horizon(90) {
            pick("horizon_age", "Age 90");
        }
        // Look for the age only after the Social Security words, so "retire at 67 and claim
        // at 70" claims at 70.
        if let Some(ss) = ["social security", "claim"]
            .iter()
            .filter_map(|w| t.find(w))
            .min()
            .map(|i| &t[i..])
        {
            for (age, label) in [(62, "Age 62"), (67, "Age 67"), (70, "Age 70")] {
                if ss.contains(&format!("at {age}")) || ss.contains(&format!("age {age}")) {
                    pick("ss_claim_age", label);
                    break;
                }
            }
        }
        if has(&[
            "spending falls",
            "spending declines",
            "spend less as",
            "spending drops",
        ]) {
            pick("spending_change", "Declines");
        } else if has(&[
            "healthcare costs rise",
            "spending rises",
            "spend more as",
            "medical costs",
        ]) {
            pick("spending_change", "Rises");
        }

        changes.extend(out);
        Interpretation { changes }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WhatIfReply {
    pub understood: bool,
    pub message: String,
    pub changes: Vec<Change>,
    pub descriptions: Vec<String>,
    pub before: Option<QuickPlanResult>,
    pub after: Option<QuickPlanResult>,
}

/// Everything needed to run a what-if without holding the app lock.
pub struct WhatIfPrep {
    pub message: String,
    pub changes: Vec<Change>,
    pub descriptions: Vec<String>,
    pub base: Option<(fin_plan::projection::PlanInput, BTreeMap<String, String>)>,
    pub alt: Option<(fin_plan::projection::PlanInput, BTreeMap<String, String>)>,
}

impl WhatIfPrep {
    /// Run both plans (the CPU-heavy part).
    pub fn run(self, runs: usize) -> Result<WhatIfReply, String> {
        let (before, after) = match (&self.base, &self.alt) {
            (Some((bp, bk)), Some((ap, ak))) => (
                Some(quickplan::run_plan(bp, bk, runs)?),
                Some(quickplan::run_plan(ap, ak, runs)?),
            ),
            _ => (None, None),
        };
        Ok(WhatIfReply {
            understood: !self.changes.is_empty(),
            message: self.message,
            changes: self.changes,
            descriptions: self.descriptions,
            before,
            after,
        })
    }
}

const HELP: &str = "I couldn't turn that into a change yet. Try things like: \"what if I retire at 65\", \"spend $80k a year\", \"what if the market crashes\", \"claim Social Security at 70\", \"move to Florida\", \"a bonus of $100k in 2028\", \"a 5% raise\", \"buy a house for $400k in 2030\", \"college costs $30k a year from 2036 to 2040\" or \"an inheritance of $200k in 2032\".";

impl App {
    /// Swap in a different interpreter (for example one backed by a language model).
    pub fn with_interpreter(mut self, i: Box<dyn Interpreter>) -> App {
        self.interpreter = i;
        self
    }

    pub fn prepare_whatif(&self, text: &str, today: NaiveDate) -> Result<WhatIfPrep, String> {
        self.prepare_whatif_on(text, today, None)
    }

    /// A what-if stacked on a saved scenario: the plan "before" is that scenario, and the
    /// reply's changes are the scenario's own followed by the new ones, so saving or keeping
    /// the result carries the whole stack.
    pub fn prepare_whatif_on(
        &self,
        text: &str,
        today: NaiveDate,
        on: Option<u32>,
    ) -> Result<WhatIfPrep, String> {
        let current = self.household()?;
        let (prior, h) = match on {
            None => (vec![], current),
            Some(id) => {
                let s = self
                    .list_scenarios()?
                    .into_iter()
                    .find(|s| s.id == id)
                    .ok_or("That scenario no longer exists.")?;
                let mut h = current;
                for c in &s.changes {
                    c.apply(&mut h)?;
                }
                (s.changes, h)
            }
        };
        let found = self.interpreter.interpret(text, &h, today);
        if found.changes.is_empty() {
            return Ok(WhatIfPrep {
                message: HELP.into(),
                changes: vec![],
                descriptions: vec![],
                base: None,
                alt: None,
            });
        }
        let mut alt = h.clone();
        for c in &found.changes {
            c.apply(&mut alt)?;
        }
        let mut changes = prior;
        changes.extend(found.changes.iter().cloned());
        let base = (h.plan_input(today)?, h.picks.clone());
        let altp = (
            alt.plan_input(today)
                .map_err(|e| format!("That change doesn't work: {e}"))?,
            alt.picks.clone(),
        );
        let descriptions: Vec<String> = found.changes.iter().map(|c| c.describe(&h)).collect();
        Ok(WhatIfPrep {
            message: format!("Here is what changes: {}.", descriptions.join("; ")),
            changes,
            descriptions,
            base: Some(base),
            alt: Some(altp),
        })
    }

    /// Keep a what-if: save the changes into the household.
    pub fn apply_changes(&self, changes: &[Change]) -> Result<Household, String> {
        let mut h = self.household()?;
        for c in changes {
            c.apply(&mut h)?;
        }
        self.save_household(&h)?;
        self.household()
    }
}
