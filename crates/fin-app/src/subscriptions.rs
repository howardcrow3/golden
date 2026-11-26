//! A short list of regular payments you have decided to cancel, so they stay in view until
//! they actually stop. Golden never contacts anyone on your behalf.
use crate::App;

const KEY: &str = "cancel_marks";

impl App {
    /// Payees marked "to cancel", sorted.
    pub fn cancel_marks(&self) -> Result<Vec<String>, String> {
        match self.db.get_setting(KEY).map_err(|e| e.to_string())? {
            Some(s) => {
                serde_json::from_str(&s).map_err(|e| format!("saved list is unreadable: {e}"))
            }
            None => Ok(vec![]),
        }
    }

    /// Mark or unmark a payee as something to cancel.
    pub fn set_cancel_mark(&self, payee: &str, on: bool) -> Result<Vec<String>, String> {
        self.require_edit()?;
        let payee = payee.trim();
        if payee.is_empty() {
            return Err("Choose a payment first.".into());
        }
        let mut v = self.cancel_marks()?;
        v.retain(|p| p != payee);
        if on {
            v.push(payee.to_string());
        }
        v.sort();
        let json = serde_json::to_string(&v).map_err(|e| e.to_string())?;
        self.db.set_setting(KEY, &json).map_err(|e| e.to_string())?;
        Ok(v)
    }
}
