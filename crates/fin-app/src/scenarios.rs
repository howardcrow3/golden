//! Named scenarios (PR-4): keep a what-if, stack changes, compare up to three beside the current
//! plan, and make one the plan in a single step with undo.
use crate::chat::Change;
use crate::household::Household;
use crate::App;
use chrono::NaiveDate;
use fin_plan::quickplan::{self, QuickPlanResult};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const KEY: &str = "scenarios";
const UNDO_KEY: &str = "plan_undo";
pub const MAX_SCENARIOS: usize = 20;
pub const MAX_COMPARE: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedScenario {
    pub id: u32,
    pub name: String,
    pub changes: Vec<Change>,
    /// What each change does, in words, as of when it was saved.
    pub descriptions: Vec<String>,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CompareColumn {
    pub name: String,
    pub descriptions: Vec<String>,
    pub result: QuickPlanResult,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CompareReply {
    /// The current plan first, then each chosen scenario in the order given.
    pub columns: Vec<CompareColumn>,
}

type Plan = (fin_plan::projection::PlanInput, BTreeMap<String, String>);

/// Everything needed to compare scenarios without holding the app lock.
pub struct ComparePrep {
    columns: Vec<(String, Vec<String>, Plan)>,
}

impl ComparePrep {
    pub fn run(self, runs: usize) -> Result<CompareReply, String> {
        let mut columns = vec![];
        for (name, descriptions, (plan, picks)) in self.columns {
            columns.push(CompareColumn {
                name,
                descriptions,
                result: quickplan::run_plan(&plan, &picks, runs)?,
            });
        }
        Ok(CompareReply { columns })
    }
}

#[derive(Serialize, Deserialize)]
struct Undo {
    name: String,
    household: String,
}

fn apply_all(base: &Household, changes: &[Change]) -> Result<Household, String> {
    let mut h = base.clone();
    for c in changes {
        c.apply(&mut h)?;
    }
    Ok(h)
}

impl App {
    pub fn list_scenarios(&self) -> Result<Vec<SavedScenario>, String> {
        match self.db.get_setting(KEY).map_err(|e| e.to_string())? {
            Some(json) => serde_json::from_str(&json).map_err(|e| e.to_string()),
            None => Ok(vec![]),
        }
    }

    fn store_scenarios(&self, list: &[SavedScenario]) -> Result<(), String> {
        let json = serde_json::to_string(list).map_err(|e| e.to_string())?;
        self.db.set_setting(KEY, &json).map_err(|e| e.to_string())
    }

    /// Save a what-if's changes under a name. The changes must work on the current household.
    pub fn save_scenario(
        &self,
        name: &str,
        changes: &[Change],
        today: NaiveDate,
    ) -> Result<SavedScenario, String> {
        self.require_edit()?;
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 60 {
            return Err("Give the scenario a name of up to 60 characters.".into());
        }
        if changes.is_empty() {
            return Err("There is nothing to save: this what-if changes nothing.".into());
        }
        let mut list = self.list_scenarios()?;
        if list.len() >= MAX_SCENARIOS {
            return Err(format!(
                "You can keep up to {MAX_SCENARIOS} scenarios. Delete one first."
            ));
        }
        if list.iter().any(|s| s.name.eq_ignore_ascii_case(name)) {
            return Err("A scenario with that name already exists.".into());
        }
        let h = self.household()?;
        let alt = apply_all(&h, changes)?;
        alt.plan_input(today)
            .map_err(|e| format!("That change doesn't work: {e}"))?;
        let saved = SavedScenario {
            id: list.iter().map(|s| s.id).max().unwrap_or(0) + 1,
            name: name.to_string(),
            changes: changes.to_vec(),
            descriptions: changes.iter().map(|c| c.describe(&h)).collect(),
        };
        list.push(saved.clone());
        self.store_scenarios(&list)?;
        Ok(saved)
    }

    pub fn delete_scenario(&self, id: u32) -> Result<(), String> {
        self.require_edit()?;
        let mut list = self.list_scenarios()?;
        let before = list.len();
        list.retain(|s| s.id != id);
        if list.len() == before {
            return Err("That scenario no longer exists.".into());
        }
        self.store_scenarios(&list)
    }

    /// Give a scenario a new name (up to 60 characters, unique ignoring case).
    pub fn rename_scenario(&self, id: u32, name: &str) -> Result<SavedScenario, String> {
        self.require_edit()?;
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 60 {
            return Err("Give the scenario a name of up to 60 characters.".into());
        }
        let mut list = self.list_scenarios()?;
        if list
            .iter()
            .any(|s| s.id != id && s.name.eq_ignore_ascii_case(name))
        {
            return Err("A scenario with that name already exists.".into());
        }
        let found = list
            .iter_mut()
            .find(|s| s.id == id)
            .ok_or("That scenario no longer exists.")?;
        found.name = name.to_string();
        let out = found.clone();
        self.store_scenarios(&list)?;
        Ok(out)
    }

    /// Copy a scenario under "<name> copy" (numbered if that is taken), right after the original.
    pub fn duplicate_scenario(&self, id: u32) -> Result<SavedScenario, String> {
        self.require_edit()?;
        let mut list = self.list_scenarios()?;
        if list.len() >= MAX_SCENARIOS {
            return Err(format!(
                "You can keep up to {MAX_SCENARIOS} scenarios. Delete one first."
            ));
        }
        let at = list
            .iter()
            .position(|s| s.id == id)
            .ok_or("That scenario no longer exists.")?;
        let original = list[at].clone();
        let taken = |n: &str| list.iter().any(|s| s.name.eq_ignore_ascii_case(n));
        let base: String = original.name.chars().take(54).collect();
        let mut name = format!("{base} copy");
        let mut n = 2;
        while taken(&name) {
            name = format!("{base} copy {n}");
            n += 1;
        }
        let copy = SavedScenario {
            id: list.iter().map(|s| s.id).max().unwrap_or(0) + 1,
            name,
            ..original
        };
        list.insert(at + 1, copy.clone());
        self.store_scenarios(&list)?;
        Ok(copy)
    }

    /// Move a scenario up (negative) or down (positive) the list by `by` places, stopping at the ends.
    pub fn move_scenario(&self, id: u32, by: i32) -> Result<Vec<SavedScenario>, String> {
        self.require_edit()?;
        let mut list = self.list_scenarios()?;
        let at = list
            .iter()
            .position(|s| s.id == id)
            .ok_or("That scenario no longer exists.")?;
        let to = (at as i64 + by as i64).clamp(0, list.len() as i64 - 1) as usize;
        let item = list.remove(at);
        list.insert(to, item);
        self.store_scenarios(&list)?;
        Ok(list)
    }

    /// Set up a side-by-side of the current plan and up to three saved scenarios.
    pub fn prepare_compare(&self, ids: &[u32], today: NaiveDate) -> Result<ComparePrep, String> {
        if ids.is_empty() {
            return Err("Choose at least one scenario to compare.".into());
        }
        if ids.len() > MAX_COMPARE {
            return Err(format!(
                "You can compare up to {MAX_COMPARE} scenarios at once."
            ));
        }
        let list = self.list_scenarios()?;
        let h = self.household()?;
        let mut columns = vec![(
            "Current plan".to_string(),
            vec![],
            (h.plan_input(today)?, h.picks.clone()),
        )];
        for id in ids {
            let s = list
                .iter()
                .find(|s| s.id == *id)
                .ok_or("That scenario no longer exists.")?;
            let alt = apply_all(&h, &s.changes)?;
            let plan = alt
                .plan_input(today)
                .map_err(|e| format!("{} no longer works: {e}", s.name))?;
            columns.push((
                s.name.clone(),
                s.descriptions.clone(),
                (plan, alt.picks.clone()),
            ));
        }
        Ok(ComparePrep { columns })
    }

    /// Make a scenario the plan: its changes are saved into the household, and the household as
    /// it was is kept so `undo_make_plan` can put it back.
    pub fn make_plan(&self, id: u32) -> Result<Household, String> {
        self.require_edit()?;
        let s = self
            .list_scenarios()?
            .into_iter()
            .find(|s| s.id == id)
            .ok_or("That scenario no longer exists.")?;
        let h = self.household()?;
        let json = serde_json::to_string(&h).map_err(|e| e.to_string())?;
        let alt = apply_all(&h, &s.changes)?;
        self.save_household(&alt)?;
        let undo = serde_json::to_string(&Undo {
            name: s.name,
            household: json,
        })
        .map_err(|e| e.to_string())?;
        self.db
            .set_setting(UNDO_KEY, &undo)
            .map_err(|e| e.to_string())?;
        self.household()
    }

    /// The name of the scenario whose "make this the plan" can be undone, if any.
    pub fn undoable_plan(&self) -> Result<Option<String>, String> {
        Ok(self
            .db
            .get_setting(UNDO_KEY)
            .map_err(|e| e.to_string())?
            .and_then(|j| serde_json::from_str::<Undo>(&j).ok())
            .map(|u| u.name))
    }

    /// Put the household back as it was before the last "make this the plan".
    pub fn undo_make_plan(&self) -> Result<Household, String> {
        self.require_edit()?;
        let json = self
            .db
            .get_setting(UNDO_KEY)
            .map_err(|e| e.to_string())?
            .ok_or("There is nothing to undo.")?;
        let undo: Undo = serde_json::from_str(&json).map_err(|e| e.to_string())?;
        let h = Household::from_json(&undo.household)?;
        self.save_household(&h)?;
        self.db
            .set_setting(UNDO_KEY, "")
            .map_err(|e| e.to_string())?;
        self.household()
    }
}
