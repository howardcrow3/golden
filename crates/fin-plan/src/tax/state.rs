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
    /// New York style benefit recapture for high incomes.
    #[serde(default)]
    pub recapture: Option<Recapture>,
    /// A minimum tax of `rate` x AGI once AGI is over `agi_over` (Vermont).
    #[serde(default)]
    pub agi_floor_tax: Option<AgiFloorTax>,
    /// Explicit head-of-household brackets (Connecticut).
    #[serde(default)]
    pub head_brackets: Option<BracketTable>,
    /// Connecticut style schedule where the exemption, an add-back, a recapture and a personal
    /// credit all step with adjusted gross income.
    #[serde(default)]
    pub agi_tables: Option<AgiTables>,
    /// True when the schedule is simplified; the UI labels these as estimates.
    pub approximate: bool,
    pub notes: String,
}

/// The NY IT-201-I "tax computation" worksheets: above `agi_over` the lower
/// rates are clawed back over `phase_width` of AGI, tier by tier, until the
/// whole of taxable income is taxed at the flat rate of its tier; above
/// `agi_full` every dollar is taxed at the top rate.
#[derive(Debug, Clone, Deserialize)]
pub struct Recapture {
    pub agi_over: Money,
    pub agi_full: Money,
    pub phase_width: Money,
    /// Taxable-income ceilings of the recapture tiers (single / joint).
    pub tops_single: Vec<Money>,
    pub tops_joint: Vec<Money>,
}

/// Step tables keyed by filing status ("single", "joint", "separate", "head"). Each row is
/// `[above, value]`: the value applies when AGI is more than `above` and up to the next row's
/// `above`. Money is in cents; the credit is a decimal share of the tax that is forgiven.
#[derive(Debug, Clone, Deserialize)]
pub struct AgiTables {
    pub exemption: BTreeMap<String, Vec<(i64, i64)>>,
    pub addback: BTreeMap<String, Vec<(i64, i64)>>,
    pub recapture: BTreeMap<String, Vec<(i64, i64)>>,
    pub credit: BTreeMap<String, Vec<(i64, f64)>>,
}

fn step<T: Copy>(rows: &[(i64, T)], agi: Money, below: T) -> T {
    // The first row's `above` is zero for money tables; for the credit it is the point under
    // which the whole tax is forgiven, so `below` stands in for incomes at or under it.
    let a = agi.cents();
    match rows.iter().rev().find(|(above, _)| a > *above) {
        Some((_, v)) => *v,
        None => below,
    }
}

impl AgiTables {
    fn key(fs: FilingStatus) -> &'static str {
        match fs {
            FilingStatus::Single => "single",
            FilingStatus::MarriedJoint => "joint",
            FilingStatus::MarriedSeparate => "separate",
            FilingStatus::HeadOfHousehold => "head",
        }
    }

    pub fn exemption(&self, fs: FilingStatus, agi: Money) -> Money {
        let rows = &self.exemption[Self::key(fs)];
        Money::from_cents(step(rows, agi, rows[0].1))
    }

    pub fn tax(&self, fs: FilingStatus, agi: Money, initial: Money) -> Money {
        let k = Self::key(fs);
        let rows = &self.addback[k];
        let add = Money::from_cents(step(rows, agi, rows[0].1));
        let rows = &self.recapture[k];
        let rec = Money::from_cents(step(rows, agi, rows[0].1));
        let credit = step(&self.credit[k], agi, 1.0);
        let total = initial + add + rec;
        total - total.mul_rate(credit)
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct AgiFloorTax {
    pub agi_over: Money,
    pub rate: f64,
}

fn recapture_tax(
    table: &BracketTable,
    rc: &Recapture,
    tops: &[Money],
    agi: Money,
    ti: Money,
) -> Money {
    let sched = table.tax(ti);
    if agi <= rc.agi_over || tops.is_empty() || ti <= Money::ZERO {
        return sched;
    }
    if agi > rc.agi_full {
        let top = table.0.last().map_or(0.0, |b| b.rate);
        return ti.mul_rate(top);
    }
    let one = Money::from_cents(1);
    // The flat rate of tier k: the rate just below its ceiling (the top tier
    // uses the rate that starts at the last ceiling).
    let flat = |k: usize| {
        if k < tops.len() {
            table.marginal_rate(tops[k] - one)
        } else {
            table.marginal_rate(tops[tops.len() - 1])
        }
    };
    let ceiling = |k: usize| tops[k.min(tops.len() - 1)];
    let benefit = |k: usize| ceiling(k).mul_rate(flat(k)) - table.tax(ceiling(k));
    let phase = |over: Money| {
        let f = (over.cents() as f64 / rc.phase_width.cents() as f64).clamp(0.0, 1.0);
        (f * 10_000.0).round() / 10_000.0
    };
    let k = tops.iter().position(|&t| ti <= t).unwrap_or(tops.len());
    if k == 0 {
        let full = (ti.mul_rate(flat(0)) - sched).max(Money::ZERO);
        return sched + full.mul_rate(phase(agi - rc.agi_over));
    }
    let base = benefit(k - 1);
    let incr = benefit(k) - base;
    sched + base + incr.mul_rate(phase(agi - tops[k - 1]))
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
        if let Some(t) = &r.agi_tables {
            // Connecticut: no standard deduction; the exemption steps down with AGI.
            let taxable = (income - t.exemption(fs, income)).max(Money::ZERO);
            let table = match (fs, joint, &r.head_brackets, &r.joint_brackets) {
                (FilingStatus::HeadOfHousehold, _, Some(h), _) => h,
                (_, true, _, Some(j)) => j,
                _ => &r.brackets,
            };
            return Some(t.tax(fs, income, table.tax(taxable)).max(Money::ZERO));
        }
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
        let mut tax = match &r.recapture {
            Some(rc) => {
                let tops = if joint {
                    &rc.tops_joint
                } else {
                    &rc.tops_single
                };
                recapture_tax(&table, rc, tops, income, taxable)
            }
            None => table.tax(taxable),
        };
        if let Some(f) = r.agi_floor_tax {
            if income > f.agi_over {
                tax = tax.max(income.mul_rate(f.rate));
            }
        }
        Some((tax - credit).max(Money::ZERO))
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
        for c in ["AR", "MD", "MN", "NY", "VT"] {
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

    // Official 2025 NY IT-201-I schedules, to check the recapture against the
    // worksheets' own constants.
    fn ny_2025(joint: bool) -> (BracketTable, Recapture) {
        let t = |v: &[(i64, f64)]| {
            BracketTable::new(
                v.iter()
                    .map(|&(d, r)| Bracket {
                        floor: m(d),
                        rate: r,
                    })
                    .collect(),
            )
        };
        let table = if joint {
            t(&[
                (0, 0.04),
                (17_150, 0.045),
                (23_600, 0.0525),
                (27_900, 0.055),
                (161_550, 0.06),
                (323_200, 0.0685),
                (2_155_350, 0.0965),
                (5_000_000, 0.103),
                (25_000_000, 0.109),
            ])
        } else {
            t(&[
                (0, 0.04),
                (8_500, 0.045),
                (11_700, 0.0525),
                (13_900, 0.055),
                (80_650, 0.06),
                (215_400, 0.0685),
                (1_077_550, 0.0965),
                (5_000_000, 0.103),
                (25_000_000, 0.109),
            ])
        };
        let rc = Recapture {
            agi_over: m(107_650),
            agi_full: m(25_000_000),
            phase_width: m(50_000),
            tops_single: vec![m(215_400), m(1_077_550), m(5_000_000)],
            tops_joint: vec![m(161_550), m(323_200), m(2_155_350), m(5_000_000)],
        };
        (table, rc)
    }

    fn near(got: Money, want_dollars: f64) {
        let diff = (got.cents() as f64 / 100.0 - want_dollars).abs();
        assert!(diff <= 1.0, "got {got:?}, want {want_dollars}");
    }

    #[test]
    fn ny_recapture_matches_the_2025_worksheets() {
        let (jt, rc) = ny_2025(true);
        let (st, _) = ny_2025(false);
        let j = |agi, ti| recapture_tax(&jt, &rc, &rc.tops_joint, m(agi), m(ti));
        let s = |agi, ti| recapture_tax(&st, &rc, &rc.tops_single, m(agi), m(ti));
        // Below the AGI threshold: the plain schedule.
        near(j(100_000, 80_000), 4_067.5);
        // Worksheet 1: 5.5% against the schedule, phased over $50,000 of AGI.
        near(j(120_000, 100_000), 5_249.6);
        near(j(157_650, 100_000), 5_500.0);
        // Worksheet 2, partly and fully phased in.
        near(j(180_000, 170_000), 9_690.8);
        near(j(230_000, 200_000), 12_000.0);
        // Worksheets 3 and 4 fully phased: the tier's flat rate on all income.
        near(j(900_000, 500_000), 0.0685 * 500_000.0);
        near(j(3_000_000, 2_500_000), 0.0965 * 2_500_000.0);
        near(j(9_000_000, 6_000_000), 0.103 * 6_000_000.0);
        // Worksheet 6: over $25,000,000 AGI, 10.9% on everything.
        near(j(30_000_000, 8_000_000), 0.109 * 8_000_000.0);
        // Single worksheets 7 and 8.
        near(s(125_000, 100_000), 5_629.1);
        near(s(400_000, 300_000), 20_550.0);
        // Constants printed on the worksheets: base 568, incremental 1,831.
        let at_top = s(1_000_000, 215_400);
        near(at_top, 0.06 * 215_400.0);
    }

    #[test]
    fn ny_2026_data_applies_the_recapture() {
        let y = StateTaxYear::y2026();
        let rules = y.rules("NY").unwrap();
        assert!(rules.recapture.is_some());
        // Single, AGI $300,000: past the phase-in, so 6.85% on the $292,000
        // of taxable income after the $8,000 standard deduction.
        let t = y
            .income_tax("NY", FilingStatus::Single, m(300_000))
            .unwrap();
        near(t, 0.0685 * 292_000.0);
        // Under the threshold nothing changes.
        let low = y.income_tax("NY", FilingStatus::Single, m(60_000)).unwrap();
        assert!(low < m(3_000));
    }

    #[test]
    fn vermont_charges_at_least_three_percent_of_agi_over_150k() {
        let json = r#"{"year":2026,"source":"t","states":{"XX":{"name":"X","kind":"progressive",
            "brackets":[[0,0.01]],"std_single":0,"std_joint":0,"joint_multiplier":1.0,
            "approximate":false,"notes":"","agi_floor_tax":{"agi_over":15000000,"rate":0.03}}}}"#;
        let y = StateTaxYear::from_json(json).unwrap();
        assert_eq!(
            y.income_tax("XX", FilingStatus::Single, m(200_000)),
            Some(m(6_000))
        );
        assert_eq!(
            y.income_tax("XX", FilingStatus::Single, m(100_000)),
            Some(m(1_000))
        );
        let vt = StateTaxYear::y2026();
        assert!(vt.rules("VT").unwrap().agi_floor_tax.is_some());
    }

    // Connecticut, checked by hand against the CT-1040 TCS tables and its worked examples.
    fn ct(fs: FilingStatus, agi: i64) -> Money {
        StateTaxYear::y2026().income_tax("CT", fs, m(agi)).unwrap()
    }

    #[test]
    fn connecticut_is_no_longer_an_estimate() {
        assert!(!StateTaxYear::y2026().rules("CT").unwrap().approximate);
    }

    #[test]
    fn connecticut_low_income_pays_nothing() {
        assert_eq!(ct(FilingStatus::Single, 10_000), Money::ZERO);
        assert_eq!(ct(FilingStatus::MarriedJoint, 20_000), Money::ZERO);
    }

    #[test]
    fn connecticut_single_60k_uses_addback_and_credit() {
        // 60,000 taxable: 2,550 + 25 add-back, less the 10% credit.
        assert_eq!(ct(FilingStatus::Single, 60_000), Money::from_cents(231_750));
    }

    #[test]
    fn connecticut_joint_over_a_million_adds_recapture_and_loses_the_credit() {
        // Worked example in the form: 69,490, plus 500 add-back and 6,800 recapture.
        assert_eq!(ct(FilingStatus::MarriedJoint, 1_100_000), m(76_790));
    }

    #[test]
    fn connecticut_head_of_household_has_its_own_brackets_and_exemption() {
        // AGI 39,000: exemption 18,000, taxable 21,000 -> 320 + 4.5% x 5,000 = 545; credit 15%.
        assert_eq!(
            ct(FilingStatus::HeadOfHousehold, 39_000),
            Money::from_cents(46_325)
        );
    }

    #[test]
    fn connecticut_tax_rises_with_income_in_broad_strokes() {
        let mut last = Money::ZERO;
        for agi in (0..=1_200_000).step_by(50_000) {
            let t = ct(FilingStatus::Single, agi);
            assert!(t + m(300) >= last, "{agi}: {t:?} < {last:?}");
            last = t;
        }
        let high = ct(FilingStatus::Single, 600_000);
        assert!(high > m(35_000) && high < m(42_000), "{high:?}");
    }
}
