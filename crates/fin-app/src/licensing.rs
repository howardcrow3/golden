//! Trial and licence state, stored in the database settings table.
use crate::App;
use chrono::NaiveDate;
use fin_license::{access, plaid_active, verify, Access, LicensePayload, APP_MAJOR};
use serde::Serialize;

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LicenseStatus {
    /// "trial", "licensed" or "expired"
    pub state: String,
    pub days_left: Option<i64>,
    pub can_edit: bool,
    pub plaid_active: bool,
}

const FIRST_RUN: &str = "first_run";
const LICENSE: &str = "license_token";
/// A key for the bank-connection add-on alone (`core: false`), kept beside the app's key so a
/// renewal of the add-on never replaces the key that unlocks the app.
const ADDON: &str = "license_addon_token";
/// Settings that survive "delete everything" so erasing cannot reset the trial or drop a licence.
pub(crate) const KEEP_ON_ERASE: &[&str] = &[FIRST_RUN, LICENSE, ADDON];

impl App {
    /// Every saved key that verifies, with its token.
    fn stored_keys(&self) -> Vec<(String, LicensePayload)> {
        let Some(key) = self.public_key.as_ref() else {
            return vec![];
        };
        [LICENSE, ADDON]
            .iter()
            .filter_map(|slot| self.db.get_setting(slot).ok().flatten())
            .filter_map(|t| verify(key, &t, APP_MAJOR).ok().map(|p| (t, p)))
            .collect()
    }

    /// The key that unlocks the app, if one is saved.
    fn app_key(&self) -> Option<LicensePayload> {
        self.stored_keys()
            .into_iter()
            .map(|(_, p)| p)
            .find(LicensePayload::is_core)
    }

    /// The saved key with the latest add-on date, for the bank sync service to check.
    fn addon_key(&self) -> Option<(String, LicensePayload)> {
        self.stored_keys()
            .into_iter()
            .filter(|(_, p)| p.plaid_until.is_some())
            .max_by_key(|(_, p)| p.plaid_until)
    }

    /// The saved licence token, for the bank sync service to check. None when there is none.
    pub(crate) fn license_token(&self) -> Option<String> {
        self.addon_key()
            .map(|(t, _)| t)
            .or_else(|| self.db.get_setting(LICENSE).ok()?)
    }

    fn current_access(&self, today: NaiveDate) -> Result<(Access, Option<LicensePayload>), String> {
        let first = self
            .db
            .set_setting_if_absent(FIRST_RUN, &today.format("%Y-%m-%d").to_string())
            .map_err(|e| e.to_string())?;
        let first = NaiveDate::parse_from_str(&first, "%Y-%m-%d").unwrap_or(today);
        Ok((
            access(first, today, self.app_key().as_ref()),
            self.app_key(),
        ))
    }

    pub fn license_status(&self, today: NaiveDate) -> Result<LicenseStatus, String> {
        let (a, _) = self.current_access(today)?;
        let (state, days_left) = match &a {
            Access::Trial { days_left } => ("trial", Some(*days_left)),
            Access::Licensed => ("licensed", None),
            Access::Expired => ("expired", None),
        };
        Ok(LicenseStatus {
            state: state.into(),
            days_left,
            can_edit: a.can_edit(),
            plaid_active: plaid_active(self.addon_key().map(|(_, p)| p).as_ref(), today),
        })
    }

    /// Check a pasted licence and keep it if valid for this version of the app.
    pub fn activate_license(&self, token: &str, today: NaiveDate) -> Result<LicenseStatus, String> {
        let key = self
            .public_key
            .as_ref()
            .ok_or("Licence activation is not available in this build yet.")?;
        let new = verify(key, token, APP_MAJOR).map_err(|e| e.to_string())?;
        let stored = self.stored_keys();
        if new.is_core() {
            // Never downgrade: re-opening an older email (the core key, or last month's
            // renewal) must not drop Plaid time the person already paid for.
            let keeps_current = stored
                .iter()
                .any(|(_, cur)| cur.is_core() && cur.plaid_until > new.plaid_until);
            if !keeps_current {
                // An add-on key saved in the app's place (by a version that had only one
                // place) moves aside first, so its paid-for bank time is kept.
                let in_app_slot = self.db.get_setting(LICENSE).map_err(|e| e.to_string())?;
                if let Some((t, p)) = stored.iter().find(|(t, _)| Some(t) == in_app_slot.as_ref()) {
                    let addon_later = stored
                        .iter()
                        .all(|(other, o)| other == t || o.plaid_until < p.plaid_until);
                    if !p.is_core() && addon_later {
                        self.db.set_setting(ADDON, t).map_err(|e| e.to_string())?;
                    }
                }
                self.db
                    .set_setting(LICENSE, token.trim())
                    .map_err(|e| e.to_string())?;
            }
        } else {
            // An add-on key goes in its own place, so it can never take the app away, and only
            // when it pays for later than any key already saved.
            let newer = stored
                .iter()
                .all(|(_, cur)| cur.plaid_until < new.plaid_until);
            if newer {
                self.db
                    .set_setting(ADDON, token.trim())
                    .map_err(|e| e.to_string())?;
            }
        }
        self.license_status(today)
    }

    pub(crate) fn require_edit(&self) -> Result<(), String> {
        let (a, _) = self.current_access((self.clock)())?;
        if a.can_edit() {
            Ok(())
        } else {
            Err("Your 14-day trial has ended. Your data is safe and exports still work; enter a licence to keep importing and editing.".into())
        }
    }
}
