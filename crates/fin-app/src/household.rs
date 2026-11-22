//! The household profile the plan runs on, plus the assumption options a person can pick from.
//! Stored as JSON in the settings table.
use crate::App;
use chrono::{Datelike, NaiveDate};
use fin_core::{Money, Person};
use fin_plan::params::Assumptions;
use fin_plan::projection::{
    Accounts, IncomeStream, PlanInput, SocialSecurity, SpendingRule, WithdrawalOrder,
};
use fin_plan::tax::FilingStatus;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const KEY: &str = "household";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonRow {
    pub name: String,
    pub birth_date: String,
    pub retire_age: i32,
    /// Estimated Social Security per year in today's dollars (at the assumed claim age).
    pub ss_annual_cents: i64,
}

/// One source of earned income. Leave the years empty for "from now until that person retires";
/// set both to the same year for a one-time amount (a bonus, a stock vest, a sale).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamRow {
    pub label: String,
    /// Index into `people`.
    pub person: usize,
    pub annual_cents: i64,
    /// Business income: pays self-employment tax and gets the QBI deduction.
    #[serde(default)]
    pub self_employed: bool,
    #[serde(default)]
    pub start_year: Option<i32>,
    #[serde(default)]
    pub end_year: Option<i32>,
    #[serde(default)]
    pub contribution_cents: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Household {
    pub people: Vec<PersonRow>,
    pub streams: Vec<StreamRow>,
    pub filing: String,
    pub state: String,
    pub spending_cents: i64,
    /// Planned one-off costs (and untaxed inflows when negative).
    #[serde(default)]
    pub events: Vec<EventRow>,
    pub cash_cents: i64,
    pub taxable_cents: i64,
    pub traditional_cents: i64,
    pub roth_cents: i64,
    /// Chosen option label per assumption key; missing means "the likeliest".
    #[serde(default)]
    pub picks: BTreeMap<String, String>,
    /// How spending responds to the portfolio in retirement: fixed, percent, vpw or guardrails.
    #[serde(default = "default_spending_rule")]
    pub spending_rule: String,
    /// Which accounts to draw first: standard, traditional_first, roth_first or blended.
    #[serde(default = "default_withdrawal_order")]
    pub withdrawal_order: String,
    /// Charge Medicare premiums (with IRMAA) from age 65.
    #[serde(default = "default_true")]
    pub medicare: bool,
    /// Add estimated healthcare costs: premiums before 65, out-of-pocket costs after, and an
    /// expected long-term care spell.
    #[serde(default)]
    pub health: bool,
    /// Yearly giving to charity (part of spending). From 70 and a half it can go straight from
    /// the IRA, which lowers taxable income.
    #[serde(default)]
    pub giving_cents: i64,
    /// The home the household owns, if it is part of the plan.
    #[serde(default)]
    pub home: Option<HomeRow>,
}

/// The home: its value, what is owed, and an optional sale.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HomeRow {
    pub value_cents: i64,
    pub basis_cents: i64,
    pub mortgage_balance_cents: i64,
    /// Hundredths of a percent: 450 is 4.5%.
    pub mortgage_rate_bp: i32,
    pub mortgage_years_left: i32,
    #[serde(default)]
    pub sell_year: Option<i32>,
    /// Hundredths of a percent of the price: 600 is 6%.
    #[serde(default = "default_selling_cost")]
    pub selling_cost_bp: i32,
    #[serde(default)]
    pub replacement_cost_cents: i64,
    #[serde(default)]
    pub rent_after_cents: i64,
}

fn default_selling_cost() -> i32 {
    600
}

fn default_true() -> bool {
    true
}

fn default_spending_rule() -> String {
    "fixed".into()
}
fn default_withdrawal_order() -> String {
    "standard".into()
}

impl Household {
    fn spending_rule_value(&self) -> Result<SpendingRule, String> {
        Ok(match self.spending_rule.as_str() {
            "fixed" => SpendingRule::Fixed,
            // 4% of the portfolio each year, never below a quarter of planned spending.
            "percent" => SpendingRule::PercentOfPortfolio {
                rate: 0.04,
                floor: Money::from_cents(self.spending_cents / 4),
            },
            "vpw" => SpendingRule::Vpw { rate: 0.03 },
            "guardrails" => SpendingRule::Guardrails {
                band: 0.2,
                step: 0.1,
            },
            _ => return Err("Choose a spending rule from the list.".into()),
        })
    }

    fn withdrawal_order_value(&self) -> Result<WithdrawalOrder, String> {
        Ok(match self.withdrawal_order.as_str() {
            "standard" => WithdrawalOrder::Standard,
            "traditional_first" => WithdrawalOrder::TraditionalFirst,
            "roth_first" => WithdrawalOrder::RothFirst,
            "blended" => WithdrawalOrder::Blended,
            _ => return Err("Choose a withdrawal order from the list.".into()),
        })
    }
}

/// The first version of the saved profile (one person, one salary); still read so earlier
/// installs keep their data.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyHousehold {
    birth_date: String,
    retire_age: i32,
    filing: String,
    state: String,
    salary_cents: i64,
    contribution_cents: i64,
    spending_cents: i64,
    cash_cents: i64,
    taxable_cents: i64,
    traditional_cents: i64,
    roth_cents: i64,
    ss_annual_cents: i64,
    #[serde(default)]
    picks: BTreeMap<String, String>,
}

impl From<LegacyHousehold> for Household {
    fn from(l: LegacyHousehold) -> Household {
        Household {
            people: vec![PersonRow {
                name: "You".into(),
                birth_date: l.birth_date,
                retire_age: l.retire_age,
                ss_annual_cents: l.ss_annual_cents,
            }],
            streams: if l.salary_cents > 0 {
                vec![StreamRow {
                    label: "Salary".into(),
                    person: 0,
                    annual_cents: l.salary_cents,
                    self_employed: false,
                    start_year: None,
                    end_year: None,
                    contribution_cents: l.contribution_cents,
                }]
            } else {
                vec![]
            },
            filing: l.filing,
            state: l.state,
            spending_cents: l.spending_cents,
            events: vec![],
            cash_cents: l.cash_cents,
            taxable_cents: l.taxable_cents,
            traditional_cents: l.traditional_cents,
            roth_cents: l.roth_cents,
            picks: l.picks,
            spending_rule: default_spending_rule(),
            withdrawal_order: default_withdrawal_order(),
            medicare: true,
            health: false,
            giving_cents: 0,
            home: None,
        }
    }
}

fn age_in(p: &Person, today: NaiveDate) -> i32 {
    p.age_on(today)
}

fn age_of(birth: &str, today: NaiveDate) -> Result<(NaiveDate, i32), String> {
    let b = NaiveDate::parse_from_str(birth, "%Y-%m-%d")
        .map_err(|_| "Enter birth dates as YYYY-MM-DD.".to_string())?;
    Ok((b, fin_core::person::age_on(b, today)))
}

impl Household {
    /// A placeholder household so a first-time user sees a working plan to edit.
    pub fn sample() -> Household {
        LegacyHousehold {
            birth_date: "1981-06-15".into(),
            retire_age: 62,
            filing: "married_joint".into(),
            state: "TX".into(),
            salary_cents: 18_000_000,
            contribution_cents: 2_350_000,
            spending_cents: 9_000_000,
            cash_cents: 3_000_000,
            taxable_cents: 15_000_000,
            traditional_cents: 60_000_000,
            roth_cents: 12_000_000,
            ss_annual_cents: 3_800_000,
            picks: BTreeMap::new(),
        }
        .into()
    }

    pub fn from_json(json: &str) -> Result<Household, String> {
        match serde_json::from_str::<Household>(json) {
            Ok(h) => Ok(h),
            Err(_) => serde_json::from_str::<LegacyHousehold>(json)
                .map(Household::from)
                .map_err(|e| e.to_string()),
        }
    }

    /// First person's age (the plan runs to their horizon age).
    pub fn age_on(&self, today: NaiveDate) -> Result<i32, String> {
        let p = self.people.first().ok_or("Add at least one person.")?;
        Ok(age_of(&p.birth_date, today)?.1)
    }

    pub fn validate(&self, today: NaiveDate) -> Result<(), String> {
        if self.people.is_empty() || self.people.len() > 4 {
            return Err("A household needs between one and four people.".into());
        }
        for p in &self.people {
            let name = if p.name.trim().is_empty() {
                "A person"
            } else {
                p.name.trim()
            };
            let (_, age) = age_of(&p.birth_date, today)?;
            if !(18..=100).contains(&age) {
                return Err(format!(
                    "{name}: birth date should make them between 18 and 100."
                ));
            }
            if p.retire_age < age {
                return Err(format!(
                    "{name}: retirement age must be at or after their current age."
                ));
            }
            if p.retire_age > 100 {
                return Err(format!("{name}: retirement age is too high."));
            }
            if p.ss_annual_cents < 0 {
                return Err("Amounts cannot be negative.".into());
            }
        }
        if self.streams.len() > 20 {
            return Err("That is more income sources than Golden can plan with (20).".into());
        }
        for s in &self.streams {
            if s.person >= self.people.len() {
                return Err(format!("{}: choose which person earns it.", s.label));
            }
            if s.annual_cents < 0 || s.contribution_cents < 0 {
                return Err("Amounts cannot be negative.".into());
            }
            if let (Some(a), Some(b)) = (s.start_year, s.end_year) {
                if b < a {
                    return Err(format!(
                        "{}: the last year is before the first year.",
                        s.label
                    ));
                }
            }
            for y in [s.start_year, s.end_year].into_iter().flatten() {
                if y < today.year() - 1 || y > today.year() + 80 {
                    return Err(format!(
                        "{}: pick years between now and {}.",
                        s.label,
                        today.year() + 80
                    ));
                }
            }
        }
        if self.state.len() != 2 || !self.state.chars().all(|c| c.is_ascii_alphabetic()) {
            return Err("State should be a two-letter code like TX.".into());
        }
        if [
            self.spending_cents,
            self.cash_cents,
            self.taxable_cents,
            self.traditional_cents,
            self.roth_cents,
        ]
        .iter()
        .any(|c| *c < 0)
        {
            return Err("Amounts cannot be negative.".into());
        }
        let defaults = Assumptions::defaults();
        for (k, label) in &self.picks {
            let p = defaults.get(k).map_err(|e| e.to_string())?;
            if !p.options.iter().any(|o| &o.label == label) {
                return Err(format!("'{k}' has no option '{label}'."));
            }
        }
        Ok(())
    }

    /// Build the engine's plan: every person, every income stream, Social Security per person.
    pub fn plan_input(&self, today: NaiveDate) -> Result<PlanInput, String> {
        self.validate(today)?;
        let year = today.year();
        let filing: FilingStatus =
            serde_json::from_value(serde_json::Value::String(self.filing.clone()))
                .map_err(|_| "Choose a filing status.".to_string())?;
        let mut people = vec![];
        let mut retire_year = vec![];
        let mut ss = vec![];
        for (i, p) in self.people.iter().enumerate() {
            let (birth, age) = age_of(&p.birth_date, today)?;
            people.push(Person {
                id: format!("p{i}"),
                name: p.name.trim().to_string(),
                birth_date: birth,
                is_dependent: false,
            });
            // Last year of work: the year before they reach retirement age.
            retire_year.push(year + (p.retire_age - age) - 1);
            if p.ss_annual_cents > 0 {
                ss.push(SocialSecurity {
                    person: i,
                    annual_benefit: Money::from_cents(p.ss_annual_cents),
                    claim_age: 67,
                });
            }
        }
        let mut streams = vec![];
        for s in &self.streams {
            let start = s.start_year.unwrap_or(year);
            let end = s.end_year.unwrap_or(retire_year[s.person]);
            if end < start || s.annual_cents == 0 {
                continue; // already retired, or nothing earned
            }
            streams.push(IncomeStream {
                label: s.label.clone(),
                person: s.person,
                annual_gross: Money::from_cents(s.annual_cents),
                start_year: start,
                end_year: end,
                traditional_contribution: Money::from_cents(
                    s.contribution_cents.min(s.annual_cents),
                ),
                self_employed: s.self_employed,
            });
        }
        let home_costs = match &self.home {
            Some(r) => {
                if r.value_cents < 0 || r.basis_cents < 0 || r.mortgage_balance_cents < 0 {
                    return Err("Home amounts cannot be negative.".into());
                }
                if !(0..=3000).contains(&r.mortgage_rate_bp)
                    || !(0..=3000).contains(&r.selling_cost_bp)
                {
                    return Err(
                        "Mortgage rate and selling costs should be between 0 and 30 percent."
                            .into(),
                    );
                }
                fin_plan::housing::yearly_costs(
                    &fin_plan::housing::Home {
                        value: Money::from_cents(r.value_cents),
                        basis: Money::from_cents(r.basis_cents),
                        mortgage_balance: Money::from_cents(r.mortgage_balance_cents),
                        mortgage_rate: r.mortgage_rate_bp as f64 / 10_000.0,
                        mortgage_years_left: r.mortgage_years_left,
                        sell_year: r.sell_year,
                        selling_cost: r.selling_cost_bp as f64 / 10_000.0,
                        replacement_cost: Money::from_cents(r.replacement_cost_cents),
                        rent_after: Money::from_cents(r.rent_after_cents),
                    },
                    filing,
                    year,
                    year + 95 - people.first().map_or(40, |p| age_in(p, today)),
                )
            }
            None => vec![],
        };
        let health_costs = if self.health {
            fin_plan::healthcare::yearly_costs(
                &fin_plan::healthcare::HealthCosts::defaults(),
                &people,
                year,
                95,
            )
        } else {
            vec![]
        };
        Ok(PlanInput {
            start_year: year,
            people,
            filing,
            state: self.state.to_ascii_uppercase(),
            accounts: Accounts {
                cash: Money::from_cents(self.cash_cents),
                taxable: Money::from_cents(self.taxable_cents),
                taxable_gain_share: 0.3,
                traditional: Money::from_cents(self.traditional_cents),
                roth: Money::from_cents(self.roth_cents),
            },
            streams,
            social_security: ss,
            annual_spending: Money::from_cents(self.spending_cents),
            one_time_costs: {
                let mut costs: Vec<fin_plan::projection::OneTimeCost> = self
                    .events
                    .iter()
                    .map(|e| fin_plan::projection::OneTimeCost {
                        year: e.year,
                        amount: Money::from_cents(e.cents),
                    })
                    .collect();
                costs.extend(health_costs);
                costs.extend(home_costs);
                costs
            },
            horizon_age: 95,
            spending_rule: self.spending_rule_value()?,
            withdrawal_order: self.withdrawal_order_value()?,
            roth_conversion: None,
            medicare: self.medicare,
            charity: Money::from_cents(self.giving_cents.max(0)),
        })
    }
}

/// A planned one-off event: a cost when `cents` is positive, an untaxed inflow when negative.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventRow {
    pub label: String,
    pub year: i32,
    pub cents: i64,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OptionRow {
    pub label: String,
    pub value: f64,
    pub likelihood: f64,
    pub probability: f64,
    pub source: String,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AssumptionRow {
    pub key: String,
    pub options: Vec<OptionRow>,
    /// The label in use: the person's pick, else the likeliest option.
    pub active: String,
    pub picked: bool,
}

impl App {
    pub fn household(&self) -> Result<Household, String> {
        match self.db.get_setting(KEY).map_err(|e| e.to_string())? {
            Some(json) => Household::from_json(&json),
            None => Ok(Household::sample()),
        }
    }

    /// Whether the person has saved their own household (as opposed to the sample shown first).
    pub fn household_is_saved(&self) -> Result<bool, String> {
        Ok(self
            .db
            .get_setting(KEY)
            .map_err(|e| e.to_string())?
            .is_some())
    }

    pub fn save_household(&self, h: &Household) -> Result<(), String> {
        self.require_edit()?;
        let mut h = h.clone();
        h.state = h.state.to_ascii_uppercase();
        h.validate((self.clock)())?;
        let json = serde_json::to_string(&h).map_err(|e| e.to_string())?;
        self.db.set_setting(KEY, &json).map_err(|e| e.to_string())
    }

    /// Choose an option for one assumption ("" goes back to the likeliest).
    pub fn set_pick(&self, key: &str, label: &str) -> Result<Household, String> {
        let mut h = self.household()?;
        if label.is_empty() {
            h.picks.remove(key);
        } else {
            h.picks.insert(key.to_string(), label.to_string());
        }
        self.save_household(&h)?;
        self.household()
    }

    pub fn assumptions(&self) -> Result<Vec<AssumptionRow>, String> {
        let h = self.household()?;
        let defaults = Assumptions::defaults();
        let mut rows = vec![];
        for key in defaults.keys() {
            let p = defaults.get(key).map_err(|e| e.to_string())?;
            let probs = p.probabilities();
            let picked = h.picks.get(key);
            let active = picked
                .cloned()
                .unwrap_or_else(|| p.options[p.likeliest()].label.clone());
            rows.push(AssumptionRow {
                key: key.to_string(),
                options: p
                    .options
                    .iter()
                    .zip(probs)
                    .map(|(o, pr)| OptionRow {
                        label: o.label.clone(),
                        value: o.value,
                        likelihood: o.likelihood,
                        probability: pr,
                        source: o.source.clone(),
                    })
                    .collect(),
                active,
                picked: picked.is_some(),
            });
        }
        Ok(rows)
    }

    /// The saved household as an engine plan plus its what-if picks.
    pub fn plan(&self, today: NaiveDate) -> Result<(PlanInput, BTreeMap<String, String>), String> {
        let h = self.household()?;
        Ok((h.plan_input(today)?, h.picks))
    }
}
