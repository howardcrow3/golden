//! What-if chat. A person types a sentence; an `Interpreter` turns it into concrete changes;
//! Golden runs the plan with and without them and shows both. Nothing is saved until the
//! person keeps the change. The built-in `RuleInterpreter` understands common phrasings with
//! no network or model; a language model can be plugged in behind the same trait later.
use crate::household::{Household, StreamRow};
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
}

impl Change {
    pub fn describe(&self, h: &Household) -> String {
        match self {
            Change::SetPick { key, label } => format!("Assume {}: {label}", pick_name(key)),
            Change::RetireAt { person, age } => format!(
                "{} retires at {age}",
                h.people.get(*person).map_or("Someone", |p| p.name.as_str())
            ),
            Change::Spending { cents } => {
                format!("Spend ${} a year", group(*cents / 100))
            }
            Change::MoveTo { state } => format!("Live in {state}"),
            Change::OneTime {
                label, cents, year, ..
            } => format!("{label} of ${} in {year}", group(*cents / 100)),
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
                start_year: Some(*year),
                end_year: Some(*year),
                contribution_cents: 0,
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
    if !v.is_finite() || v < 0.0 {
        return None;
    }
    Some((v * mult * 100.0).round() as i64)
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

        for marker in ["spend ", "spending ", "live on "] {
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

        if t.contains("move to ") || t.contains("moving to ") || t.contains("relocate to ") {
            let after = t
                .split("to ")
                .nth(1)
                .unwrap_or("")
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
            "inheritance",
            "sell the business",
            "business sale",
        ] {
            if let Some(i) = t.find(word) {
                let rest = &t[i..];
                let cents = amount_after(rest, word);
                let year = rest
                    .split_whitespace()
                    .filter_map(|w| {
                        w.trim_matches(|c: char| !c.is_ascii_digit())
                            .parse::<i32>()
                            .ok()
                    })
                    .find(|y| (today.year()..=today.year() + 60).contains(y));
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
        if has(&["live to 100", "to age 100", "until 100", "to 100"]) {
            pick("horizon_age", "Age 100");
        } else if has(&["live to 95", "to age 95", "until 95", "to 95"]) {
            pick("horizon_age", "Age 95");
        } else if has(&["live to 90", "to age 90", "until 90", "to 90"]) {
            pick("horizon_age", "Age 90");
        }
        if has(&["social security", "claim"]) {
            for (age, label) in [(62, "Age 62"), (67, "Age 67"), (70, "Age 70")] {
                if t.contains(&format!("at {age}")) || t.contains(&format!("age {age}")) {
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

const HELP: &str = "I couldn't turn that into a change yet. Try things like: \"what if I retire at 65\", \"spend $80k a year\", \"what if the market crashes\", \"claim Social Security at 70\", \"move to Florida\", or \"a bonus of $100k in 2028\".";

impl App {
    /// Swap in a different interpreter (for example one backed by a language model).
    pub fn with_interpreter(mut self, i: Box<dyn Interpreter>) -> App {
        self.interpreter = i;
        self
    }

    pub fn prepare_whatif(&self, text: &str, today: NaiveDate) -> Result<WhatIfPrep, String> {
        let h = self.household()?;
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
        let base = (h.plan_input(today)?, h.picks.clone());
        let altp = (
            alt.plan_input(today)
                .map_err(|e| format!("That change doesn't work: {e}"))?,
            alt.picks.clone(),
        );
        let descriptions: Vec<String> = found.changes.iter().map(|c| c.describe(&h)).collect();
        Ok(WhatIfPrep {
            message: format!("Here is what changes: {}.", descriptions.join("; ")),
            changes: found.changes,
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
