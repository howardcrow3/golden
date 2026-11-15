//! Parameters with options: every projection input holds several candidate values, each with
//! a likelihood and a source. The app picks the likeliest; a what-if is just a list of picks.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParamOption {
    pub label: String,
    pub value: f64,
    /// Relative weight; need not sum to 1 across options.
    pub likelihood: f64,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Parameter {
    pub key: String,
    pub options: Vec<ParamOption>,
    /// Index of the option the plan currently uses.
    pub active: usize,
    /// When true the app never auto-switches this parameter.
    pub pinned: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParamError {
    #[error("unknown parameter '{0}'")]
    Unknown(String),
    #[error("parameter '{key}' has no option '{label}'")]
    NoOption { key: String, label: String },
    #[error("parameter '{0}' needs at least one option")]
    Empty(String),
}

impl Parameter {
    pub fn new(key: &str, options: Vec<ParamOption>) -> Result<Parameter, ParamError> {
        if options.is_empty() {
            return Err(ParamError::Empty(key.into()));
        }
        let mut p = Parameter {
            key: key.into(),
            options,
            active: 0,
            pinned: false,
        };
        p.active = p.likeliest();
        Ok(p)
    }

    /// Index of the most likely option (first wins ties).
    pub fn likeliest(&self) -> usize {
        let mut best = 0;
        for (i, o) in self.options.iter().enumerate() {
            if o.likelihood > self.options[best].likelihood {
                best = i;
            }
        }
        best
    }

    pub fn value(&self) -> f64 {
        self.options[self.active].value
    }

    /// Option probabilities normalized to sum to one.
    pub fn probabilities(&self) -> Vec<f64> {
        let total: f64 = self.options.iter().map(|o| o.likelihood.max(0.0)).sum();
        if total <= 0.0 {
            return vec![1.0 / self.options.len() as f64; self.options.len()];
        }
        self.options
            .iter()
            .map(|o| o.likelihood.max(0.0) / total)
            .collect()
    }

    /// The (low, high) option values, used by the sensitivity view.
    pub fn range(&self) -> (f64, f64) {
        let vals = self.options.iter().map(|o| o.value);
        (
            vals.clone().fold(f64::INFINITY, f64::min),
            vals.fold(f64::NEG_INFINITY, f64::max),
        )
    }

    /// Sample an option index given a uniform `u` in [0, 1) (used per Monte Carlo run).
    pub fn sample(&self, u: f64) -> usize {
        let mut acc = 0.0;
        for (i, p) in self.probabilities().iter().enumerate() {
            acc += p;
            if u < acc {
                return i;
            }
        }
        self.options.len() - 1
    }
}

/// A set of parameters plus optional what-if picks layered on top.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Assumptions {
    params: BTreeMap<String, Parameter>,
}

/// A what-if: parameter key → option label. Costs nothing to throw away.
pub type Picks = BTreeMap<String, String>;

impl Assumptions {
    pub fn insert(&mut self, p: Parameter) {
        self.params.insert(p.key.clone(), p);
    }

    pub fn get(&self, key: &str) -> Result<&Parameter, ParamError> {
        self.params
            .get(key)
            .ok_or_else(|| ParamError::Unknown(key.into()))
    }

    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.params.keys().map(|s| s.as_str())
    }

    /// Value of `key` with `picks` applied on top of the active options.
    pub fn value(&self, key: &str, picks: &Picks) -> Result<f64, ParamError> {
        let p = self.get(key)?;
        match picks.get(key) {
            None => Ok(p.value()),
            Some(label) => p
                .options
                .iter()
                .find(|o| &o.label == label)
                .map(|o| o.value)
                .ok_or_else(|| ParamError::NoOption {
                    key: key.into(),
                    label: label.clone(),
                }),
        }
    }

    /// Make a what-if permanent ("Make this the plan"): switch every picked parameter.
    /// All-or-nothing: nothing changes if any pick is invalid.
    pub fn apply(&mut self, picks: &Picks) -> Result<(), ParamError> {
        let mut changes = vec![];
        for (key, label) in picks {
            let p = self.get(key)?;
            let idx = p
                .options
                .iter()
                .position(|o| &o.label == label)
                .ok_or_else(|| ParamError::NoOption {
                    key: key.clone(),
                    label: label.clone(),
                })?;
            changes.push((key.clone(), idx));
        }
        for (key, idx) in changes {
            let p = self.params.get_mut(&key).unwrap();
            p.active = idx;
            p.pinned = true;
        }
        Ok(())
    }

    /// Re-rank unpinned parameters to their likeliest option (e.g. after new data arrives).
    pub fn refresh(&mut self) {
        for p in self.params.values_mut().filter(|p| !p.pinned) {
            p.active = p.likeliest();
        }
    }

    /// The default US household assumptions bundled with the app (real, annual).
    pub fn defaults() -> Assumptions {
        let o = |label: &str, value: f64, likelihood: f64, source: &str| ParamOption {
            label: label.into(),
            value,
            likelihood,
            source: source.into(),
        };
        let mut a = Assumptions::default();
        a.insert(
            Parameter::new(
                "real_return",
                vec![
                    o("Cautious", 0.02, 0.25, "Bond-heavy, low valuations"),
                    o(
                        "Historical",
                        0.04,
                        0.5,
                        "Long-run balanced portfolio, after inflation",
                    ),
                    o("Optimistic", 0.06, 0.25, "All-equity long-run average"),
                ],
            )
            .unwrap(),
        );
        a.insert(
            Parameter::new(
                "return_volatility",
                vec![
                    o("Balanced", 0.11, 0.5, "60/40 portfolio"),
                    o("Aggressive", 0.16, 0.3, "90/10"),
                    o("Gentle", 0.07, 0.2, "30/70"),
                ],
            )
            .unwrap(),
        );
        a.insert(
            Parameter::new(
                "horizon_age",
                vec![
                    o("Age 90", 90.0, 0.4, "Typical planning"),
                    o("Age 95", 95.0, 0.45, "Conservative for couples"),
                    o("Age 100", 100.0, 0.15, "Very conservative"),
                ],
            )
            .unwrap(),
        );
        a.insert(
            Parameter::new(
                "ss_claim_age",
                vec![
                    o("Age 62", 62.0, 0.2, "Earliest claim"),
                    o("Age 67", 67.0, 0.55, "Full retirement age"),
                    o("Age 70", 70.0, 0.25, "Maximum benefit"),
                ],
            )
            .unwrap(),
        );
        a.insert(
            Parameter::new(
                "spending_change",
                vec![
                    o("Flat", 0.0, 0.5, "Real spending constant"),
                    o(
                        "Declines",
                        -0.01,
                        0.3,
                        "Retiree spending typically falls ~1%/yr",
                    ),
                    o("Rises", 0.01, 0.2, "Healthcare-driven"),
                ],
            )
            .unwrap(),
        );
        a
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn p() -> Parameter {
        Parameter::new(
            "x",
            vec![
                ParamOption {
                    label: "a".into(),
                    value: 1.0,
                    likelihood: 1.0,
                    source: "s".into(),
                },
                ParamOption {
                    label: "b".into(),
                    value: 2.0,
                    likelihood: 3.0,
                    source: "s".into(),
                },
                ParamOption {
                    label: "c".into(),
                    value: 3.0,
                    likelihood: 1.0,
                    source: "s".into(),
                },
            ],
        )
        .unwrap()
    }

    #[test]
    fn picks_likeliest_by_default() {
        assert_eq!(p().active, 1);
        assert_eq!(p().value(), 2.0);
    }

    #[test]
    fn empty_options_rejected() {
        assert_eq!(
            Parameter::new("k", vec![]),
            Err(ParamError::Empty("k".into()))
        );
    }

    #[test]
    fn probabilities_normalize() {
        let pr = p().probabilities();
        assert!((pr.iter().sum::<f64>() - 1.0).abs() < 1e-12);
        assert!((pr[1] - 0.6).abs() < 1e-12);
    }

    #[test]
    fn sampling_follows_weights() {
        let q = p();
        assert_eq!(q.sample(0.0), 0);
        assert_eq!(q.sample(0.25), 1);
        assert_eq!(q.sample(0.79), 1);
        assert_eq!(q.sample(0.95), 2);
        assert_eq!(q.sample(0.9999999), 2);
    }

    #[test]
    fn range_spans_options() {
        assert_eq!(p().range(), (1.0, 3.0));
    }

    #[test]
    fn what_if_picks_override_without_changing_the_plan() {
        let a = Assumptions::defaults();
        let mut picks = Picks::new();
        picks.insert("ss_claim_age".into(), "Age 70".into());
        assert_eq!(a.value("ss_claim_age", &picks).unwrap(), 70.0);
        assert_eq!(a.value("ss_claim_age", &Picks::new()).unwrap(), 67.0);
    }

    #[test]
    fn apply_is_all_or_nothing() {
        let mut a = Assumptions::defaults();
        let before = a.clone();
        let mut picks = Picks::new();
        picks.insert("real_return".into(), "Optimistic".into());
        picks.insert("ss_claim_age".into(), "Age 99".into());
        assert!(a.apply(&picks).is_err());
        assert_eq!(a, before);
    }

    #[test]
    fn apply_pins_and_refresh_respects_pins() {
        let mut a = Assumptions::defaults();
        let mut picks = Picks::new();
        picks.insert("real_return".into(), "Optimistic".into());
        a.apply(&picks).unwrap();
        a.refresh();
        assert_eq!(a.get("real_return").unwrap().value(), 0.06);
        assert!(a.get("real_return").unwrap().pinned);
        assert_eq!(a.get("horizon_age").unwrap().value(), 95.0); // unpinned stays likeliest
    }

    #[test]
    fn unknown_keys_and_labels_error() {
        let a = Assumptions::defaults();
        assert_eq!(
            a.value("nope", &Picks::new()),
            Err(ParamError::Unknown("nope".into()))
        );
        let mut picks = Picks::new();
        picks.insert("real_return".into(), "Wild".into());
        assert!(matches!(
            a.value("real_return", &picks),
            Err(ParamError::NoOption { .. })
        ));
    }

    #[test]
    fn defaults_are_serializable() {
        let a = Assumptions::defaults();
        let json = serde_json::to_string(&a).unwrap();
        assert_eq!(serde_json::from_str::<Assumptions>(&json).unwrap(), a);
    }

    proptest! {
        #[test]
        fn sample_always_in_range(u in 0.0f64..1.0) {
            let q = p();
            prop_assert!(q.sample(u) < q.options.len());
        }
    }
}
