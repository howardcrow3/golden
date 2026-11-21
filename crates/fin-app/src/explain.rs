//! Plain-English explanations of plan results. The built-in sentence template always works; when
//! a model is set up it may phrase it more naturally, but it is shown only a handful of rounded
//! facts (no balances, income or birth dates), and its text is dropped in favour of the template
//! if it mentions any number those facts do not contain.
use crate::App;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Compare {
    pub before_rate: f64,
    pub after_rate: f64,
    /// What changed, in the person's own words (for example "retire at 62").
    pub change: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExplainFacts {
    /// Share of simulated futures where the money lasts, 0.0 to 1.0.
    pub success_rate: f64,
    pub depleted_year: Option<i32>,
    pub retire_age: Option<i32>,
    pub horizon_age: Option<i32>,
    pub compare: Option<Compare>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Explanation {
    pub text: String,
    /// "model" or "built-in".
    pub source: String,
}

fn pct(rate: f64) -> i64 {
    (rate.clamp(0.0, 1.0) * 100.0).round() as i64
}

/// The sentence template used with no model, or when the model's text is refused.
pub fn template(f: &ExplainFacts) -> String {
    let p = pct(f.success_rate);
    let verdict = match p {
        90..=100 => "That is a strong result.",
        75..=89 => "That is a reasonable result, with some room to improve.",
        50..=74 => "That is shaky: small changes could matter a lot.",
        _ => "That is a weak result, and the plan needs changes to be safe.",
    };
    let mut out = format!(
        "In {p} out of 100 simulated futures your money lasts{}. {verdict}",
        f.horizon_age
            .map(|a| format!(" to age {a}"))
            .unwrap_or_default()
    );
    if let Some(y) = f.depleted_year {
        out.push_str(&format!(" In the typical bad case it runs out around {y}."));
    }
    if let Some(c) = &f.compare {
        let (b, a) = (pct(c.before_rate), pct(c.after_rate));
        let d = a - b;
        let dir = match d {
            0 => "does not change the result".to_string(),
            d if d > 0 => format!("improves it by {d} points"),
            d => format!("lowers it by {} points", -d),
        };
        out.push_str(&format!(
            " Compared with your current plan ({b} out of 100), {} {dir}.",
            c.change.trim()
        ));
    }
    out
}

/// Every whole number the facts contain, as text, so the model's text can be checked against them.
fn allowed_numbers(f: &ExplainFacts) -> BTreeSet<String> {
    let mut set = BTreeSet::new();
    let mut add = |n: i64| {
        set.insert(n.to_string());
    };
    add(pct(f.success_rate));
    add(100);
    for v in [f.depleted_year, f.retire_age, f.horizon_age]
        .into_iter()
        .flatten()
    {
        add(v as i64);
    }
    if let Some(c) = &f.compare {
        let (b, a) = (pct(c.before_rate), pct(c.after_rate));
        add(b);
        add(a);
        add((a - b).abs());
        // Numbers inside the person's own description of the change are theirs to repeat.
        for n in numbers_in(&c.change) {
            set.insert(n);
        }
    }
    set
}

fn numbers_in(text: &str) -> Vec<String> {
    let mut out = vec![];
    let mut run = String::new();
    for c in text.chars().chain(std::iter::once(' ')) {
        if c.is_ascii_digit() {
            run.push(c);
        } else if !run.is_empty() {
            out.push(run.trim_start_matches('0').to_string());
            if out.last().is_some_and(String::is_empty) {
                *out.last_mut().unwrap() = "0".into();
            }
            run.clear();
        }
    }
    out
}

/// True when every number in `text` is one the facts contain.
pub fn numbers_are_grounded(text: &str, f: &ExplainFacts) -> bool {
    let ok = allowed_numbers(f);
    numbers_in(text).iter().all(|n| ok.contains(n))
}

fn prompt(f: &ExplainFacts) -> String {
    let mut facts = format!(
        "- Chance the money lasts: {} out of 100\n",
        pct(f.success_rate)
    );
    if let Some(a) = f.retire_age {
        facts.push_str(&format!("- Retirement age: {a}\n"));
    }
    if let Some(a) = f.horizon_age {
        facts.push_str(&format!("- Planned to age: {a}\n"));
    }
    if let Some(y) = f.depleted_year {
        facts.push_str(&format!(
            "- In the typical bad case the money runs out in {y}\n"
        ));
    }
    if let Some(c) = &f.compare {
        facts.push_str(&format!(
            "- Change tested: {} (chance goes from {} to {} out of 100)\n",
            c.change
                .replace('\n', " ")
                .chars()
                .take(80)
                .collect::<String>(),
            pct(c.before_rate),
            pct(c.after_rate)
        ));
    }
    format!(
        "Explain a retirement plan result to the person in 2 to 4 plain sentences, kind and \
         honest, no jargon. Use ONLY the facts below and do not use any other number. Reply with \
         ONLY JSON: {{\"text\":\"...\"}}.\nFacts:\n{facts}"
    )
}

impl App {
    /// Explain a result. Never fails: with no model, or a refused answer, the template is used.
    pub fn explain_result(&self, facts: &ExplainFacts) -> Explanation {
        let built_in = || Explanation {
            text: template(facts),
            source: "built-in".into(),
        };
        let Ok(reply) = self.ask_model("explain", &prompt(facts)) else {
            return built_in();
        };
        let text = serde_json::from_str::<Value>(reply.trim())
            .ok()
            .and_then(|v| {
                v.get("text")
                    .and_then(Value::as_str)
                    .map(|t| t.trim().to_string())
            });
        match text {
            Some(t)
                if (20..=600).contains(&t.chars().count()) && numbers_are_grounded(&t, facts) =>
            {
                Explanation {
                    text: t,
                    source: "model".into(),
                }
            }
            _ => built_in(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts() -> ExplainFacts {
        ExplainFacts {
            success_rate: 0.874,
            depleted_year: Some(2061),
            retire_age: Some(62),
            horizon_age: Some(95),
            compare: Some(Compare {
                before_rate: 0.874,
                after_rate: 0.921,
                change: "retire at 62".into(),
            }),
        }
    }

    #[test]
    fn the_template_reads_well_and_uses_only_the_facts() {
        let t = template(&facts());
        assert!(t.contains("87 out of 100") && t.contains("age 95") && t.contains("around 2061"));
        assert!(t.contains("improves it by 5 points") && t.contains("retire at 62"));
        assert!(numbers_are_grounded(&t, &facts()));
    }

    #[test]
    fn verdict_follows_the_rate() {
        let mut f = facts();
        f.compare = None;
        f.depleted_year = None;
        for (r, word) in [
            (0.95, "strong"),
            (0.8, "reasonable"),
            (0.6, "shaky"),
            (0.2, "weak"),
        ] {
            f.success_rate = r;
            assert!(template(&f).contains(word), "{r}");
        }
    }

    #[test]
    fn invented_numbers_are_caught() {
        let f = facts();
        assert!(numbers_are_grounded(
            "About 87 out of 100 futures work, retiring at 62.",
            &f
        ));
        assert!(!numbers_are_grounded("You will have $1,200,000 at 95.", &f));
        assert!(!numbers_are_grounded("A 40 percent chance.", &f));
        assert!(numbers_are_grounded("No digits at all here.", &f));
    }
}
