//! The Connections page (SC-2): every outside service Golden can talk to, what it is sent, and
//! whether it is in use right now. Golden is private by default: with nothing here in use,
//! nothing leaves the computer.
use crate::App;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Connection {
    pub name: String,
    /// Where it connects ("this computer" when it never leaves).
    pub host: String,
    /// What is sent, in plain words.
    pub sends: String,
    /// True when you have turned it on or are using it now.
    pub in_use: bool,
    /// Always runs, or only when you ask?
    pub when: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Connections {
    /// True when nothing on the list is in use, so no financial data leaves this computer.
    pub private: bool,
    pub items: Vec<Connection>,
}

impl App {
    pub fn connections(&self) -> Result<Connections, String> {
        let saved = self.model_settings()?;
        let local_model = !saved.endpoint.trim().is_empty();
        let banks_on = self.bank_items()?.iter().any(|i| i.status == "active");
        let items = vec![
            Connection {
                name: "Built-in model download".into(),
                host: "huggingface.co".into(),
                sends: "A request for the model file, which shows your internet address. None of your financial data.".into(),
                in_use: false,
                when: "Only when you press Download".into(),
            },
            Connection {
                name: "Model on this computer".into(),
                host: "this computer".into(),
                sends: "Questions and scrubbed text to a model you run yourself. It never leaves this computer.".into(),
                in_use: local_model,
                when: "Only if you set a local model address".into(),
            },
            Connection {
                name: "Buy or manage your licence".into(),
                host: "turtlebox.studio".into(),
                sends: "Nothing from Golden. Your browser opens the store page.".into(),
                in_use: false,
                when: "Only when you press Buy".into(),
            },
            Connection {
                name: "Send feedback".into(),
                host: "feedback.turtlebox.studio".into(),
                sends: "Only what you write in the feedback form: your star rating, your words, and your email if you add it, with Golden's version and your operating system. None of your financial data.".into(),
                in_use: false,
                when: "Only when you press Send".into(),
            },
            Connection {
                name: "Bank connections (Plaid)".into(),
                host: "Golden's bank sync service, then plaid.com".into(),
                sends: "Your licence key and a bank sign-in token go to Golden's sync service, which asks Plaid for your transactions and passes them back to this computer. Plaid sees your bank sign-in; Golden never does. Needs the Plaid add-on.".into(),
                in_use: banks_on,
                when: "Off until you connect a bank. Then about once a day, or when you press Sync".into(),
            },
        ];
        // A local-address model never leaves the computer, so only services that do count here.
        let private = !items.iter().any(|c| c.in_use && c.host != "this computer");
        Ok(Connections { private, items })
    }
}

/// The word the person must type to erase everything.
pub const ERASE_WORD: &str = "DELETE";

impl App {
    /// SC-1: delete every account, transaction, household member, budget, rule and saved
    /// setting in one step. The trial start and licence are kept so erasing can't reset the trial.
    /// Statement files already filed in the library folder are not touched.
    pub fn erase_all_data(&mut self, confirm: &str) -> Result<(), String> {
        if confirm.trim() != ERASE_WORD {
            return Err(format!("Type {ERASE_WORD} to confirm."));
        }
        // Erasing the settings would forget a linked bank while its access token stays in the
        // keychain and the Plaid connection stays live, so those must be unlinked first.
        if !self.bank_items()?.is_empty() {
            return Err(
                "Unlink your banks on the Banks page first, so their sign-ins are removed too. Then erase."
                    .into(),
            );
        }
        self.db
            .erase_personal_data(crate::licensing::KEEP_ON_ERASE)
            .map_err(|e| e.to_string())?;
        self.rules = crate::RuleSet::starter();
        self.library_root = self.library_default.clone();
        self.refresh_interpreter();
        Ok(())
    }
}
