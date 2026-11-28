//! Acceptance of the terms of use. The text and its version live with the UI
//! (apps/desktop/src/lib/terms.ts); this only remembers which version the person
//! accepted and on what day, so the app can ask again when the version changes.
use crate::App;
use serde::{Deserialize, Serialize};

const KEY: &str = "terms_accepted";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TermsAcceptance {
    pub version: String,
    /// The local day it was accepted, as YYYY-MM-DD.
    pub accepted_on: String,
}

impl App {
    /// The version last accepted on this computer, or None if the terms were never accepted.
    pub fn terms_acceptance(&self) -> Result<Option<TermsAcceptance>, String> {
        match self.db.get_setting(KEY).map_err(|e| e.to_string())? {
            // A damaged value counts as never accepted, so the person is simply asked again.
            Some(raw) => Ok(serde_json::from_str(&raw).ok()),
            None => Ok(None),
        }
    }

    /// Record that the person accepted this version today. Works after the trial ends too,
    /// since accepting the terms is not editing their data.
    pub fn accept_terms(&self, version: &str) -> Result<TermsAcceptance, String> {
        let version = version.trim();
        if version.is_empty() || version.len() > 64 {
            return Err("That terms version isn't valid.".into());
        }
        let a = TermsAcceptance {
            version: version.into(),
            accepted_on: (self.clock)().format("%Y-%m-%d").to_string(),
        };
        let json = serde_json::to_string(&a).map_err(|e| e.to_string())?;
        self.db.set_setting(KEY, &json).map_err(|e| e.to_string())?;
        Ok(a)
    }
}

/// Kept by "delete everything": erasing your data is not withdrawing from the terms.
pub(crate) const TERMS_KEY: &str = KEY;
