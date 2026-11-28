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

/// Federal income tax taken off state income: Alabama all of it, Missouri a share that falls as
/// income rises (up to a limit), Oregon up to a limit that falls with federal AGI.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct FederalTaxDeduction {
    /// The share of federal tax allowed by state income, as (at least, share) rows in rising
    /// order; empty means all of it.
    #[serde(default)]
    pub shares: Vec<(Money, f64)>,
    /// The most that can be taken by federal AGI, as (at least, limit) rows in rising order;
    /// empty means no limit.
    #[serde(default)]
    pub limits_single: Vec<(Money, Money)>,
    #[serde(default)]
    pub limits_joint: Vec<(Money, Money)>,
    /// Head of household filers use the joint limits (Oregon).
    #[serde(default)]
    pub head_uses_joint: bool,
}

impl FederalTaxDeduction {
    fn row<T: Copy>(rows: &[(Money, T)], at: Money) -> Option<T> {
        rows.iter().rev().find(|(from, _)| at >= *from).map(|r| r.1)
    }

    /// The deduction for `federal_tax`, given state income and federal AGI.
    pub fn amount(
        &self,
        fs: FilingStatus,
        state_income: Money,
        federal_agi: Money,
        federal_tax: Money,
    ) -> Money {
        let tax = federal_tax.max(Money::ZERO);
        let share = if self.shares.is_empty() {
            1.0
        } else {
            Self::row(&self.shares, state_income).unwrap_or(1.0)
        };
        let mut out = tax.mul_rate(share);
        let joint = fs == FilingStatus::MarriedJoint
            || (self.head_uses_joint && fs == FilingStatus::HeadOfHousehold);
        let limits = if joint {
            &self.limits_joint
        } else {
            &self.limits_single
        };
        if let Some(cap) = Self::row(limits, federal_agi) {
            out = out.min(cap);
        }
        out.max(Money::ZERO)
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
    /// From `age`, a deduction against any income that tops the retirement and military
    /// deductions up to `amount` a person (South Carolina's $15,000 at 65).
    #[serde(default)]
    pub age_floor: Option<AgeAllowance>,
    /// Credits against tax for retirement income and age (Ohio).
    #[serde(default)]
    pub retirement_credit: Option<RetirementCredit>,
    /// A credit on pensions and withdrawals for low-income older households, shrunk by Social
    /// Security and by income over a base (Oregon).
    #[serde(default)]
    pub low_income_retirement_credit: Option<LowIncomeRetirementCredit>,
    /// For states in `taxes_social_security`: when benefits are left out of state income.
    #[serde(default)]
    pub social_security: Option<SocialSecurityRule>,
    /// Deductions on all income, by the older person's year of birth, claimed instead of the
    /// retirement exclusion when larger (Michigan Tier 2 and Tier 3 standard deductions).
    #[serde(default)]
    pub birth_year_deductions: Vec<BirthYearDeduction>,
    /// A credit for each person born by a year, cut as income rises, taken instead of the Social
    /// Security credit when larger (Utah's $450 retirement credit).
    #[serde(default)]
    pub birth_year_credit: Option<BirthYearCredit>,
    /// Military retirement pay left out of income, apart from the other retirement rules.
    #[serde(default)]
    pub military: Option<MilitaryRetirement>,
    /// Federal income tax deducted from state income (Alabama, Missouri, Oregon).
    #[serde(default)]
    pub federal_tax_deduction: Option<FederalTaxDeduction>,
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
    /// Government pensions are exempt in full at any age; caps then cover what is left
    /// (New York, Louisiana, Kansas).
    #[serde(default)]
    pub public_pension_full: bool,
    /// Otherwise, government pensions up to this much a person are left out, apart from the
    /// other caps and with no income limit (Missouri: the $47,633 top Social Security benefit;
    /// West Virginia: $2,000).
    #[serde(default)]
    pub public_pension_cap: Money,
    /// The public cap is reduced by the Social Security the state left out (Missouri).
    #[serde(default)]
    pub public_less_ss_subtracted: bool,
    /// With `pension_full`, pensions are exempt at any age, since a pension paid on retiring
    /// counts as retirement income (Pennsylvania, Mississippi); `min_age` then only gates
    /// account distributions.
    #[serde(default)]
    pub pension_any_age: bool,
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

/// Oregon's retirement income credit (ORS 316.157, Publication OR-17 worksheet): `rate` of the
/// lesser of retirement income and `base` less Social Security less income over `income_base`.
#[derive(Debug, Clone, Deserialize)]
pub struct LowIncomeRetirementCredit {
    pub min_age: u8,
    pub base_single: Money,
    pub base_joint: Money,
    pub income_base_single: Money,
    pub income_base_joint: Money,
    /// No credit when household income, or household income plus Social Security, reaches this.
    pub income_limit_single: Money,
    pub income_limit_joint: Money,
    pub rate: f64,
}

impl LowIncomeRetirementCredit {
    /// `income` is household income without Social Security; `ss` is total benefits.
    pub fn total(
        &self,
        fs: FilingStatus,
        income: Money,
        retirement: Money,
        ss: Money,
        ages: &[i32],
    ) -> Money {
        let zero = Money::ZERO;
        if !ages.iter().any(|&a| a >= self.min_age as i32) {
            return zero;
        }
        let joint = fs == FilingStatus::MarriedJoint;
        let pick = |s: Money, j: Money| if joint { j } else { s };
        let limit = pick(self.income_limit_single, self.income_limit_joint);
        let (income, ss) = (income.max(zero), ss.max(zero));
        if income >= limit || income + ss >= limit {
            return zero;
        }
        let room = (pick(self.base_single, self.base_joint) - ss).max(zero);
        let over = (income - pick(self.income_base_single, self.income_base_joint)).max(zero);
        let line10 = (room - over).max(zero);
        retirement.max(zero).min(line10).mul_rate(self.rate)
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
    /// When set, the phase-out moves in whole steps of this much income, a part step counting
    /// as a whole one (Minnesota: 10% for each $4,000 or fraction of $4,000).
    #[serde(default)]
    pub phaseout_step: Money,
    /// Over the `exempt_agi` line no more than this share of the lesser of total benefits and
    /// the federal worksheet's income over its base amount stays in (Connecticut: 25%).
    #[serde(default)]
    pub benefit_cap_rate: f64,
}

/// The federal Social Security worksheet's base amounts (line 8), set in law and not indexed.
const FED_SS_BASE_SINGLE: i64 = 2_500_000;
const FED_SS_BASE_JOINT: i64 = 3_200_000;

impl SocialSecurityRule {
    /// Caps `in_state` by `benefit_cap_rate` of the lesser of `gross` benefits and the federal
    /// worksheet's line 9 (provisional income over the base). With no gross figure, or under the
    /// AGI line, `in_state` is returned unchanged.
    pub fn capped_by_benefits(
        &self,
        fs: FilingStatus,
        agi: Money,
        taxable_ss: Money,
        gross: Money,
        in_state: Money,
    ) -> Money {
        if self.benefit_cap_rate <= 0.0 || gross <= Money::ZERO {
            return in_state;
        }
        let joint = fs == FilingStatus::MarriedJoint;
        let limit = if joint {
            self.exempt_agi_joint
        } else {
            self.exempt_agi_single
        };
        if agi <= limit {
            return in_state;
        }
        let base = Money::from_cents(if joint {
            FED_SS_BASE_JOINT
        } else {
            FED_SS_BASE_SINGLE
        });
        // Line 9: other income plus half the benefits, less the base (tax-exempt interest and
        // adjustments are not tracked, so this can run a little high).
        let provisional =
            (agi - taxable_ss).max(Money::ZERO) + Money::from_cents(gross.cents() / 2);
        let line9 = (provisional - base).max(Money::ZERO);
        let cap = gross.min(line9).mul_rate(self.benefit_cap_rate);
        in_state.min(cap)
    }

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
            let mut into = (agi - limit).max(Money::ZERO).cents() as f64;
            if self.phaseout_step > Money::ZERO {
                let step = self.phaseout_step.cents() as f64;
                into = (into / step).ceil() * step;
            }
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

/// A deduction for households whose older person was born in a span of years.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct BirthYearDeduction {
    pub born_from: i32,
    pub born_to: i32,
    /// The older person must also be at least this old.
    #[serde(default)]
    pub min_age: u8,
    pub single: Money,
    pub joint: Money,
    /// Reduced by the federally taxable Social Security.
    #[serde(default)]
    pub less_taxable_ss: bool,
    /// Reduced by the state's personal exemptions.
    #[serde(default)]
    pub less_exemptions: bool,
}

impl BirthYearDeduction {
    /// The deduction, before any cut, when the household's oldest person qualifies.
    fn applies(&self, inc: &RetirementIncome) -> bool {
        let oldest = inc.birth_years.iter().min();
        let age = inc.ages.iter().max();
        match (oldest, age) {
            (Some(&b), Some(&a)) => {
                b >= self.born_from && b <= self.born_to && a >= self.min_age as i32
            }
            _ => false,
        }
    }
}

/// Utah's retirement credit (TC-40, code 18): `per_person` for each person born on or before
/// `born_by`, less `rate` of income over the start for the filing status.
#[derive(Debug, Clone, Deserialize)]
pub struct BirthYearCredit {
    pub born_by: i32,
    pub per_person: Money,
    pub start_single: Money,
    pub start_joint: Money,
    pub rate: f64,
}

impl BirthYearCredit {
    pub fn total(&self, fs: FilingStatus, income: Money, birth_years: &[i32]) -> Money {
        let n = birth_years.iter().filter(|&&b| b <= self.born_by).count() as i64;
        if n == 0 {
            return Money::ZERO;
        }
        let start = if fs == FilingStatus::MarriedJoint {
            self.start_joint
        } else {
            self.start_single
        };
        let over = (income - start).max(Money::ZERO);
        (Money::from_cents(self.per_person.cents() * n) - over.mul_rate(self.rate)).max(Money::ZERO)
    }
}

/// Military retirement pay the state leaves out. With no ceilings set it is all out.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct MilitaryRetirement {
    /// Each retiree must be at least this old (Idaho: 62).
    #[serde(default)]
    pub min_age: u8,
    /// At most this much for each retiree (New Mexico: $30,000); zero means no ceiling.
    #[serde(default)]
    pub per_person_cap: Money,
    /// Per-retiree ceilings by age: each gets the cap of the highest `age` reached (Maryland:
    /// $12,500, and $20,000 from 55).
    #[serde(default)]
    pub age_caps: Vec<AgeCap>,
    /// A household ceiling by filing status (Idaho: $48,216 and $72,324); zero means none.
    #[serde(default)]
    pub cap_single: Money,
    #[serde(default)]
    pub cap_joint: Money,
    /// The household ceiling is reduced by the Social Security benefits received (Idaho).
    #[serde(default)]
    pub less_social_security: bool,
    /// Only retirees this old or younger qualify (Colorado: 54); zero means no limit.
    #[serde(default)]
    pub max_age: u8,
    /// The amount falls to nothing over `width` of federal AGI above the start, with the share
    /// kept rounded to two places (Vermont: full to $125,000, none from $175,000).
    #[serde(default)]
    pub phaseout: Option<ExclusionPhaseout>,
    /// When above zero the state gives a credit of this share of the amount instead of leaving
    /// it out of income (Utah: 4.5%).
    #[serde(default)]
    pub credit_rate: f64,
    /// The military pay left out also uses up the general retirement caps (Arkansas's $6,000,
    /// South Carolina's retirement deduction).
    #[serde(default)]
    pub reduces_retirement_caps: bool,
    /// A retiree whose own earned income is over this much may leave out this much more
    /// (Georgia: $17,500); zero means no such extra.
    #[serde(default)]
    pub earned_extra: Money,
}

impl MilitaryRetirement {
    /// The military pay left out for a return.
    pub fn excluded(&self, fs: FilingStatus, federal_agi: Money, inc: &RetirementIncome) -> Money {
        let zero = Money::ZERO;
        let mut out = zero;
        for (i, &pay) in inc.military.iter().enumerate() {
            let age = inc.ages.get(i).copied().unwrap_or(0);
            if age < self.min_age as i32 || (self.max_age > 0 && age > self.max_age as i32) {
                continue;
            }
            let mut cut = pay.max(zero);
            if self.per_person_cap > zero {
                let mut cap = self.per_person_cap;
                if self.earned_extra > zero
                    && inc.earned.get(i).copied().unwrap_or(zero) > self.earned_extra
                {
                    cap += self.earned_extra;
                }
                cut = cut.min(cap);
            }
            if let Some(c) = self
                .age_caps
                .iter()
                .filter(|c| age >= c.age as i32)
                .max_by_key(|c| c.age)
            {
                cut = cut.min(c.cap);
            }
            out += cut;
        }
        let cap = if fs == FilingStatus::MarriedJoint {
            self.cap_joint
        } else {
            self.cap_single
        };
        if cap > zero {
            let mut cap = cap;
            if self.less_social_security {
                cap = (cap - inc.social_security.max(zero)).max(zero);
            }
            out = out.min(cap);
        }
        if let Some(p) = &self.phaseout {
            let start = if fs == FilingStatus::MarriedJoint {
                p.start_joint
            } else {
                p.start_single
            };
            let over = (federal_agi - start).max(zero).cents() as f64;
            let keep = (1.0 - over / p.width.cents().max(1) as f64).max(0.0);
            out = out.mul_rate((keep * 100.0).round() / 100.0);
        }
        out.min(inc.pension.max(zero))
    }
}

/// Retirement income in a year, for the state's exclusion.
#[derive(Debug, Clone, Default)]
pub struct RetirementIncome {
    pub pension: Money,
    /// The part of `pension` that is a government pension.
    pub public_pension: Money,
    pub distributions: Money,
    /// Gross Social Security benefits received, taxable or not.
    pub social_security: Money,
    /// Social Security the state left out of income; set by the tax calculation.
    pub ss_subtracted: Money,
    /// Federal adjusted gross income; set by the tax calculation.
    pub federal_agi: Money,
    /// Ages of the (adult) people on the return.
    pub ages: Vec<i32>,
    /// Their years of birth, in the same order; empty when not known.
    pub birth_years: Vec<i32>,
    /// Military retirement pay of each of them, in the same order (part of `pension`, and not
    /// of `public_pension`); empty when there is none.
    pub military: Vec<Money>,
    /// Taken off the retirement caps before they apply; set by the tax calculation.
    pub cap_reduction: Money,
    /// Each adult's own wages and self-employment income, in the same order as `ages`; empty
    /// when not known.
    pub earned: Vec<Money>,
    /// Federal income tax for the year, after credits; zero when not known.
    pub federal_tax: Money,
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
        if self.public_pension_full || self.public_pension_cap > zero {
            // Government pensions have their own rule (`public_excluded`) and do not use the
            // caps below.
            left_p -= inc.public_pension.max(zero).min(left_p);
        }
        if self.pension_full && (eligible || self.pension_any_age) {
            out += left_p;
            left_p = zero;
        }
        if eligible && self.distributions_full {
            out += left_d;
            left_d = zero;
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
        cap = (cap - inc.cap_reduction.max(zero)).max(zero);
        let pool = if self.pension_only {
            left_p
        } else {
            left_p + left_d
        };
        out + cap.min(pool)
    }

    /// The government pension left out, outside the caps and any income phase-out.
    pub fn public_excluded(&self, inc: &RetirementIncome) -> Money {
        let zero = Money::ZERO;
        let public = inc.public_pension.max(zero).min(inc.pension.max(zero));
        if self.public_pension_full {
            return public;
        }
        if self.public_pension_cap <= zero {
            return zero;
        }
        let n = inc.ages.len().max(1) as i64;
        let mut cut = public.min(Money::from_cents(self.public_pension_cap.cents() * n));
        if self.public_less_ss_subtracted {
            cut -= inc.ss_subtracted.max(zero);
        }
        cut.max(zero)
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
            (Some(rule), true) => {
                let kept = rule.taxed(fs, federal_agi, ss, &ret.ages);
                rule.capped_by_benefits(fs, federal_agi, ss, ret.social_security, kept)
            }
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
        // Military pay the state exempts comes out first and is not counted again by the
        // pension rules below.
        let mut military_cut = r
            .military
            .as_ref()
            .map_or(Money::ZERO, |m| m.excluded(fs, federal_agi, ret));
        let mut rest = ret.clone();
        rest.pension = (rest.pension - military_cut).max(Money::ZERO);
        rest.public_pension = rest.public_pension.min(rest.pension);
        if r.military
            .as_ref()
            .is_some_and(|m| m.reduces_retirement_caps)
        {
            rest.cap_reduction = military_cut;
        }
        let ret = &rest;
        let military_credit = match &r.military {
            Some(m) if m.credit_rate > 0.0 => {
                // A credit instead: the pay stays in income (and out of the pension rules).
                let c = military_cut.mul_rate(m.credit_rate);
                military_cut = Money::ZERO;
                c
            }
            _ => Money::ZERO,
        };
        income = (income - military_cut).max(Money::ZERO);
        if let Some(a) = &r.age_allowance {
            // The allowance is cut by the other modifications above it: Social Security and
            // government pensions (West Virginia Schedule M lines 29 to 34).
            let mut sub = (ss - ss_in_state).max(Money::ZERO) + military_cut;
            if let Some(x) = &r.retirement {
                sub += x.public_excluded(ret);
            }
            income = (income - a.total(sub, &ret.ages)).max(Money::ZERO);
        }
        let mut ret_cut = Money::ZERO;
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
            ret_cut = cut + x.household_excluded(fs, before, &ret) + x.public_excluded(&ret);
        }
        let joint = fs == FilingStatus::MarriedJoint;
        for d in r.birth_year_deductions.iter().filter(|d| d.applies(ret)) {
            let mut alt = if joint { d.joint } else { d.single };
            if d.less_taxable_ss {
                alt -= ss;
            }
            if d.less_exemptions {
                alt -= if joint { r.std_joint } else { r.std_single };
            }
            ret_cut = ret_cut.max(alt);
        }
        if let Some(f) = &r.age_floor {
            // Applied for the household as a whole: the retirement and military amounts left
            // out are not split by person.
            let n = ret.ages.iter().filter(|&&a| a >= f.age as i32).count() as i64;
            let floor = Money::from_cents(f.amount.cents() * n);
            let used = ret_cut + military_cut;
            if floor > used {
                ret_cut += floor - used;
            }
        }
        income = (income - ret_cut).max(Money::ZERO);
        // Federal tax comes off after the state's income is worked out, so the deductions that
        // depend on that income (Alabama's sliding standard deduction) still see all of it.
        let fed_cut = r.federal_tax_deduction.as_ref().map_or(Money::ZERO, |d| {
            d.amount(fs, income, federal_agi, ret.federal_tax)
        });
        let tax = self.tax_less(code, fs, income, fed_cut)?;
        let mut credit = match (&r.social_security, r.taxes_social_security) {
            (Some(rule), true) => rule.credit(fs, income, ss),
            _ => Money::ZERO,
        };
        // Utah allows the Social Security and military credits together, or the birth-year
        // retirement credit instead of both.
        credit += military_credit;
        if let Some(c) = &r.birth_year_credit {
            // One or the other: the household takes whichever is larger.
            credit = credit.max(c.total(fs, federal_agi, &ret.birth_years));
        }
        let rc = r.retirement_credit.as_ref().map_or(Money::ZERO, |c| {
            c.total(
                before_ss,
                ret.pension.max(Money::ZERO) + ret.distributions.max(Money::ZERO),
                &ret.ages,
            )
        });
        let lirc = r
            .low_income_retirement_credit
            .as_ref()
            .map_or(Money::ZERO, |c| {
                c.total(
                    fs,
                    before_ss,
                    ret.pension.max(Money::ZERO) + ret.distributions.max(Money::ZERO),
                    ret.social_security,
                    &ret.ages,
                )
            });
        Some((tax - credit - rc - lirc).max(Money::ZERO))
    }

    /// State income tax on `federal_agi`-like income for a resident of `code`.
    /// Returns None if the state code is unknown.
    pub fn income_tax(&self, code: &str, fs: FilingStatus, income: Money) -> Option<Money> {
        self.tax_less(code, fs, income, Money::ZERO)
    }

    /// As `income_tax`, with `less` also taken off taxable income (but not off the income that
    /// the standard deduction, exemption and recapture rules look at).
    fn tax_less(&self, code: &str, fs: FilingStatus, income: Money, less: Money) -> Option<Money> {
        let r = self.rules(code)?;
        if r.kind == StateKind::None {
            return Some(Money::ZERO);
        }
        let joint = fs == FilingStatus::MarriedJoint;
        if let Some(t) = &r.agi_tables {
            // Connecticut: no standard deduction; the exemption steps down with AGI.
            let taxable = (income - t.exemption(fs, income) - less).max(Money::ZERO);
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
        let taxable = (income - std - less).max(Money::ZERO);
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
            public_pension: Money::ZERO,
            distributions: m(dist),
            social_security: Money::ZERO,
            ss_subtracted: Money::ZERO,
            federal_agi: Money::ZERO,
            ages: ages.to_vec(),
            birth_years: Vec::new(),
            military: Vec::new(),
            cap_reduction: Money::ZERO,
            earned: Vec::new(),
            federal_tax: Money::ZERO,
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
        assert_eq!(x("DE").excluded(&ret(20_000, 0, &[50])), m(2_000));
        // Georgia steps up at 65; South Carolina from $3,000 to $10,000 at 65.
        assert_eq!(x("GA").excluded(&ret(0, 100_000, &[63])), m(35_000));
        assert_eq!(x("GA").excluded(&ret(0, 100_000, &[65])), m(65_000));
        assert_eq!(x("SC").excluded(&ret(0, 20_000, &[64])), m(3_000));
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
        // Minnesota moves in $4,000 steps, a part step counting in full: $20,000 over is five
        // steps (50% taxed), and $1 over is already one step (10%).
        assert_eq!(go("MN", 104_490, 20_000), plain("MN", 94_490));
        assert_eq!(go("MN", 84_491, 20_000), plain("MN", 64_491 + 2_000));
        assert_eq!(go("MN", 88_491, 20_000), plain("MN", 68_491 + 4_000));
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
    fn alabama_missouri_and_oregon_deduct_federal_tax() {
        let y = StateTaxYear::y2026();
        let single = FilingStatus::Single;
        let joint = FilingStatus::MarriedJoint;
        let go = |c: &str, fs: FilingStatus, agi: i64, fed: i64| {
            let r = RetirementIncome {
                federal_tax: m(fed),
                ..ret(0, 0, &[50])
            };
            y.income_tax_with_retirement(c, fs, m(agi), Money::ZERO, &r)
                .unwrap()
        };
        let plain = |c: &str, fs: FilingStatus, inc: i64| y.income_tax(c, fs, m(inc)).unwrap();
        // Alabama: all of it. The sliding standard deduction still looks at the full income, so
        // a $30,000 earner is taxed as $30,000 less the deduction at $30,000, less $2,000.
        let al = y.rules("AL").unwrap();
        let std = al
            .std_phase_down
            .as_ref()
            .unwrap()
            .apply(false, al.std_single, m(30_000));
        let expect = al.brackets.tax(m(30_000) - std - m(2_000));
        assert_eq!(go("AL", single, 30_000, 2_000), expect);
        assert!(go("AL", single, 30_000, 2_000) < plain("AL", single, 30_000));
        // Missouri: 15% of it at $60,000 ...
        assert_eq!(go("MO", single, 60_000, 6_000), plain("MO", single, 59_100));
        // ... at most $5,000 single and $10,000 joint ...
        assert_eq!(
            go("MO", single, 24_000, 20_000),
            plain("MO", single, 19_000)
        );
        assert_eq!(go("MO", joint, 24_000, 40_000), plain("MO", joint, 14_000));
        // ... and none over $125,000.
        assert_eq!(
            go("MO", single, 130_000, 20_000),
            plain("MO", single, 130_000)
        );
        // Oregon: up to $8,500, less as federal AGI passes $125,000 ($250,000 joint).
        assert_eq!(go("OR", single, 80_000, 7_000), plain("OR", single, 73_000));
        assert_eq!(
            go("OR", single, 80_000, 12_000),
            plain("OR", single, 71_500)
        );
        assert_eq!(
            go("OR", single, 132_000, 20_000),
            plain("OR", single, 126_900)
        );
        assert_eq!(
            go("OR", single, 150_000, 20_000),
            plain("OR", single, 150_000)
        );
        assert_eq!(
            go("OR", joint, 255_000, 40_000),
            plain("OR", joint, 248_200)
        );
        // Oregon heads of household use the joint limits; Missouri's get the single $5,000.
        let head = FilingStatus::HeadOfHousehold;
        assert_eq!(go("OR", head, 140_000, 20_000), plain("OR", head, 131_500));
        assert_eq!(go("MO", head, 24_000, 20_000), plain("MO", head, 19_000));
        // States without the rule ignore it.
        assert_eq!(go("CA", single, 80_000, 9_000), plain("CA", single, 80_000));
    }

    #[test]
    fn pennsylvania_and_mississippi_exempt_pensions_at_any_age() {
        for c in ["PA", "MS"] {
            // A pension at 52 is left out; an IRA withdrawal at 52 is taxed.
            assert_eq!(tax(c, 40_000, &ret(40_000, 0, &[52])), Money::ZERO, "{c}");
            assert_eq!(
                tax(c, 40_000, &ret(0, 40_000, &[52])),
                tax(c, 40_000, &ret(0, 0, &[52])),
                "{c}"
            );
            // From 60 both are.
            assert_eq!(tax(c, 40_000, &ret(0, 40_000, &[60])), Money::ZERO, "{c}");
        }
    }

    #[test]
    fn wisconsin_slides_its_standard_deduction_down_to_nothing() {
        let y = StateTaxYear::y2026();
        let p = y.rules("WI").unwrap().std_phase_down.clone().unwrap();
        let r = y.rules("WI").unwrap();
        let (s, j) = (r.std_single, r.std_joint);
        // Full deduction at low incomes, none at high ones.
        assert_eq!(p.apply(false, s, m(20_000)), s);
        assert_eq!(p.apply(false, s, m(150_000)), Money::ZERO);
        assert_eq!(p.apply(true, j, m(30_000)), j);
        assert_eq!(p.apply(true, j, m(170_000)), Money::ZERO);
        // In between it falls 12 cents (single) and about 19.8 cents (joint) per dollar.
        let drop = p.apply(false, s, m(50_000)) - p.apply(false, s, m(60_000));
        assert!((drop.cents() - 120_000).abs() <= 100, "{drop:?}");
        let drop = p.apply(true, j, m(50_000)) - p.apply(true, j, m(60_000));
        assert!((drop.cents() - 197_780).abs() <= 200, "{drop:?}");
        // So a $60,000 single filer pays more than the flat maximum deduction suggested.
        let single = FilingStatus::Single;
        let tax = y.income_tax("WI", single, m(60_000)).unwrap();
        let flat = r.brackets.tax(m(60_000) - s);
        assert!(tax > flat);
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
    fn government_pensions_are_fully_exempt_where_the_state_says_so() {
        let y = StateTaxYear::y2026();
        let single = FilingStatus::Single;
        let go = |c: &str, agi: i64, pension: i64, public: i64, dist: i64, age: i32| {
            let r = RetirementIncome {
                public_pension: m(public),
                ..ret(pension, dist, &[age])
            };
            y.income_tax_with_retirement(c, single, m(agi), Money::ZERO, &r)
                .unwrap()
        };
        let plain = |c: &str, inc: i64| y.income_tax(c, single, m(inc)).unwrap();
        // New York: a $50,000 state pension is all out, and the $20,000 cap still covers IRA money.
        assert_eq!(
            go("NY", 90_000, 50_000, 50_000, 30_000, 62),
            plain("NY", 20_000)
        );
        // The same pension from a private employer: only the $20,000 cap.
        assert_eq!(go("NY", 90_000, 50_000, 0, 30_000, 62), plain("NY", 70_000));
        // Even before 59 and a half.
        assert_eq!(go("NY", 90_000, 50_000, 50_000, 0, 55), plain("NY", 40_000));
        // Kansas: government pensions out, everything else taxed.
        assert_eq!(go("KS", 80_000, 40_000, 30_000, 0, 66), plain("KS", 50_000));
        // A state without the rule treats it like any pension.
        assert_eq!(go("CA", 80_000, 40_000, 40_000, 0, 66), plain("CA", 80_000));
    }

    #[test]
    fn military_retired_pay_follows_each_states_rule() {
        let y = StateTaxYear::y2026();
        let go = |c: &str,
                  fs: FilingStatus,
                  agi: i64,
                  ss: i64,
                  pension: i64,
                  mil: &[i64],
                  ages: &[i32]| {
            let r = RetirementIncome {
                social_security: m(ss),
                military: mil.iter().map(|&d| m(d)).collect(),
                ..ret(pension, 0, ages)
            };
            y.income_tax_with_retirement(c, fs, m(agi), Money::ZERO, &r)
                .unwrap()
        };
        let single = FilingStatus::Single;
        let joint = FilingStatus::MarriedJoint;
        let plain = |c: &str, fs: FilingStatus, inc: i64| y.income_tax(c, fs, m(inc)).unwrap();
        // Missouri: all of it, at any age.
        assert_eq!(
            go("MO", single, 80_000, 0, 30_000, &[30_000], &[45]),
            plain("MO", single, 50_000)
        );
        // Rhode Island: all of it at any age, and the $50,000 pension
        // exclusion at 67 still applies to the rest.
        assert_eq!(
            go("RI", single, 80_000, 0, 30_000, &[30_000], &[50]),
            plain("RI", single, 50_000)
        );
        assert_eq!(
            go("RI", single, 100_000, 0, 90_000, &[30_000], &[70]),
            plain("RI", single, 20_000)
        );
        // California has no such rule.
        assert_eq!(
            go("CA", single, 80_000, 0, 30_000, &[30_000], &[66]),
            plain("CA", single, 80_000)
        );
        // Idaho: nothing before 62.
        assert_eq!(
            go("ID", single, 80_000, 0, 30_000, &[30_000], &[61]),
            plain("ID", single, 80_000)
        );
        // At 62, up to $48,216.
        assert_eq!(
            go("ID", single, 80_000, 0, 30_000, &[30_000], &[62]),
            plain("ID", single, 50_000)
        );
        // Less the Social Security received, taxed or not.
        assert_eq!(
            go("ID", single, 80_000, 30_000, 30_000, &[30_000], &[62]),
            plain("ID", single, 80_000 - 18_216)
        );
        // New Mexico: $30,000 for each retiree.
        assert_eq!(
            go(
                "NM",
                joint,
                120_000,
                0,
                50_000,
                &[40_000, 10_000],
                &[50, 50]
            ),
            plain("NM", joint, 80_000)
        );
        // Maine: military pay is out on its own, and the $48,216 cap still covers the other
        // $20,000 of pension (not counted twice).
        assert_eq!(
            go("ME", single, 60_000, 0, 50_000, &[30_000], &[66]),
            plain("ME", single, 10_000)
        );
        // The military pay cannot be more than the pension it is part of.
        assert_eq!(
            go("MO", single, 80_000, 0, 10_000, &[30_000], &[66]),
            plain("MO", single, 70_000)
        );
    }

    #[test]
    fn vermont_colorado_and_utah_military_rules() {
        let y = StateTaxYear::y2026();
        let single = FilingStatus::Single;
        let go = |c: &str, agi: i64, pension: i64, mil: i64, age: i32| {
            let r = RetirementIncome {
                military: vec![m(mil)],
                ..ret(pension, 0, &[age])
            };
            y.income_tax_with_retirement(c, single, m(agi), Money::ZERO, &r)
                .unwrap()
        };
        let plain = |c: &str, inc: i64| y.income_tax(c, single, m(inc)).unwrap();
        // Vermont: all of it up to $125,000 of AGI.
        assert_eq!(go("VT", 120_000, 40_000, 40_000, 60), plain("VT", 80_000));
        // At $150,000, half ((175,000 - 150,000) / 50,000).
        assert_eq!(go("VT", 150_000, 40_000, 40_000, 60), plain("VT", 130_000));
        // At $163,000 the share rounds to 0.24.
        assert_eq!(
            go("VT", 163_000, 40_000, 40_000, 60),
            plain("VT", 163_000 - 9_600)
        );
        // From $175,000 none.
        assert_eq!(go("VT", 180_000, 40_000, 40_000, 60), plain("VT", 180_000));
        // Colorado: $15,000 at 54 or younger.
        assert_eq!(go("CO", 80_000, 30_000, 30_000, 50), plain("CO", 65_000));
        // Utah: a credit of 4.5% of the pay, the pay stays in income.
        let ut = go("UT", 80_000, 30_000, 30_000, 50);
        assert_eq!(ut, plain("UT", 80_000) - m(1_350));
    }

    #[test]
    fn military_pay_in_the_other_states() {
        let y = StateTaxYear::y2026();
        let single = FilingStatus::Single;
        let joint = FilingStatus::MarriedJoint;
        let go = |c: &str,
                  fs: FilingStatus,
                  agi: i64,
                  pension: i64,
                  dist: i64,
                  mil: &[i64],
                  ages: &[i32]| {
            let r = RetirementIncome {
                military: mil.iter().map(|&d| m(d)).collect(),
                ..ret(pension, dist, ages)
            };
            y.income_tax_with_retirement(c, fs, m(agi), Money::ZERO, &r)
                .unwrap()
        };
        let plain = |c: &str, fs: FilingStatus, inc: i64| y.income_tax(c, fs, m(inc)).unwrap();
        // Full exemptions.
        for c in ["AZ", "IN", "MN", "NE", "NJ", "NC", "ND", "OK", "WI"] {
            assert_eq!(
                go(c, single, 90_000, 40_000, 0, &[40_000], &[50]),
                plain(c, single, 50_000),
                "{c}"
            );
        }
        // Arkansas: all of it, and it uses up the $6,000 retirement exemption.
        assert_eq!(
            go("AR", single, 90_000, 40_000, 10_000, &[40_000], &[50]),
            plain("AR", single, 50_000)
        );
        // Military pay under $6,000 leaves the rest of it for IRA money.
        assert_eq!(
            go("AR", single, 90_000, 4_000, 10_000, &[4_000], &[50]),
            plain("AR", single, 84_000)
        );
        // South Carolina: likewise with its $3,000 retirement deduction under 65, which
        // $8,000 of military pay uses up.
        assert_eq!(
            go("SC", single, 90_000, 30_000, 20_000, &[8_000], &[60]),
            plain("SC", single, 82_000)
        );
        // Georgia: $17,500 under 62; from 62 the general exclusion covers it.
        assert_eq!(
            go("GA", single, 90_000, 30_000, 0, &[30_000], &[50]),
            plain("GA", single, 72_500)
        );
        assert_eq!(
            go("GA", single, 90_000, 30_000, 0, &[30_000], &[63]),
            plain("GA", single, 60_000)
        );
        // Maryland: $12,500 under 55, $20,000 from 55, each.
        assert_eq!(
            go(
                "MD",
                joint,
                150_000,
                60_000,
                0,
                &[30_000, 30_000],
                &[50, 56]
            ),
            plain("MD", joint, 150_000 - 32_500)
        );
        // Virginia: up to $40,000 each.
        assert_eq!(
            go("VA", single, 90_000, 50_000, 0, &[50_000], &[50]),
            plain("VA", single, 50_000)
        );
        // Delaware: $12,500 under 60.
        assert_eq!(
            go("DE", single, 90_000, 30_000, 0, &[30_000], &[50]),
            plain("DE", single, 77_500)
        );
    }

    #[test]
    fn georgia_allows_more_military_pay_out_with_earned_income() {
        let y = StateTaxYear::y2026();
        let single = FilingStatus::Single;
        let go = |earned: i64| {
            let r = RetirementIncome {
                military: vec![m(40_000)],
                earned: vec![m(earned)],
                ..ret(40_000, 0, &[50])
            };
            y.income_tax_with_retirement("GA", single, m(100_000), Money::ZERO, &r)
                .unwrap()
        };
        let plain = |inc: i64| y.income_tax("GA", single, m(inc)).unwrap();
        // $17,500 without earnings over $17,500; $35,000 with them.
        assert_eq!(go(0), plain(82_500));
        assert_eq!(go(17_500), plain(82_500));
        assert_eq!(go(60_000), plain(65_000));
    }

    #[test]
    fn south_carolina_tops_retirement_deductions_up_to_15000_at_65() {
        let y = StateTaxYear::y2026();
        let single = FilingStatus::Single;
        let go = |pension: i64, mil: i64, age: i32| {
            let r = RetirementIncome {
                military: if mil > 0 { vec![m(mil)] } else { vec![] },
                ..ret(pension, 0, &[age])
            };
            y.income_tax_with_retirement("SC", single, m(80_000), Money::ZERO, &r)
                .unwrap()
        };
        let plain = |inc: i64| y.income_tax("SC", single, m(inc)).unwrap();
        // Under 65: $3,000 of retirement income.
        assert_eq!(go(20_000, 0, 60), plain(77_000));
        // At 65 with no retirement income: $15,000 against wages.
        assert_eq!(go(0, 0, 66), plain(65_000));
        // $10,000 retirement deduction, topped up to $15,000.
        assert_eq!(go(20_000, 0, 66), plain(65_000));
        // Military pay of $30,000 is out in full and leaves no top-up.
        assert_eq!(go(30_000, 30_000, 66), plain(50_000));
    }

    #[test]
    fn missouri_and_west_virginia_cap_government_pensions_their_own_way() {
        let y = StateTaxYear::y2026();
        let single = FilingStatus::Single;
        let go = |c: &str, agi: i64, ss: i64, pension: i64, public: i64, age: i32| {
            let r = RetirementIncome {
                public_pension: m(public),
                ..ret(pension, 0, &[age])
            };
            y.income_tax_with_retirement(c, single, m(agi), m(ss), &r)
                .unwrap()
        };
        let plain = |c: &str, inc: i64| y.income_tax(c, single, m(inc)).unwrap();
        // Missouri: a $60,000 state pension, no benefits: $47,633 out, and no income limit.
        assert_eq!(
            go("MO", 100_000, 0, 60_000, 60_000, 66),
            plain("MO", 100_000 - 47_633)
        );
        // Arizona: $2,500 of a government pension each, nothing for a private one.
        assert_eq!(go("AZ", 80_000, 0, 30_000, 30_000, 66), plain("AZ", 77_500));
        assert_eq!(go("AZ", 80_000, 0, 30_000, 0, 66), plain("AZ", 80_000));
        // Taxable benefits of $10,000 come out on their own and cut the public cap by as much.
        assert_eq!(
            go("MO", 110_000, 10_000, 60_000, 60_000, 66),
            plain("MO", 100_000 - 37_633)
        );
        // The same pension from a private employer gets only the $6,000 private cap, which has
        // phased out at this income.
        assert_eq!(go("MO", 100_000, 0, 60_000, 0, 66), plain("MO", 100_000));
        // West Virginia: $2,000 of a government pension at 60, and at 66 the $8,000 senior
        // allowance is cut by that $2,000, so $8,000 in all.
        assert_eq!(go("WV", 50_000, 0, 20_000, 20_000, 60), plain("WV", 48_000));
        assert_eq!(go("WV", 50_000, 0, 20_000, 20_000, 66), plain("WV", 42_000));
    }

    #[test]
    fn utah_retirement_credit_is_for_people_born_by_1952_and_fades_with_income() {
        let y = StateTaxYear::y2026();
        let c = y.rules("UT").unwrap().birth_year_credit.clone().unwrap();
        let single = FilingStatus::Single;
        let joint = FilingStatus::MarriedJoint;
        assert_eq!(c.total(single, m(20_000), &[1950]), m(450));
        // $10,000 over $25,000 takes off $250.
        assert_eq!(c.total(single, m(35_000), &[1950]), m(200));
        assert_eq!(c.total(single, m(60_000), &[1950]), Money::ZERO);
        assert_eq!(c.total(single, m(20_000), &[1953]), Money::ZERO);
        assert_eq!(c.total(joint, m(32_000), &[1948, 1951]), m(900));
        assert_eq!(c.total(joint, m(32_000), &[1948, 1960]), m(450));
        // In the tax: with no benefits it is the only credit, so it comes off in full.
        let r = RetirementIncome {
            birth_years: vec![1950],
            ..ret(20_000, 0, &[76])
        };
        let with = y
            .income_tax_with_retirement("UT", single, m(30_000), Money::ZERO, &r)
            .unwrap();
        let without = y
            .income_tax_with_retirement(
                "UT",
                single,
                m(30_000),
                Money::ZERO,
                &ret(20_000, 0, &[76]),
            )
            .unwrap();
        assert_eq!(without - with, m(325).min(without));
        // When the Social Security credit is larger, that one is used instead, not both.
        let r = RetirementIncome {
            birth_years: vec![1950],
            ..ret(0, 0, &[76])
        };
        let both = y
            .income_tax_with_retirement("UT", single, m(40_000), m(20_000), &r)
            .unwrap();
        let ss_only = y
            .income_tax_with_retirement("UT", single, m(40_000), m(20_000), &ret(0, 0, &[76]))
            .unwrap();
        assert_eq!(both, ss_only);
    }

    #[test]
    fn oregon_retirement_income_credit_follows_the_or17_worksheet() {
        let c = StateTaxYear::y2026()
            .rules("OR")
            .unwrap()
            .low_income_retirement_credit
            .clone()
            .unwrap();
        let single = FilingStatus::Single;
        // $10,000 pension, $3,000 of benefits, $14,000 other income: 9% of $4,500.
        assert_eq!(
            c.total(single, m(14_000), m(10_000), m(3_000), &[70]),
            m(405)
        );
        // $17,000 of income: $2,000 over the base comes off too.
        assert_eq!(
            c.total(single, m(17_000), m(10_000), m(3_000), &[70]),
            m(225)
        );
        // Small pensions cap it.
        assert_eq!(c.total(single, m(5_000), m(1_000), m(0), &[70]), m(90));
        // Too young, too much income, or income plus benefits too high.
        assert_eq!(
            c.total(single, m(14_000), m(10_000), m(3_000), &[61]),
            Money::ZERO
        );
        assert_eq!(
            c.total(single, m(22_500), m(10_000), m(0), &[70]),
            Money::ZERO
        );
        assert_eq!(
            c.total(single, m(16_000), m(10_000), m(7_000), &[70]),
            Money::ZERO
        );
        // Joint figures double: at most 9% of $15,000.
        let joint = FilingStatus::MarriedJoint;
        assert_eq!(
            c.total(joint, m(20_000), m(20_000), m(0), &[66, 63]),
            m(1_350)
        );
        // It comes off Oregon tax, which it cannot push below zero.
        let y = StateTaxYear::y2026();
        let r = ret(10_000, 0, &[70]);
        let with = y
            .income_tax_with_retirement("OR", single, m(20_000), Money::ZERO, &r)
            .unwrap();
        let without = y.income_tax("OR", single, m(20_000)).unwrap();
        // $20,000 of income is $5,000 over the base: 9% of $2,500.
        assert_eq!(with, (without - m(225)).max(Money::ZERO));
    }

    #[test]
    fn michigan_older_tiers_can_take_a_standard_deduction_instead() {
        let y = StateTaxYear::y2026();
        let single = FilingStatus::Single;
        let go = |agi: i64, taxable_ss: i64, pension: i64, age: i32, born: i32| {
            let r = RetirementIncome {
                birth_years: vec![born],
                ..ret(pension, 0, &[age])
            };
            y.income_tax_with_retirement("MI", single, m(agi), m(taxable_ss), &r)
                .unwrap()
        };
        let plain = |inc: i64| y.income_tax("MI", single, m(inc)).unwrap();
        // Tier 2 (born 1950): $20,000 beats a $10,000 pension subtraction.
        assert_eq!(go(60_000, 0, 10_000, 76, 1950), plain(40_000));
        // A larger pension is still worth more than the standard deduction.
        assert_eq!(go(60_000, 0, 30_000, 76, 1950), plain(30_000));
        // Tier 3 (born 1956): $20,000 less $8,000 taxable benefits and the $5,800 exemption.
        assert_eq!(go(68_000, 8_000, 3_000, 70, 1956), plain(60_000 - 6_200));
        // Born 1962: only the pension subtraction.
        assert_eq!(go(68_000, 8_000, 3_000, 64, 1962), plain(60_000 - 3_000));
        // Unknown birth year: no standard deduction.
        assert_eq!(go(60_000, 0, 10_000, 76, 0), plain(50_000));
        // Joint: the older spouse decides.
        let r = RetirementIncome {
            birth_years: vec![1951, 1962],
            ..ret(5_000, 0, &[75, 64])
        };
        let joint = FilingStatus::MarriedJoint;
        assert_eq!(
            y.income_tax_with_retirement("MI", joint, m(90_000), Money::ZERO, &r)
                .unwrap(),
            y.income_tax("MI", joint, m(50_000)).unwrap()
        );
    }

    #[test]
    fn connecticut_keeps_at_most_a_quarter_of_benefits_over_its_line() {
        let y = StateTaxYear::y2026();
        let single = FilingStatus::Single;
        let go = |agi: i64, taxable: i64, gross: i64| {
            let r = RetirementIncome {
                social_security: m(gross),
                ..ret(0, 0, &[70])
            };
            y.income_tax_with_retirement("CT", single, m(agi), m(taxable), &r)
                .unwrap()
        };
        let plain = |inc: i64| y.income_tax("CT", single, m(inc)).unwrap();
        // $24,000 of benefits, $20,400 federally taxable: 25% of $24,000 stays in.
        assert_eq!(go(100_000, 20_400, 24_000), plain(79_600 + 6_000));
        // When income over the federal base is smaller than the benefits, it sets the cap:
        // $25,000 + $30,000 - $25,000 = $30,000, and 25% of that is $7,500.
        assert_eq!(go(76_000, 51_000, 60_000), plain(25_000 + 7_500));
        // Under the line benefits still come out entirely.
        assert_eq!(go(70_000, 20_400, 24_000), plain(49_600));
        // Without a gross figure the federally taxable amount is used, as before.
        assert_eq!(go(100_000, 20_400, 0), plain(100_000));
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
