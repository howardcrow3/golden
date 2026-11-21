//! Suggested assumptions: which of the existing options (each with its likelihood) suit this
//! household. Suggestions never change anything; the person accepts one (which sets the pick the
//! normal way) or rejects it. The model sees only age bands and retirement ages, and every
//! suggestion is checked against the real options, so it cannot invent a value.
use crate::explain::numbers_in;
use crate::household::Household;
use crate::App;
use chrono::Datelike;
use fin_plan::params::Assumptions;
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Serialize, PartialEq, Eq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AssumptionSuggestion {
    pub key: String,
    pub label: String,
    pub reason: String,
    /// "model" or "built-in".
    pub source: String,
}

fn age_band(age: i32) -> &'static str {
    match age {
        i32::MIN..=39 => "under 40",
        40..=49 => "in their 40s",
        50..=59 => "in their 50s",
        60..=69 => "in their 60s",
        _ => "70 or older",
    }
}

fn ages(h: &Household, year: i32) -> Vec<i32> {
    h.people
        .iter()
        .map(|p| {
            p.birth_date
                .get(..4)
                .and_then(|y| y.parse::<i32>().ok())
                .map_or(50, |y| year - y)
        })
        .collect()
}

/// What the plan currently uses for each assumption: the person's pick, else the likeliest.
fn active_label(h: &Household, a: &Assumptions, key: &str) -> Option<String> {
    let p = a.get(key).ok()?;
    Some(
        h.picks
            .get(key)
            .cloned()
            .unwrap_or_else(|| p.options[p.likeliest()].label.clone()),
    )
}

/// Plain rules that need no model.
pub fn rule_suggestions(h: &Household, year: i32) -> Vec<AssumptionSuggestion> {
    let a = Assumptions::defaults();
    let ages = ages(h, year);
    let mut out = vec![];
    let mut add = |key: &str, label: &str, reason: &str| {
        if active_label(h, &a, key).as_deref() != Some(label)
            && a.get(key)
                .is_ok_and(|p| p.options.iter().any(|o| o.label == label))
        {
            out.push(AssumptionSuggestion {
                key: key.into(),
                label: label.into(),
                reason: reason.into(),
                source: "built-in".into(),
            });
        }
    };
    if h.people.len() >= 2 {
        add(
            "horizon_age",
            "Age 95",
            "With two people, planning further out covers the chance that one of you lives longer.",
        );
    }
    let soonest = h
        .people
        .iter()
        .zip(&ages)
        .map(|(p, age)| p.retire_age - age)
        .min();
    if soonest.is_some_and(|y| (0..=10).contains(&y)) {
        add(
            "return_volatility",
            "Gentle",
            "Retirement is close, and a gentler portfolio lowers the damage of a bad market early on.",
        );
    }
    out
}

fn prompt(h: &Household, a: &Assumptions, year: i32) -> String {
    let ages = ages(h, year);
    let people: Vec<String> = h
        .people
        .iter()
        .zip(&ages)
        .map(|(p, age)| {
            format!(
                "  - {} (plans to retire at {})",
                age_band(*age),
                p.retire_age
            )
        })
        .collect();
    let mut keys = String::new();
    for key in a.keys() {
        let Ok(p) = a.get(key) else { continue };
        let probs = p.probabilities();
        let opts: Vec<String> = p
            .options
            .iter()
            .zip(probs)
            .map(|(o, pr)| {
                format!(
                    "\"{}\" ({}% likely, {})",
                    o.label,
                    (pr * 100.0).round(),
                    o.source
                )
            })
            .collect();
        keys.push_str(&format!(
            "  {key} (now: {}): {}\n",
            active_label(h, a, key).unwrap_or_default(),
            opts.join("; ")
        ));
    }
    format!(
        "You help choose planning assumptions for a retirement plan. Suggest at most 3 changes, \
         only where another option suits this household better than the current one. Each \
         suggestion must use a key and option label exactly as written below, with a short plain \
         reason that uses no numbers. Reply with ONLY JSON: \
         {{\"suggestions\":[{{\"key\":\"...\",\"label\":\"...\",\"reason\":\"...\"}}]}}.\n\
         People:\n{}\nAssumptions:\n{keys}",
        people.join("\n")
    )
}

/// Keep suggestions that name a real option other than the current one, once per assumption,
/// with a reason that adds no numbers of its own.
pub fn parse_suggestions(reply: &str, h: &Household, year: i32) -> Vec<AssumptionSuggestion> {
    let a = Assumptions::defaults();
    let Ok(v) = serde_json::from_str::<Value>(reply.trim()) else {
        return vec![];
    };
    let Some(items) = v.get("suggestions").and_then(Value::as_array) else {
        return vec![];
    };
    let mut out: Vec<AssumptionSuggestion> = vec![];
    for item in items {
        let get = |k: &str| item.get(k).and_then(Value::as_str).map(str::trim);
        let (Some(key), Some(label), Some(reason)) = (get("key"), get("label"), get("reason"))
        else {
            continue;
        };
        let Ok(param) = a.get(key) else { continue };
        let Some(opt) = param
            .options
            .iter()
            .find(|o| o.label.eq_ignore_ascii_case(label))
        else {
            continue;
        };
        if active_label(h, &a, key).as_deref() == Some(opt.label.as_str())
            || out.iter().any(|s| s.key == key)
            || reason.chars().count() < 10
            || reason.chars().count() > 160
        {
            continue;
        }
        let mut allowed: Vec<String> = numbers_in(&opt.label);
        allowed.extend(numbers_in(&opt.source));
        allowed.extend(h.people.iter().map(|p| p.retire_age.to_string()));
        if !numbers_in(reason).iter().all(|n| allowed.contains(n)) {
            continue;
        }
        out.push(AssumptionSuggestion {
            key: key.into(),
            label: opt.label.clone(),
            reason: reason.into(),
            source: "model".into(),
        });
    }
    let _ = year;
    out
}

impl App {
    /// Suggestions from the model when one is set up and gives usable answers, else from plain
    /// rules. Nothing is saved; accepting one goes through `set_pick`.
    pub fn suggest_assumptions(&self) -> Result<Vec<AssumptionSuggestion>, String> {
        let h = self.household()?;
        let year = (self.clock)().year();
        let a = Assumptions::defaults();
        if self.ai.is_some() {
            if let Ok(reply) = self.ask_model("suggest_assumptions", &prompt(&h, &a, year)) {
                let found = parse_suggestions(&reply, &h, year);
                if !found.is_empty() {
                    return Ok(found);
                }
            }
        }
        Ok(rule_suggestions(&h, year))
    }
}
