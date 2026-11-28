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
/// Settings that survive "delete everything" so erasing cannot reset the trial, drop a licence
/// or ask for the terms again.
pub(crate) const KEEP_ON_ERASE: &[&str] = &[FIRST_RUN, LICENSE, crate::terms::TERMS_KEY];

impl App {
    fn stored_license(&self) -> Option<LicensePayload> {
        let key = self.public_key.as_ref()?;
        let token = self.db.get_setting(LICENSE).ok()??;
        verify(key, &token, APP_MAJOR).ok()
    }

    /// The saved licence token, for the bank sync service to check. None when there is none.
    pub(crate) fn license_token(&self) -> Option<String> {
        self.db.get_setting(LICENSE).ok()?
    }

    fn current_access(&self, today: NaiveDate) -> Result<(Access, Option<LicensePayload>), String> {
        let first = self
            .db
            .set_setting_if_absent(FIRST_RUN, &today.format("%Y-%m-%d").to_string())
            .map_err(|e| e.to_string())?;
        let first = NaiveDate::parse_from_str(&first, "%Y-%m-%d").unwrap_or(today);
        let lic = self.stored_license();
        Ok((access(first, today, lic.as_ref()), lic))
    }

    pub fn license_status(&self, today: NaiveDate) -> Result<LicenseStatus, String> {
        let (a, lic) = self.current_access(today)?;
        let (state, days_left) = match &a {
            Access::Trial { days_left } => ("trial", Some(*days_left)),
            Access::Licensed => ("licensed", None),
            Access::Expired => ("expired", None),
        };
        Ok(LicenseStatus {
            state: state.into(),
            days_left,
            can_edit: a.can_edit(),
            plaid_active: plaid_active(lic.as_ref(), today),
        })
    }

    /// Check a pasted licence and keep it if valid for this version of the app.
    pub fn activate_license(&self, token: &str, today: NaiveDate) -> Result<LicenseStatus, String> {
        let key = self
            .public_key
            .as_ref()
            .ok_or("Licence activation is not available in this build yet.")?;
        let new = verify(key, token, APP_MAJOR).map_err(|e| e.to_string())?;
        // Never downgrade: re-opening an older email (the core key, or last month's
        // renewal) must not drop Plaid time the person already paid for.
        let keeps_current = self
            .stored_license()
            .is_some_and(|cur| cur.plaid_until > new.plaid_until);
        if !keeps_current {
            self.db
                .set_setting(LICENSE, token.trim())
                .map_err(|e| e.to_string())?;
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
