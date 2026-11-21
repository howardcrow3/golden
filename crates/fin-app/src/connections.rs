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
                name: "Bank connections (Plaid)".into(),
                host: "plaid.com".into(),
                sends: "Not available yet. Statement files are the only way in for now.".into(),
                in_use: false,
                when: "Off. It will be opt-in".into(),
            },
        ];
        // A local-address model never leaves the computer, so only services that do count here.
        let private = !items.iter().any(|c| c.in_use && c.host != "this computer");
        Ok(Connections { private, items })
    }
}
