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

/// A standard deduction that falls in a straight line from `std` at `start` income to `floor`
/// at `end` income (the state's own table steps; the line follows it closely).
#[derive(Debug, Clone, Deserialize, Default)]
pub struct StdPhaseDown {
    pub start_single: Money,
    pub end_single: Money,
    pub floor_single: Money,
    pub start_joint: Money,
    pub end_joint: Money,
    pub floor_joint: Money,
}

impl StdPhaseDown {
    fn apply(&self, joint: bool, std: Money, income: Money) -> Money {
        let (start, end, floor) = if joint {
            (self.start_joint, self.end_joint, self.floor_joint)
        } else {
            (self.start_single, self.end_single, self.floor_single)
        };
        if end <= start || income <= start {
            return std;
        }
        if income >= end {
            return floor.min(std);
        }
        let share = (income - start).cents() as f64 / (end - start).cents() as f64;
        std - (std - floor.min(std)).mul_rate(share)
    }
}

/// How one state taxes ordinary income.
#[derive(Debug, Clone, Deserialize)]
pub struct StateRules {
    pub name: String,
    pub kind: StateKind,
    pub brackets: BracketTable,
    pub std_single: Money,
    pub std_joint: Money,
    /// The standard deduction shrinks as income rises (Alabama).
    #[serde(default)]
    pub std_phase_down: Option<StdPhaseDown>,
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
    /// True for the few states that tax Social Security benefits (Colorado, Connecticut,
    /// Minnesota, Montana, New Mexico, Rhode Island, Utah, Vermont, each only partly or above an
    /// income level; the full federally taxable amount is used here, which errs high). All other
    /// states leave benefits out of state income.
    #[serde(default)]
    pub taxes_social_security: bool,
    /// What the state leaves out of income for pensions and withdrawals from pre-tax accounts.
    #[serde(default)]
    pub retirement: Option<RetirementExclusion>,
    /// An age-based deduction on all income, reduced dollar for dollar above an income line
    /// (Virginia).
    #[serde(default)]
    pub age_deduction: Option<AgeDeduction>,
    /// An exemption for each person of an age whose amount steps down as income rises (New Mexico).
    #[serde(default)]
    pub age_exemption: Option<AgeExemptionSteps>,
    /// Allowance for people of an age, cut by the Social Security left out (West Virginia).
    #[serde(default)]
    pub age_allowance: Option<AgeAllowance>,
    /// Credits against tax for retirement income and age (Ohio).
    #[serde(default)]
    pub retirement_credit: Option<RetirementCredit>,
    /// For states in `taxes_social_security`: when benefits are left out of state income.
    #[serde(default)]
    pub social_security: Option<SocialSecurityRule>,
    /// True when the schedule is simplified; the UI labels these as estimates.
    pub approximate: bool,
    pub notes: String,
}

/// A state's exemption for retirement income (pensions, and IRA/401(k) distributions).
#[derive(Debug, Clone, Deserialize, Default)]
pub struct RetirementExclusion {
    /// Pensions are exempt in full once someone in the household reaches `min_age`.
    #[serde(default)]
    pub pension_full: bool,
    /// Pre-tax account distributions are exempt in full once someone reaches `min_age`.
    #[serde(default)]
    pub distributions_full: bool,
    #[serde(default)]
    pub min_age: u8,
    /// Per-person ceilings on the combined pension and distribution exclusion, by age: each person
    /// gets the cap of the highest `age` they have reached.
    #[serde(default)]
    pub caps: Vec<AgeCap>,
    /// The caps cover pension income only, not IRA and 401(k) distributions (Maryland).
    #[serde(default)]
    pub pension_only: bool,
    /// The caps are reduced by the Social Security benefits received (Maryland).
    #[serde(default)]
    pub less_social_security: bool,
    /// The caps are reduced by the Social Security the state left out of income (Colorado).
    #[serde(default)]
    pub less_ss_subtracted: bool,
    /// The caps shrink once federal AGI passes a line (Maine).
    #[serde(default)]
    pub phaseout: Option<ExclusionPhaseout>,
    /// A household ceiling that depends on filing status and shrinks as income rises (New Jersey).
    #[serde(default)]
    pub household: Option<HouseholdCap>,
}

/// The exclusion falls to nothing over `width` of federal AGI above the start for the filing status.
#[derive(Debug, Clone, Deserialize)]
pub struct ExclusionPhaseout {
    pub start_single: Money,
    pub start_joint: Money,
    pub width: Money,
    /// Instead of fading over `width`, the exclusion falls a dollar for each dollar of AGI (less
    /// taxable Social Security) over the start (Missouri).
    #[serde(default)]
    pub dollar: bool,
}

/// New Jersey style: once someone in the household is `min_age`, pension and IRA income up to the
/// filing-status ceiling is left out while income is at or under `full_to`; above it only a
/// share (by filing status) is, down to nothing over the last step.
#[derive(Debug, Clone, Deserialize)]
pub struct HouseholdCap {
    pub min_age: u8,
    pub cap_single: Money,
    pub cap_joint: Money,
    pub cap_separate: Money,
    pub full_to: Money,
    pub steps: Vec<IncomeStep>,
    /// Only pensions count, not withdrawals from IRAs and 401(k)s (Rhode Island).
    #[serde(default)]
    pub pension_only: bool,
    /// Nothing is left out when federal AGI is over this (single, others); 0 means no test.
    #[serde(default)]
    pub agi_limit_single: Money,
    #[serde(default)]
    pub agi_limit_joint: Money,
}

/// Income up to `to` leaves out these shares of the pension and withdrawals.
#[derive(Debug, Clone, Deserialize)]
pub struct IncomeStep {
    pub to: Money,
    pub single: f64,
    pub joint: f64,
    pub separate: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgeCap {
    pub age: u8,
    pub cap: Money,
}

/// A deduction each person of `age` or older may take, cut by $1 for every $1 that income
/// (federal AGI less taxable Social Security) is over the line.
#[derive(Debug, Clone, Deserialize)]
pub struct AgeDeduction {
    pub age: u8,
    pub amount: Money,
    pub phase_start_single: Money,
    pub phase_start_joint: Money,
}

impl AgeDeduction {
    pub fn total(&self, fs: FilingStatus, income: Money, ages: &[i32]) -> Money {
        let start = if fs == FilingStatus::MarriedJoint {
            self.phase_start_joint
        } else {
            self.phase_start_single
        };
        let each = (self.amount - (income - start).max(Money::ZERO)).max(Money::ZERO);
        let n = ages.iter().filter(|&&a| a >= self.age as i32).count() as i64;
        Money::from_cents(each.cents() * n)
    }
}

/// Each person of `age` or older may leave out up to `amount` of income, less their share of the
/// Social Security the state already left out (West Virginia).
#[derive(Debug, Clone, Deserialize)]
pub struct AgeAllowance {
    pub age: u8,
    pub amount: Money,
}

impl AgeAllowance {
    pub fn total(&self, ss_subtracted: Money, ages: &[i32]) -> Money {
        let n = ages.len().max(1) as i64;
        let share = ss_subtracted.max(Money::ZERO).cents() / n;
        let each = (self.amount.cents() - share).max(0);
        let eligible = ages.iter().filter(|&&a| a >= self.age as i32).count() as i64;
        Money::from_cents(each * eligible)
    }
}

/// Ohio style credits against tax for people with retirement income and modest income.
#[derive(Debug, Clone, Deserialize)]
pub struct RetirementCredit {
    /// No credit when income (AGI less taxable Social Security) is this much or more.
    pub income_limit: Money,
    /// (retirement income up to, credit), in order; the last row catches everything above.
    pub steps: Vec<(Money, Money)>,
    /// A flat credit for the return when someone is `senior_age` or older.
    #[serde(default)]
    pub senior_age: u8,
    #[serde(default)]
    pub senior_credit: Money,
}

impl RetirementCredit {
    pub fn total(&self, income: Money, retirement: Money, ages: &[i32]) -> Money {
        if income >= self.income_limit {
            return Money::ZERO;
        }
        let r = retirement.max(Money::ZERO);
        let base = self
            .steps
            .iter()
            .find(|(to, _)| r <= *to)
            .or(self.steps.last())
            .map(|(_, c)| *c)
            .unwrap_or(Money::ZERO);
        let senior = if self.senior_age > 0 && ages.iter().any(|&a| a >= self.senior_age as i32) {
            self.senior_credit
        } else {
            Money::ZERO
        };
        base + senior
    }
}

/// Each person of `age` or older gets `amount`, less `step` for every `width` of federal AGI over
/// the start for the filing status (New Mexico's table: $8,000 down to nothing).
#[derive(Debug, Clone, Deserialize)]
pub struct AgeExemptionSteps {
    pub age: u8,
    pub amount: Money,
    pub step: Money,
    pub start_single: Money,
    pub width_single: Money,
    pub start_joint: Money,
    pub width_joint: Money,
    pub start_separate: Money,
    pub width_separate: Money,
}

impl AgeExemptionSteps {
    pub fn total(&self, fs: FilingStatus, agi: Money, ages: &[i32]) -> Money {
        let (start, width) = match fs {
            FilingStatus::MarriedJoint | FilingStatus::HeadOfHousehold => {
                (self.start_joint, self.width_joint)
            }
            FilingStatus::MarriedSeparate => (self.start_separate, self.width_separate),
            _ => (self.start_single, self.width_single),
        };
        let over = (agi - start).max(Money::ZERO).cents();
        let steps = if over == 0 || width.cents() <= 0 {
            0
        } else {
            (over + width.cents() - 1) / width.cents()
        };
        let each = (self.amount.cents() - steps * self.step.cents()).max(0);
        let n = ages.iter().filter(|&&a| a >= self.age as i32).count() as i64;
        Money::from_cents(each * n)
    }
}

/// How a state that taxes Social Security leaves part of it out. Without a rule the whole
/// federally taxable amount is used (which errs high).
#[derive(Debug, Clone, Deserialize, Default)]
pub struct SocialSecurityRule {
    /// Benefits are left out entirely while federal AGI is at or below these amounts.
    #[serde(default)]
    pub exempt_agi_single: Money,
    #[serde(default)]
    pub exempt_agi_joint: Money,
    /// Per-person subtraction by age (Colorado).
    #[serde(default)]
    pub caps: Vec<AgeCap>,
    /// A flat deduction from the taxable benefits (Montana).
    #[serde(default)]
    pub deduction_single: Money,
    #[serde(default)]
    pub deduction_joint: Money,
    /// People this old leave all their benefits out, whatever the income; younger people use
    /// `caps` once income is over the `exempt_agi` line (Colorado).
    #[serde(default)]
    pub full_age: u8,
    /// Benefits are left out under the AGI line only for people this old; younger people's share
    /// stays in (Rhode Island: full retirement age).
    #[serde(default)]
    pub exempt_min_age: u8,
    /// A credit of this share of the taxable benefits, cut by `credit_phaseout_rate` of the
    /// income over the `exempt_agi` line (Utah). The benefits stay in state income.
    #[serde(default)]
    pub credit_rate: f64,
    #[serde(default)]
    pub credit_phaseout_rate: f64,
    /// Over the `exempt_agi` line the exempt share falls in a straight line to nothing across
    /// this much more income (Vermont: $10,000; Minnesota: 10% per $4,000, so $40,000).
    #[serde(default)]
    pub phaseout_width: Money,
}

impl SocialSecurityRule {
    /// The part of `taxable_ss` that stays in state income.
    pub fn taxed(&self, fs: FilingStatus, agi: Money, taxable_ss: Money, ages: &[i32]) -> Money {
        let joint = fs == FilingStatus::MarriedJoint;
        let limit = if joint {
            self.exempt_agi_joint
        } else {
            self.exempt_agi_single
        };
        if self.credit_rate > 0.0 {
            return taxable_ss;
        }
        if self.full_age > 0 {
            let n = ages.len().max(1) as i64;
            let share = Money::from_cents(taxable_ss.cents() / n);
            let each = |a: i32| -> Money {
                if a >= self.full_age as i32 {
                    return Money::ZERO;
                }
                let cap = self
                    .caps
                    .iter()
                    .filter(|c| a >= c.age as i32)
                    .max_by_key(|c| c.age)
                    .map(|c| c.cap);
                match cap {
                    Some(_) if limit > Money::ZERO && agi <= limit => Money::ZERO,
                    Some(c) => (share - c).max(Money::ZERO),
                    None => share,
                }
            };
            if ages.is_empty() {
                return taxable_ss;
            }
            return ages
                .iter()
                .map(|&a| each(a))
                .fold(Money::ZERO, |x, y| x + y);
        }
        if limit > Money::ZERO && agi <= limit {
            if self.exempt_min_age == 0 || ages.is_empty() {
                return Money::ZERO;
            }
            let ok = ages
                .iter()
                .filter(|&&a| a >= self.exempt_min_age as i32)
                .count() as i64;
            let left = taxable_ss.cents() * (ages.len() as i64 - ok) / ages.len() as i64;
            return Money::from_cents(left);
        }
        if limit > Money::ZERO
            && self.phaseout_width > Money::ZERO
            && agi < limit + self.phaseout_width
        {
            // Part of the benefits is still left out, less the further income is over the line.
            let into = (agi - limit).max(Money::ZERO).cents() as f64;
            let share = into / self.phaseout_width.cents() as f64;
            return taxable_ss.mul_rate(share.clamp(0.0, 1.0));
        }
        let per_person = ages
            .iter()
            .filter_map(|&a| {
                self.caps
                    .iter()
                    .filter(|c| a >= c.age as i32)
                    .max_by_key(|c| c.age)
                    .map(|c| c.cap)
            })
            .fold(Money::ZERO, |a, b| a + b);
        let flat = if joint {
            self.deduction_joint
        } else {
            self.deduction_single
        };
        (taxable_ss - per_person - flat).max(Money::ZERO)
    }

    /// The credit against tax for `taxable_ss` when income is `income` (Utah style).
    pub fn credit(&self, fs: FilingStatus, income: Money, taxable_ss: Money) -> Money {
        if self.credit_rate <= 0.0 {
            return Money::ZERO;
        }
        let limit = if fs == FilingStatus::MarriedJoint {
            self.exempt_agi_joint
        } else {
            self.exempt_agi_single
        };
        let over = (income - limit).max(Money::ZERO);
        (taxable_ss.mul_rate(self.credit_rate) - over.mul_rate(self.credit_phaseout_rate))
            .max(Money::ZERO)
    }
}

/// Retirement income in a year, for the state's exclusion.
#[derive(Debug, Clone, Default)]
pub struct RetirementIncome {
    pub pension: Money,
    pub distributions: Money,
    /// Gross Social Security benefits received, taxable or not.
    pub social_security: Money,
    /// Social Security the state left out of income; set by the tax calculation.
    pub ss_subtracted: Money,
    /// Federal adjusted gross income; set by the tax calculation.
    pub federal_agi: Money,
    /// Ages of the (adult) people on the return.
    pub ages: Vec<i32>,
}

impl RetirementExclusion {
    /// The amount left out of state income.
    pub fn excluded(&self, inc: &RetirementIncome) -> Money {
        let zero = Money::ZERO;
        let (pension, dist) = (inc.pension.max(zero), inc.distributions.max(zero));
        let eligible = inc.ages.iter().any(|&a| a >= self.min_age as i32);
        let mut out = zero;
        let mut left_p = pension;
        let mut left_d = dist;
        if eligible {
            if self.pension_full {
                out += left_p;
                left_p = zero;
            }
            if self.distributions_full {
                out += left_d;
                left_d = zero;
            }
        }
        let mut cap: Money = inc
            .ages
            .iter()
            .filter_map(|&a| {
                self.caps
                    .iter()
                    .filter(|c| a >= c.age as i32)
                    .max_by_key(|c| c.age)
                    .map(|c| c.cap)
            })
            .fold(zero, |a, b| a + b);
        if self.less_social_security {
            cap = (cap - inc.social_security.max(zero)).max(zero);
        }
        if self.less_ss_subtracted {
            cap = (cap - inc.ss_subtracted.max(zero)).max(zero);
        }
        let pool = if self.pension_only {
            left_p
        } else {
            left_p + left_d
        };
        out + cap.min(pool)
    }

    /// The household-level exclusion for a return with `income` (before the exclusion).
    pub fn household_excluded(
        &self,
        fs: FilingStatus,
        income: Money,
        inc: &RetirementIncome,
    ) -> Money {
        let Some(h) = &self.household else {
            return Money::ZERO;
        };
        if !inc.ages.iter().any(|&a| a >= h.min_age as i32) {
            return Money::ZERO;
        }
        let (cap, pick): (Money, fn(&IncomeStep) -> f64) = match fs {
            FilingStatus::MarriedJoint => (h.cap_joint, |s| s.joint),
            FilingStatus::MarriedSeparate => (h.cap_separate, |s| s.separate),
            _ => (h.cap_single, |s| s.single),
        };
        let limit = if fs == FilingStatus::MarriedJoint {
            h.agi_limit_joint
        } else {
            h.agi_limit_single
        };
        if limit > Money::ZERO && inc.federal_agi > limit {
            return Money::ZERO;
        }
        let dist = if h.pension_only {
            Money::ZERO
        } else {
            inc.distributions.max(Money::ZERO)
        };
        let base = (inc.pension.max(Money::ZERO) + dist).min(cap);
        if income <= h.full_to {
            return base;
        }
        match h.steps.iter().find(|s| income <= s.to) {
            Some(step) => base.mul_rate(pick(step)),
            None => Money::ZERO,
        }
    }
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

    /// State income tax for a resident of `code` on federal AGI that includes `taxable_ss`, the
    /// federally taxable part of Social Security. States that do not tax benefits leave it out.
    pub fn income_tax_with_ss(
        &self,
        code: &str,
        fs: FilingStatus,
        federal_agi: Money,
        taxable_ss: Money,
    ) -> Option<Money> {
        let r = self.rules(code)?;
        let income = if r.taxes_social_security {
            federal_agi
        } else {
            (federal_agi - taxable_ss.max(Money::ZERO)).max(Money::ZERO)
        };
        self.income_tax(code, fs, income)
    }

    /// As `income_tax_with_ss`, also leaving out the state's pension and retirement account
    /// exclusions.
    pub fn income_tax_with_retirement(
        &self,
        code: &str,
        fs: FilingStatus,
        federal_agi: Money,
        taxable_ss: Money,
        ret: &RetirementIncome,
    ) -> Option<Money> {
        let r = self.rules(code)?;
        let ss = taxable_ss.max(Money::ZERO);
        let ss_in_state = match (&r.social_security, r.taxes_social_security) {
            (_, false) => Money::ZERO,
            (Some(rule), true) => rule.taxed(fs, federal_agi, ss, &ret.ages),
            (None, true) => ss,
        };
        let mut income = (federal_agi - ss + ss_in_state).max(Money::ZERO);
        if let Some(a) = &r.age_deduction {
            let afagi = (federal_agi - ss).max(Money::ZERO);
            income = (income - a.total(fs, afagi, &ret.ages)).max(Money::ZERO);
        }
        if let Some(a) = &r.age_exemption {
            income = (income - a.total(fs, federal_agi, &ret.ages)).max(Money::ZERO);
        }
        let before_ss = (federal_agi - ss).max(Money::ZERO);
        if let Some(a) = &r.age_allowance {
            let sub = (ss - ss_in_state).max(Money::ZERO);
            income = (income - a.total(sub, &ret.ages)).max(Money::ZERO);
        }
        if let Some(x) = &r.retirement {
            let before = (federal_agi - ss).max(Money::ZERO);
            let mut ret = ret.clone();
            ret.ss_subtracted = (ss - ss_in_state).max(Money::ZERO);
            ret.federal_agi = federal_agi;
            let mut cut = x.excluded(&ret);
            if let Some(p) = &x.phaseout {
                let start = if fs == FilingStatus::MarriedJoint {
                    p.start_joint
                } else {
                    p.start_single
                };
                if p.dollar {
                    let over = (before - start).max(Money::ZERO);
                    cut = (cut - over).max(Money::ZERO);
                } else {
                    let over = (federal_agi - start).max(Money::ZERO).cents() as f64;
                    let share = (over / p.width.cents().max(1) as f64).min(1.0);
                    cut = cut.mul_rate(1.0 - share);
                }
            }
            income = (income - cut - x.household_excluded(fs, before, &ret)).max(Money::ZERO);
        }
        let tax = self.income_tax(code, fs, income)?;
        let credit = match (&r.social_security, r.taxes_social_security) {
            (Some(rule), true) => rule.credit(fs, income, ss),
            _ => Money::ZERO,
        };
        let rc = r.retirement_credit.as_ref().map_or(Money::ZERO, |c| {
            c.total(
                before_ss,
                ret.pension.max(Money::ZERO) + ret.distributions.max(Money::ZERO),
                &ret.ages,
            )
        });
        Some((tax - credit - rc).max(Money::ZERO))
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
        let std = if joint { r.std_joint } else { r.std_single };
        let std = match &r.std_phase_down {
            Some(p) => p.apply(joint, std, income),
            None => std,
        };
        let taxable = (income - std).max(Money::ZERO);
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

    fn ret(pension: i64, dist: i64, ages: &[i32]) -> RetirementIncome {
        RetirementIncome {
            pension: m(pension),
            distributions: m(dist),
            social_security: Money::ZERO,
            ss_subtracted: Money::ZERO,
            federal_agi: Money::ZERO,
            ages: ages.to_vec(),
        }
    }

    fn tax(c: &str, agi: i64, r: &RetirementIncome) -> Money {
        StateTaxYear::y2026()
            .income_tax_with_retirement(c, FilingStatus::Single, m(agi), Money::ZERO, r)
            .unwrap()
    }

    #[test]
    fn flat_states_take_their_standard_deduction_or_exemption() {
        // (state, single, joint): North Carolina's standard deduction, Georgia's personal
        // exemption, Mississippi's exemption plus standard deduction, Louisiana's 2025 standard
        // deduction, Indiana's $1,000 exemption, Colorado's start from federal taxable income and
        // Massachusetts' personal exemption.
        let y = StateTaxYear::y2026();
        for (code, single, joint) in [
            ("NC", 12_750, 25_500),
            ("GA", 12_000, 24_000),
            ("MS", 8_300, 16_600),
            ("LA", 12_500, 25_000),
            ("IN", 1_000, 2_000),
            ("CO", 16_100, 32_200),
            ("MA", 4_400, 8_800),
        ] {
            let rate = y.rules(code).unwrap().brackets.0[0].rate;
            for (fs, d) in [
                (FilingStatus::Single, single),
                (FilingStatus::MarriedJoint, joint),
            ] {
                assert_eq!(
                    y.income_tax(code, fs, m(d)),
                    Some(Money::ZERO),
                    "{code} {fs:?}"
                );
                assert_eq!(
                    y.income_tax(code, fs, m(d + 10_000)),
                    Some(m(10_000).mul_rate(rate)),
                    "{code} {fs:?}"
                );
            }
        }
    }

    #[test]
    fn illinois_and_pennsylvania_do_not_tax_retirement_income() {
        let r = ret(20_000, 30_000, &[70]);
        for c in ["IL", "PA", "MS"] {
            assert_eq!(tax(c, 50_000, &r), Money::ZERO, "{c}");
        }
        // Ordinary income of the same size is taxed.
        assert!(tax("IL", 50_000, &ret(0, 0, &[70])) > Money::ZERO);
    }

    #[test]
    fn age_gates_the_full_exemptions() {
        let r = ret(0, 40_000, &[50]);
        assert_eq!(
            tax("IL", 40_000, &r),
            Money::ZERO,
            "Illinois has no age test"
        );
        assert!(tax("PA", 40_000, &r) > Money::ZERO, "too young for PA");
        assert!(tax("IA", 40_000, &r) > Money::ZERO, "too young for IA");
        assert_eq!(tax("IA", 40_000, &ret(0, 40_000, &[55])), Money::ZERO);
    }

    #[test]
    fn capped_states_exclude_up_to_the_cap_per_person() {
        let y = StateTaxYear::y2026();
        let x = |c: &str| y.rules(c).unwrap().retirement.clone().unwrap();
        // Kentucky: $31,110 each.
        assert_eq!(x("KY").excluded(&ret(10_000, 50_000, &[66])), m(31_110));
        assert_eq!(x("KY").excluded(&ret(10_000, 50_000, &[66, 64])), m(60_000));
        assert_eq!(x("KY").excluded(&ret(5_000, 0, &[66])), m(5_000));
        // New York and Delaware need 60.
        assert_eq!(x("NY").excluded(&ret(30_000, 0, &[58])), Money::ZERO);
        assert_eq!(x("NY").excluded(&ret(30_000, 0, &[60])), m(20_000));
        assert_eq!(x("DE").excluded(&ret(0, 20_000, &[61])), m(12_500));
        // Georgia steps up at 65; South Carolina starts at 65.
        assert_eq!(x("GA").excluded(&ret(0, 100_000, &[63])), m(35_000));
        assert_eq!(x("GA").excluded(&ret(0, 100_000, &[65])), m(65_000));
        assert_eq!(x("SC").excluded(&ret(0, 20_000, &[64])), Money::ZERO);
        assert_eq!(x("SC").excluded(&ret(0, 20_000, &[65])), m(10_000));
    }

    #[test]
    fn social_security_states_leave_benefits_out_below_their_thresholds() {
        let y = StateTaxYear::y2026();
        let ss = |c: &str, fs, agi: i64, ben: i64, ages: &[i32]| {
            y.income_tax_with_retirement(c, fs, m(agi), m(ben), &ret(0, 0, ages))
                .unwrap()
        };
        let single = FilingStatus::Single;
        // Connecticut: $70,000 of AGI of which $20,000 is benefits is under the $75,000 line.
        let ct_without = y.income_tax("CT", single, m(50_000)).unwrap();
        assert_eq!(ss("CT", single, 70_000, 20_000, &[70]), ct_without);
        // Over the line the benefits are taxed again.
        assert!(
            ss("CT", single, 80_000, 20_000, &[70])
                > y.income_tax("CT", single, m(60_000)).unwrap()
        );
        // Joint thresholds are higher.
        assert!(
            ss("NM", FilingStatus::MarriedJoint, 120_000, 30_000, &[70, 68])
                == y.income_tax("NM", FilingStatus::MarriedJoint, m(90_000))
                    .unwrap()
        );
    }

    #[test]
    fn vermont_and_minnesota_phase_the_benefit_exemption_out_over_the_line() {
        let y = StateTaxYear::y2026();
        let single = FilingStatus::Single;
        let go = |c: &str, agi: i64, ben: i64| {
            y.income_tax_with_retirement(c, single, m(agi), m(ben), &ret(0, 0, &[70]))
                .unwrap()
        };
        let plain = |c: &str, inc: i64| y.income_tax(c, single, m(inc)).unwrap();
        // Vermont: all of it out under $55,000, half at $60,000, none from $65,000.
        assert_eq!(go("VT", 55_000, 20_000), plain("VT", 35_000));
        assert_eq!(go("VT", 60_000, 20_000), plain("VT", 50_000));
        assert_eq!(go("VT", 65_000, 20_000), plain("VT", 65_000));
        // Minnesota: 10% less is left out for each $4,000 over $84,490.
        assert_eq!(go("MN", 84_490, 20_000), plain("MN", 64_490));
        assert_eq!(go("MN", 104_490, 20_000), plain("MN", 94_490));
        assert_eq!(go("MN", 124_490, 20_000), plain("MN", 124_490));
    }

    #[test]
    fn colorado_and_montana_subtract_part_of_the_benefits() {
        let y = StateTaxYear::y2026();
        let t = |c: &str, fs, ben: i64, ages: &[i32]| {
            y.income_tax_with_retirement(c, fs, m(100_000), m(ben), &ret(0, 0, ages))
                .unwrap()
        };
        // Colorado: at 65 or older all of the benefits are left out.
        let co = y
            .income_tax("CO", FilingStatus::Single, m(100_000 - 30_000))
            .unwrap();
        assert_eq!(t("CO", FilingStatus::Single, 30_000, &[66]), co);
        // Montana's 2025 Form 2 has no Social Security subtraction (only Tier I railroad
        // benefits), so the federally taxable amount stays in.
        let mt = y
            .income_tax("MT", FilingStatus::Single, m(100_000))
            .unwrap();
        assert_eq!(t("MT", FilingStatus::Single, 30_000, &[66]), mt);
    }

    #[test]
    fn colorado_at_55_to_64_depends_on_income_and_pensions_share_the_cap() {
        let y = StateTaxYear::y2026();
        let t = |fs, agi: i64, ben: i64, pen: i64, ages: &[i32]| {
            y.income_tax_with_retirement("CO", fs, m(agi), m(ben), &ret(pen, 0, ages))
                .unwrap()
        };
        let plain = |fs, inc: i64| y.income_tax("CO", fs, m(inc)).unwrap();
        let single = FilingStatus::Single;
        // 60: at $75,000 or less every benefit dollar goes; over it only $20,000 does.
        assert_eq!(t(single, 75_000, 30_000, 0, &[60]), plain(single, 45_000));
        assert_eq!(t(single, 80_000, 30_000, 0, &[60]), plain(single, 60_000));
        // Joint limit is $95,000.
        let joint = FilingStatus::MarriedJoint;
        assert_eq!(t(joint, 95_000, 30_000, 0, &[60, 58]), plain(joint, 65_000));
        // A pension at 66 with no benefits: $24,000 comes out.
        assert_eq!(t(single, 100_000, 0, 50_000, &[66]), plain(single, 76_000));
        // At 60: $20,000.
        assert_eq!(t(single, 100_000, 0, 50_000, &[60]), plain(single, 80_000));
        // Benefits subtracted use up the pension cap: $10,000 of benefits at 66 leaves $14,000.
        assert_eq!(
            t(single, 100_000, 10_000, 50_000, &[66]),
            plain(single, 100_000 - 10_000 - 14_000)
        );
        // Benefits of $24,000 or more leave no pension subtraction.
        assert_eq!(
            t(single, 100_000, 30_000, 50_000, &[66]),
            plain(single, 70_000)
        );
    }

    #[test]
    fn utah_gives_a_credit_for_benefits_that_fades_with_income() {
        let y = StateTaxYear::y2026();
        let single = FilingStatus::Single;
        let go = |agi: i64, ben: i64| {
            y.income_tax_with_retirement("UT", single, m(agi), m(ben), &ret(0, 0, &[70]))
                .unwrap()
        };
        let plain = |inc: i64| y.income_tax("UT", single, m(inc)).unwrap();
        // Under the line the credit is 4.5% of the benefits.
        assert_eq!(go(50_000, 20_000), plain(50_000) - m(900));
        // Over it, 2.5% of the excess is taken back: $64,000 is $10,000 over.
        assert_eq!(go(64_000, 20_000), plain(64_000) - m(900) + m(250));
        // Never more than the tax itself.
        assert_eq!(go(10_000, 10_000), Money::ZERO);
        // No benefits, no credit.
        assert_eq!(go(50_000, 0), plain(50_000));
    }

    #[test]
    fn new_mexico_age_exemption_steps_down_with_income() {
        let y = StateTaxYear::y2026();
        let single = FilingStatus::Single;
        let go = |fs, agi: i64, ages: &[i32]| {
            y.income_tax_with_retirement("NM", fs, m(agi), Money::ZERO, &ret(0, 0, ages))
                .unwrap()
        };
        let plain = |fs, inc: i64| y.income_tax("NM", fs, m(inc)).unwrap();
        // Single at 70: $8,000 up to $18,000, $7,000 to $19,500, nothing past $28,500.
        assert_eq!(go(single, 18_000, &[70]), plain(single, 10_000));
        assert_eq!(go(single, 19_500, &[70]), plain(single, 12_500));
        assert_eq!(go(single, 28_500, &[70]), plain(single, 27_500));
        assert_eq!(go(single, 28_501, &[70]), plain(single, 28_501));
        // Joint, both 65 or older, AGI $35,000: $6,000 each (the form's example is $12,000).
        let joint = FilingStatus::MarriedJoint;
        assert_eq!(go(joint, 35_000, &[66, 68]), plain(joint, 23_000));
        // Only the person at 65 gets it.
        assert_eq!(go(joint, 28_000, &[66, 60]), plain(joint, 20_000));
    }

    #[test]
    fn flat_tax_states_take_their_deduction_or_exemption_first() {
        let y = StateTaxYear::y2026();
        let single = FilingStatus::Single;
        // Illinois: 4.95% after the $2,850 exemption.
        assert_eq!(
            y.income_tax("IL", single, m(52_850)).unwrap(),
            m(50_000).mul_rate(0.0495)
        );
        // Arizona and Idaho follow the federal standard deduction, Kentucky has its own.
        assert!(
            y.income_tax("AZ", single, m(60_000)).unwrap() < m(60_000).mul_rate(0.025) - m(300)
        );
        assert!(y.income_tax("ID", single, m(60_000)).unwrap() < m(60_000).mul_rate(0.053));
        assert_eq!(
            y.income_tax("KY", single, m(53_270)).unwrap(),
            m(50_000).mul_rate(0.035)
        );
    }

    #[test]
    fn alabama_cuts_its_standard_deduction_as_income_rises() {
        let y = StateTaxYear::y2026();
        let p = y.rules("AL").unwrap().std_phase_down.clone().unwrap();
        // Single: $4,500 (deduction plus exemption) up to $26,000, $4,000 from $35,500.
        assert_eq!(p.apply(false, m(4_500), m(20_000)), m(4_500));
        assert_eq!(p.apply(false, m(4_500), m(35_500)), m(4_000));
        assert_eq!(p.apply(false, m(4_500), m(90_000)), m(4_000));
        // Joint: $11,500 down to $8,000, half way between at $30,750.
        assert_eq!(p.apply(true, m(11_500), m(30_750)), m(9_750));
        // The tax follows: inside the phase-down each extra dollar also loses some deduction.
        let single = FilingStatus::Single;
        let at = |inc: i64| y.income_tax("AL", single, m(inc)).unwrap();
        assert!(at(30_500) - at(30_000) > at(40_500) - at(40_000));
    }

    #[test]
    fn alabama_leaves_pensions_out_but_taxes_ira_withdrawals() {
        let y = StateTaxYear::y2026();
        let single = FilingStatus::Single;
        let go = |pen: i64, dist: i64| {
            y.income_tax_with_retirement(
                "AL",
                single,
                m(60_000),
                Money::ZERO,
                &ret(pen, dist, &[70]),
            )
            .unwrap()
        };
        let plain = |inc: i64| y.income_tax("AL", single, m(inc)).unwrap();
        assert_eq!(go(30_000, 0), plain(30_000));
        assert_eq!(go(0, 30_000), plain(60_000));
    }

    #[test]
    fn michigan_subtracts_retirement_income_up_to_the_form_4884_limit() {
        let y = StateTaxYear::y2026();
        let go = |fs, ages: &[i32], pen: i64, dist: i64| {
            y.income_tax_with_retirement("MI", fs, m(150_000), Money::ZERO, &ret(pen, dist, ages))
                .unwrap()
        };
        let plain = |fs, inc: i64| y.income_tax("MI", fs, m(inc)).unwrap();
        let single = FilingStatus::Single;
        // $70,000 of pension and IRA income at 68: the $65,897 limit comes out.
        assert_eq!(
            go(single, &[68], 40_000, 30_000),
            plain(single, 150_000 - 65_897)
        );
        // Joint: $131,794.
        assert_eq!(
            go(FilingStatus::MarriedJoint, &[68, 66], 100_000, 60_000),
            plain(FilingStatus::MarriedJoint, 150_000 - 131_794)
        );
        // Smaller amounts come out in full.
        assert_eq!(go(single, &[68], 20_000, 0), plain(single, 130_000));
        // Born after 1966: nothing.
        assert_eq!(go(single, &[55], 20_000, 0), plain(single, 150_000));
    }

    #[test]
    fn missouri_private_pension_is_capped_and_fades_with_income() {
        let y = StateTaxYear::y2026();
        let single = FilingStatus::Single;
        let go = |agi: i64, ss: i64, pen: i64, ages: &[i32]| {
            y.income_tax_with_retirement("MO", single, m(agi), m(ss), &ret(pen, 0, ages))
                .unwrap()
        };
        let plain = |inc: i64| y.income_tax("MO", single, m(inc)).unwrap();
        // Under $25,000 of other income: the full $6,000.
        assert_eq!(go(25_000, 0, 20_000, &[66]), plain(25_000 - 6_000));
        // Each dollar over the line takes a dollar of the exemption: $3,000 over, $3,000 left.
        assert_eq!(go(28_000, 0, 20_000, &[66]), plain(28_000 - 3_000));
        // $6,000 or more over: nothing.
        assert_eq!(go(40_000, 0, 20_000, &[66]), plain(40_000));
        // Taxable Social Security does not count towards the line.
        assert_eq!(go(35_000, 10_000, 20_000, &[66]), plain(25_000 - 6_000));
        // A smaller pension comes out in full.
        assert_eq!(go(20_000, 0, 4_000, &[66]), plain(16_000));
    }

    #[test]
    fn ohio_gives_credits_for_retirement_income_and_age() {
        let y = StateTaxYear::y2026();
        let single = FilingStatus::Single;
        let go = |agi: i64, pen: i64, ages: &[i32]| {
            y.income_tax_with_retirement("OH", single, m(agi), Money::ZERO, &ret(pen, 0, ages))
                .unwrap()
        };
        let plain = |inc: i64| y.income_tax("OH", single, m(inc)).unwrap();
        // $60,000 of income: tax then a $200 retirement credit and the $50 senior credit.
        assert_eq!(go(60_000, 20_000, &[66]), plain(60_000) - m(250));
        // $4,000 of retirement income earns $80.
        assert_eq!(go(60_000, 4_000, &[66]), plain(60_000) - m(130));
        // Under 65: no senior credit.
        assert_eq!(go(60_000, 20_000, &[62]), plain(60_000) - m(200));
        // At $100,000 neither credit applies.
        assert_eq!(go(100_000, 20_000, &[66]), plain(100_000));
    }

    #[test]
    fn hawaii_leaves_out_employer_pensions_but_not_ira_withdrawals() {
        let y = StateTaxYear::y2026();
        let single = FilingStatus::Single;
        let go = |pen: i64, dist: i64| {
            y.income_tax_with_retirement(
                "HI",
                single,
                m(80_000),
                Money::ZERO,
                &ret(pen, dist, &[70]),
            )
            .unwrap()
        };
        let plain = |inc: i64| y.income_tax("HI", single, m(inc)).unwrap();
        assert_eq!(go(30_000, 0), plain(50_000));
        assert_eq!(go(0, 30_000), plain(80_000));
    }

    #[test]
    fn west_virginia_senior_modification_is_cut_by_benefits_left_out() {
        let y = StateTaxYear::y2026();
        let single = FilingStatus::Single;
        let go = |agi: i64, ss: i64, age: i32| {
            y.income_tax_with_retirement("WV", single, m(agi), m(ss), &ret(0, 0, &[age]))
                .unwrap()
        };
        let plain = |inc: i64| y.income_tax("WV", single, m(inc)).unwrap();
        // 66 with no benefits: $8,000 comes out.
        assert_eq!(go(50_000, 0, 66), plain(42_000));
        // $3,000 of benefits (all left out) cuts it to $5,000.
        assert_eq!(go(50_000, 3_000, 66), plain(50_000 - 3_000 - 5_000));
        // $8,000 or more of benefits: no senior modification.
        assert_eq!(go(50_000, 9_000, 66), plain(41_000));
        // Under 65: none.
        assert_eq!(go(50_000, 0, 60), plain(50_000));
    }

    #[test]
    fn rhode_island_pension_and_benefits_need_full_retirement_age_and_income_under_the_line() {
        let y = StateTaxYear::y2026();
        let single = FilingStatus::Single;
        let go = |agi: i64, ben: i64, pen: i64, dist: i64, ages: &[i32]| {
            y.income_tax_with_retirement("RI", single, m(agi), m(ben), &ret(pen, dist, ages))
                .unwrap()
        };
        let plain = |inc: i64| y.income_tax("RI", single, m(inc)).unwrap();
        // Pension of $70,000 at 68: $50,000 comes out.
        assert_eq!(go(100_000, 0, 70_000, 0, &[68]), plain(50_000));
        // IRA withdrawals do not count.
        assert_eq!(go(100_000, 0, 0, 40_000, &[68]), plain(100_000));
        // Too young.
        assert_eq!(go(100_000, 0, 70_000, 0, &[62]), plain(100_000));
        // Over the AGI line nothing comes out.
        assert_eq!(go(110_000, 0, 70_000, 0, &[68]), plain(110_000));
        // Benefits at 68 under the line are left out.
        assert_eq!(go(90_000, 30_000, 0, 0, &[68]), plain(60_000));
        // At 62 they are not.
        assert_eq!(go(90_000, 30_000, 0, 0, &[62]), plain(90_000));
    }

    #[test]
    fn maine_pension_deduction_shrinks_with_benefits_and_high_income() {
        let y = StateTaxYear::y2026();
        let single = FilingStatus::Single;
        let go = |agi: i64, ss: i64, pen: i64, ages: &[i32]| {
            let r = RetirementIncome {
                social_security: m(ss),
                ..ret(pen, 0, ages)
            };
            y.income_tax_with_retirement("ME", single, m(agi), Money::ZERO, &r)
                .unwrap()
        };
        let plain = |inc: i64| y.income_tax("ME", single, m(inc)).unwrap();
        // $60,000 pension at 66: the $48,216 cap comes out.
        assert_eq!(go(80_000, 0, 60_000, &[66]), plain(80_000 - 48_216));
        // $20,000 of benefits lowers the cap to $28,216.
        assert_eq!(go(80_000, 20_000, 60_000, &[66]), plain(80_000 - 28_216));
        // Halfway through the phase-out ($175,000): half the deduction.
        assert_eq!(go(175_000, 0, 60_000, &[66]), plain(175_000 - 24_108));
        // At $225,000 and over it is gone.
        assert_eq!(go(225_000, 0, 60_000, &[66]), plain(225_000));
    }

    #[test]
    fn virginia_age_deduction_phases_out_by_income() {
        let y = StateTaxYear::y2026();
        let t = |agi: i64, ages: &[i32]| {
            y.income_tax_with_retirement(
                "VA",
                FilingStatus::Single,
                m(agi),
                Money::ZERO,
                &ret(0, 0, ages),
            )
            .unwrap()
        };
        let base = |agi: i64| y.income_tax("VA", FilingStatus::Single, m(agi)).unwrap();
        // Under $50,000 the full $12,000 comes off at 65 and over, nothing at 64.
        assert_eq!(t(40_000, &[66]), base(28_000));
        assert_eq!(t(40_000, &[64]), base(40_000));
        // $5,000 over the line cuts it to $7,000; $12,000 over removes it.
        assert_eq!(t(55_000, &[66]), base(48_000));
        assert_eq!(t(62_000, &[66]), base(62_000));
    }

    #[test]
    fn new_jersey_pension_exclusion_shrinks_with_income() {
        let y = StateTaxYear::y2026();
        let x = y.rules("NJ").unwrap().retirement.clone().unwrap();
        let ex = |fs, income: i64, pen: i64, ages: &[i32]| {
            x.household_excluded(fs, m(income), &ret(pen, 0, ages))
        };
        let single = FilingStatus::Single;
        let joint = FilingStatus::MarriedJoint;
        assert_eq!(ex(single, 90_000, 60_000, &[63]), m(60_000));
        assert_eq!(ex(single, 90_000, 90_000, &[63]), m(75_000));
        assert_eq!(ex(joint, 90_000, 150_000, &[63, 50]), m(100_000));
        // Too young, or over the limit.
        assert_eq!(ex(single, 90_000, 60_000, &[61]), Money::ZERO);
        assert_eq!(ex(single, 160_000, 60_000, &[63]), Money::ZERO);
        // Steps: 37.5% of single, 50% of joint between $100,000 and $125,000.
        assert_eq!(ex(single, 110_000, 40_000, &[63]), m(15_000));
        assert_eq!(ex(joint, 110_000, 40_000, &[63]), m(20_000));
        assert_eq!(ex(single, 140_000, 40_000, &[63]), m(7_500));
    }

    #[test]
    fn louisiana_and_wisconsin_cap_retirement_income_by_age() {
        let y = StateTaxYear::y2026();
        let x = |c: &str| y.rules(c).unwrap().retirement.clone().unwrap();
        assert_eq!(x("LA").excluded(&ret(0, 30_000, &[65])), m(12_000));
        assert_eq!(x("LA").excluded(&ret(0, 30_000, &[64])), Money::ZERO);
        assert_eq!(x("WI").excluded(&ret(0, 30_000, &[67])), m(24_000));
        assert_eq!(x("WI").excluded(&ret(0, 80_000, &[67, 68])), m(48_000));
        assert_eq!(x("WI").excluded(&ret(0, 30_000, &[66])), Money::ZERO);
    }

    #[test]
    fn maryland_excludes_employer_pensions_less_social_security() {
        let y = StateTaxYear::y2026();
        let x = y.rules("MD").unwrap().retirement.clone().unwrap();
        let with_ss = |pen: i64, dist: i64, ss: i64, ages: &[i32]| {
            let mut r = ret(pen, dist, ages);
            r.social_security = m(ss);
            x.excluded(&r)
        };
        assert_eq!(with_ss(30_000, 0, 0, &[66]), m(30_000));
        assert_eq!(with_ss(60_000, 0, 0, &[66]), m(41_200));
        // IRA money does not count, and Social Security eats into the ceiling.
        assert_eq!(with_ss(0, 50_000, 0, &[66]), Money::ZERO);
        assert_eq!(with_ss(60_000, 0, 30_000, &[66]), m(11_200));
        assert_eq!(with_ss(60_000, 0, 50_000, &[66]), Money::ZERO);
        assert_eq!(with_ss(60_000, 0, 0, &[64]), Money::ZERO);
    }

    #[test]
    fn a_state_without_a_rule_taxes_everything() {
        let y = StateTaxYear::y2026();
        let r = ret(20_000, 30_000, &[70]);
        let plain = y
            .income_tax_with_ss("CA", FilingStatus::Single, m(50_000), Money::ZERO)
            .unwrap();
        let with = y
            .income_tax_with_retirement("CA", FilingStatus::Single, m(50_000), Money::ZERO, &r)
            .unwrap();
        assert_eq!(plain, with);
    }

    #[test]
    fn most_states_leave_social_security_out_and_a_few_tax_it() {
        let y = StateTaxYear::y2026();
        // $60,000 of income of which $20,000 is taxable Social Security.
        let with = |c: &str| {
            y.income_tax_with_ss(c, FilingStatus::Single, m(60_000), m(20_000))
                .unwrap()
        };
        let plain = |c: &str, inc: i64| y.income_tax(c, FilingStatus::Single, m(inc)).unwrap();
        // Illinois (4.95%, no standard deduction) does not tax benefits.
        assert_eq!(with("IL"), plain("IL", 40_000));
        // Utah does, so nothing comes out.
        assert_eq!(with("UT"), plain("UT", 60_000));
        // Without any Social Security the two ways agree.
        assert_eq!(
            y.income_tax_with_ss("IL", FilingStatus::Single, m(60_000), Money::ZERO),
            y.income_tax("IL", FilingStatus::Single, m(60_000))
        );
        // The list: exactly the eight states that tax benefits.
        let mut taxing: Vec<&str> = y
            .codes()
            .filter(|c| y.rules(c).unwrap().taxes_social_security)
            .collect();
        taxing.sort();
        assert_eq!(taxing, ["CO", "CT", "MN", "MT", "NM", "RI", "UT", "VT"]);
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
            // 4.95% after the $2,850 exemption.
            Some(m(97_150).mul_rate(0.0495))
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
