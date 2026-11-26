//! Watch lists: named searches (a payee, a category, a tag) kept on the Overview so you can see
//! what a particular kind of spending adds up to.
use crate::App;
use serde::{Deserialize, Serialize};

const KEY: &str = "watch_lists";
const MAX_LISTS: usize = 20;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WatchList {
    pub name: String,
    /// Words that must all appear in a transaction's payee, memo, note, tags, category or account.
    pub query: String,
}

impl App {
    pub fn watch_lists(&self) -> Result<Vec<WatchList>, String> {
        match self.db.get_setting(KEY).map_err(|e| e.to_string())? {
            Some(s) => serde_json::from_str(&s)
                .map_err(|e| format!("saved watch lists are unreadable: {e}")),
            None => Ok(vec![]),
        }
    }

    /// Replace the saved lists. Names must be unique and both fields filled in.
    pub fn save_watch_lists(&self, lists: Vec<WatchList>) -> Result<Vec<WatchList>, String> {
        self.require_edit()?;
        if lists.len() > MAX_LISTS {
            return Err(format!("Keep it to {MAX_LISTS} watch lists or fewer."));
        }
        let mut clean: Vec<WatchList> = vec![];
        for l in lists {
            let (name, query) = (l.name.trim().to_string(), l.query.trim().to_string());
            if name.is_empty() || query.is_empty() {
                return Err("Give each watch list a name and something to look for.".into());
            }
            if clean.iter().any(|c| c.name.eq_ignore_ascii_case(&name)) {
                return Err(format!("You already have a watch list called {name}."));
            }
            clean.push(WatchList { name, query });
        }
        let json = serde_json::to_string(&clean).map_err(|e| e.to_string())?;
        self.db.set_setting(KEY, &json).map_err(|e| e.to_string())?;
        Ok(clean)
    }
}
