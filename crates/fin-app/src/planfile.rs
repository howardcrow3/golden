//! Share a plan as a file: the household, your assumption picks and your holdings, with no
//! account, no cloud and none of your transactions. Send it to a spouse or advisor, who opens it
//! in their own Golden.
use crate::household::Household;
use crate::App;
use fin_plan::investments::Holding;
use serde::{Deserialize, Serialize};

const FORMAT: &str = "golden-plan";
const VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlanFile {
    format: String,
    version: u32,
    household: Household,
    #[serde(default)]
    holdings: Vec<Holding>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Imported {
    pub people: usize,
    pub holdings: usize,
}

impl App {
    /// The saved plan as the text of a plan file.
    pub fn export_plan_file(&self) -> Result<String, String> {
        let f = PlanFile {
            format: FORMAT.into(),
            version: VERSION,
            household: self.household()?,
            holdings: self.holdings()?,
        };
        serde_json::to_string_pretty(&f).map_err(|e| e.to_string())
    }

    /// Replace the saved household (and holdings, when the file has any) with a plan file's.
    pub fn import_plan_file(&self, text: &str) -> Result<Imported, String> {
        self.require_edit()?;
        let f: PlanFile =
            serde_json::from_str(text).map_err(|_| "That isn't a Golden plan file.".to_string())?;
        if f.format != FORMAT {
            return Err("That isn't a Golden plan file.".into());
        }
        if f.version > VERSION {
            return Err("That plan file is from a newer Golden. Update the app to open it.".into());
        }
        // Check everything before changing anything.
        f.household.validate((self.clock)())?;
        let people = f.household.people.len();
        let holdings = f.holdings.len();
        self.save_household(&f.household)?;
        if !f.holdings.is_empty() {
            self.save_holdings(&f.holdings)?;
        }
        Ok(Imported { people, holdings })
    }
}
