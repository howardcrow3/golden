use super::brackets::{Bracket, BracketTable};
use super::federal::FilingStatus;
use fin_core::Money;
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StateKind {
    None,
    Flat,
    Progressive,
}

/// How one state taxes ordinary income.
#[derive(Debug, Clone, Deserialize)]
pub struct StateRules {
    pub name: String,
    pub kind: StateKind,
    pub brackets: BracketTable,
    pub std_single: Money,
    pub std_joint: Money,
    pub joint_multiplier: f64,
    /// Explicit joint-filer brackets, when the state publishes them. Used
    /// instead of scaling the single brackets by `joint_multiplier`.
    #[serde(default)]
    pub joint_brackets: Option<BracketTable>,
    /// Non-refundable personal credits subtracted from the tax.
    #[serde(default)]
    pub credit_single: Money,
    #[serde(default)]
    pub credit_joint: Money,
    /// True when the schedule is simplified; the UI labels these as estimates.
    pub approximate: bool,
    pub notes: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StateTaxYear {
    pub year: i32,
    pub source: String,
    states: BTreeMap<String, StateRules>,
}

impl StateTaxYear {
    pub fn y2026() -> Self {
        serde_json::from_str(include_str!("../../data/states_2026.json"))
            .expect("bundled state data")
    }

    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }

    pub fn codes(&self) -> impl Iterator<Item = &str> {
        self.states.keys().map(|s| s.as_str())
    }

    pub fn rules(&self, code: &str) -> Option<&StateRules> {
        self.states.get(&code.to_ascii_uppercase())
    }

    /// State income tax on `federal_agi`-like income for a resident of `code`.
    /// Returns None if the state code is unknown.
    pub fn income_tax(&self, code: &str, fs: FilingStatus, income: Money) -> Option<Money> {
        let r = self.rules(code)?;
        if r.kind == StateKind::None {
            return Some(Money::ZERO);
        }
        let joint = fs == FilingStatus::MarriedJoint;
        let taxable = (income - if joint { r.std_joint } else { r.std_single }).max(Money::ZERO);
        let table = if let (true, Some(jb)) = (joint, r.joint_brackets.as_ref()) {
            jb.clone()
        } else if joint && r.joint_multiplier != 1.0 {
            BracketTable::new(
                r.brackets
                    .0
                    .iter()
                    .map(|b| Bracket {
                        floor: b.floor.mul_rate(r.joint_multiplier),
                        rate: b.rate,
                    })
                    .collect(),
            )
        } else {
            r.brackets.clone()
        };
        let credit = if joint {
            r.credit_joint
        } else {
            r.credit_single
        };
        Some((table.tax(taxable) - credit).max(Money::ZERO))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(d: i64) -> Money {
        Money::from_dollars(d)
    }

    #[test]
    fn covers_all_fifty_states_and_dc() {
        let y = StateTaxYear::y2026();
        assert_eq!(y.codes().count(), 51);
        for c in ["CA", "NY", "TX", "FL", "DC", "WA", "NH"] {
            assert!(y.rules(c).is_some(), "{c}");
        }
        assert!(y.rules("ZZ").is_none());
    }

    #[test]
    fn no_tax_states_owe_nothing() {
        let y = StateTaxYear::y2026();
        for c in ["TX", "FL", "WA", "NV", "AK", "SD", "WY", "TN", "NH"] {
            assert_eq!(
                y.income_tax(c, FilingStatus::Single, m(500_000)),
                Some(Money::ZERO),
                "{c}"
            );
        }
    }

    #[test]
    fn flat_state() {
        let y = StateTaxYear::y2026();
        assert_eq!(
            y.income_tax("PA", FilingStatus::Single, m(100_000)),
            Some(m(3_070))
        );
        assert_eq!(
            y.income_tax("pa", FilingStatus::MarriedJoint, m(100_000)),
            Some(m(3_070))
        );
        assert_eq!(
            y.income_tax("IL", FilingStatus::Single, m(100_000)),
            Some(m(4_950))
        );
    }

    #[test]
    fn progressive_california() {
        let y = StateTaxYear::y2026();
        let t = y
            .income_tax("CA", FilingStatus::Single, m(100_000))
            .unwrap();
        // Taxable 94,460; brackets through 9.3%: plausible band for CA.
        assert!(t > m(5_000) && t < m(7_000), "{t}");
    }

    #[test]
    fn joint_doubles_floors_when_declared() {
        let y = StateTaxYear::y2026();
        let single = y.income_tax("CA", FilingStatus::Single, m(80_000)).unwrap();
        let joint = y
            .income_tax("CA", FilingStatus::MarriedJoint, m(160_000))
            .unwrap();
        // Twice the income, twice the floors and deduction: about twice the tax.
        let diff = (joint.cents() - 2 * single.cents()).abs();
        assert!(diff < 20_000, "{diff}");
    }

    #[test]
    fn income_below_deduction_owes_nothing() {
        let y = StateTaxYear::y2026();
        assert_eq!(
            y.income_tax("VA", FilingStatus::Single, m(5_000)),
            Some(Money::ZERO)
        );
    }

    #[test]
    fn exact_states_match_published_schedules() {
        let y = StateTaxYear::y2026();
        // Missouri: federal-sized deduction, 0% to $1,348, top 4.7%.
        // $100k single: taxable 83,900, tax 262.86 on the ramp plus 4.7% of the rest.
        let mo = y
            .income_tax("MO", FilingStatus::Single, m(100_000))
            .unwrap();
        assert_eq!(mo, Money::from_cents(376_267));
        // Alabama: explicit joint brackets differ from single.
        let single = y.income_tax("AL", FilingStatus::Single, m(50_000)).unwrap();
        let joint = y
            .income_tax("AL", FilingStatus::MarriedJoint, m(50_000))
            .unwrap();
        assert!(joint < single, "{joint} {single}");
        // Delaware: credit lowers tax, never below zero.
        assert_eq!(
            y.income_tax("DE", FilingStatus::Single, m(3_000)),
            Some(Money::ZERO)
        );
        for c in [
            "AL", "DC", "DE", "HI", "KS", "ME", "MO", "MT", "ND", "NE", "NJ", "NM", "OK", "OR",
            "RI", "SC", "WI", "WV",
        ] {
            assert!(!y.rules(c).unwrap().approximate, "{c}");
        }
    }

    #[test]
    fn partially_modelled_states_stay_flagged() {
        let y = StateTaxYear::y2026();
        for c in ["AR", "CT", "MD", "MN", "NY", "VT"] {
            assert!(y.rules(c).unwrap().approximate, "{c}");
        }
    }

    #[test]
    fn arkansas_top_rate_is_3_7_percent() {
        let y = StateTaxYear::y2026();
        let r = y.rules("AR").unwrap();
        assert_eq!(r.brackets.0.last().unwrap().rate, 0.037);
        // Well above the top bracket, an extra $10,000 is taxed at 3.7%.
        let a = y
            .income_tax("AR", FilingStatus::Single, m(200_000))
            .unwrap();
        let b = y
            .income_tax("AR", FilingStatus::Single, m(210_000))
            .unwrap();
        assert_eq!(b - a, m(370));
    }

    #[test]
    fn minnesota_matches_the_department_of_revenue_2026_figures() {
        let y = StateTaxYear::y2026();
        let r = y.rules("MN").unwrap();
        assert_eq!(r.std_single, m(15_300));
        assert_eq!(r.std_joint, m(30_600));
        let floors: Vec<_> = r.brackets.0.iter().map(|b| b.floor).collect();
        assert_eq!(floors, vec![m(0), m(33_310), m(109_430), m(203_150)]);
        let joint: Vec<_> = r
            .joint_brackets
            .as_ref()
            .unwrap()
            .0
            .iter()
            .map(|b| b.floor)
            .collect();
        assert_eq!(joint, vec![m(0), m(48_700), m(193_480), m(337_930)]);
    }

    #[test]
    fn approximate_states_are_flagged() {
        let y = StateTaxYear::y2026();
        assert!(y.rules("NY").unwrap().approximate);
        assert!(!y.rules("PA").unwrap().approximate);
        assert!(!y.rules("TX").unwrap().approximate);
    }

    #[test]
    fn higher_income_never_pays_less_in_any_state() {
        let y = StateTaxYear::y2026();
        for c in y.codes().map(String::from).collect::<Vec<_>>() {
            for fs in FilingStatus::ALL {
                let mut prev = Money::ZERO;
                for k in 0..40 {
                    let t = y.income_tax(&c, fs, m(k * 25_000)).unwrap();
                    assert!(t >= prev, "{c} {fs:?} {k}");
                    prev = t;
                }
            }
        }
    }
}
