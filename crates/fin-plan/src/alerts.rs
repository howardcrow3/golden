//! Plain-language heads-ups drawn from a finished plan: "you are off track", and what is coming
//! soon. Shown in the app only; nothing is sent anywhere.
use crate::milestones::Milestone;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Good,
    Info,
    Warn,
    Danger,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Alert {
    pub key: String,
    pub severity: Severity,
    pub message: String,
}

fn a(key: &str, severity: Severity, message: String) -> Alert {
    Alert {
        key: key.into(),
        severity,
        message,
    }
}

/// How many years ahead counts as "soon" for dates on the milestones list.
pub const SOON_YEARS: i32 = 3;

pub struct Facts<'a> {
    pub success_rate: f64,
    pub depleted_year: Option<i32>,
    /// The year the plan starts in.
    pub this_year: i32,
    pub milestones: &'a [Milestone],
    pub estate_tax_cents: i64,
}

/// Worst first.
pub fn find(f: &Facts) -> Vec<Alert> {
    let mut out = Vec::new();
    let pct = (f.success_rate * 100.0).round() as i32;
    if let Some(y) = f.depleted_year {
        out.push(a(
            "depleted",
            Severity::Danger,
            format!("In the average-return projection the money runs out in {y}."),
        ));
    }
    if f.success_rate < 0.5 {
        out.push(a(
            "off_track",
            Severity::Danger,
            format!("You are off track: the plan works in only {pct}% of simulated futures."),
        ));
    } else if f.success_rate < 0.75 {
        out.push(a(
            "slipping",
            Severity::Warn,
            format!("The plan works in {pct}% of simulated futures. Aim for 75% or more."),
        ));
    } else if f.depleted_year.is_none() {
        out.push(a(
            "on_track",
            Severity::Good,
            format!("You are on track: the plan works in {pct}% of simulated futures."),
        ));
    }
    for (key, what) in [
        ("medicare", "Medicare starts"),
        ("rmd", "Required withdrawals begin"),
        ("ss", "Social Security starts"),
        ("retire", "Work ends"),
    ] {
        if let Some(m) = f.milestones.iter().find(|m| m.key == key) {
            let away = m.year - f.this_year;
            if (0..=SOON_YEARS).contains(&away) {
                let when = match away {
                    0 => "this year".to_string(),
                    1 => "next year".to_string(),
                    n => format!("in {n} years"),
                };
                out.push(a(
                    key,
                    Severity::Info,
                    format!("{what} {when} ({}).", m.year),
                ));
            }
        }
    }
    if f.estate_tax_cents > 0 {
        out.push(a(
            "estate_tax",
            Severity::Warn,
            "Your estate would owe federal estate tax at the end of the plan.".into(),
        ));
    }
    out.sort_by_key(|x| std::cmp::Reverse(rank(x.severity)));
    out
}

fn rank(s: Severity) -> u8 {
    match s {
        Severity::Danger => 3,
        Severity::Warn => 2,
        Severity::Info => 1,
        Severity::Good => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(rate: f64, depleted: Option<i32>, ms: &[Milestone]) -> Facts<'_> {
        Facts {
            success_rate: rate,
            depleted_year: depleted,
            this_year: 2026,
            milestones: ms,
            estate_tax_cents: 0,
        }
    }
    fn ms(key: &str, year: i32) -> Milestone {
        Milestone {
            key: key.into(),
            label: key.into(),
            year,
            age: 60,
        }
    }

    #[test]
    fn a_healthy_plan_is_on_track() {
        let r = find(&facts(0.92, None, &[]));
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].severity, Severity::Good);
        assert!(r[0].message.contains("92%"));
    }

    #[test]
    fn a_weak_plan_is_off_track_and_listed_before_good_news() {
        let r = find(&facts(0.4, Some(2060), &[]));
        assert_eq!(r[0].severity, Severity::Danger);
        assert!(r.iter().any(|x| x.key == "off_track"));
        assert!(r
            .iter()
            .any(|x| x.key == "depleted" && x.message.contains("2060")));
        assert!(!r.iter().any(|x| x.key == "on_track"));
    }

    #[test]
    fn a_middling_plan_gets_a_warning() {
        let r = find(&facts(0.6, None, &[]));
        assert_eq!(r[0].severity, Severity::Warn);
        assert_eq!(r[0].key, "slipping");
    }

    #[test]
    fn only_dates_within_three_years_are_mentioned() {
        let m = [
            ms("medicare", 2027),
            ms("rmd", 2040),
            ms("retire", 2026),
            ms("ss", 2029),
        ];
        let r = find(&facts(0.9, None, &m));
        let keys: Vec<_> = r.iter().map(|x| x.key.as_str()).collect();
        assert!(keys.contains(&"medicare") && keys.contains(&"retire") && keys.contains(&"ss"));
        assert!(!keys.contains(&"rmd"));
        let med = r.iter().find(|x| x.key == "medicare").unwrap();
        assert!(med.message.contains("next year"));
        let ret = r.iter().find(|x| x.key == "retire").unwrap();
        assert!(ret.message.contains("this year"));
        let ss = r.iter().find(|x| x.key == "ss").unwrap();
        assert!(ss.message.contains("in 3 years"));
    }

    #[test]
    fn past_dates_are_not_alerts_and_estate_tax_warns() {
        let m = [ms("retire", 2020)];
        let mut f = facts(0.9, None, &m);
        f.estate_tax_cents = 1;
        let r = find(&f);
        assert!(!r.iter().any(|x| x.key == "retire"));
        assert!(r.iter().any(|x| x.key == "estate_tax"));
    }
}
