//! A record of every model run, so the person can see what the AI did.
//!
//! Only metadata is kept (when, which task, how long, how big, did it work), never the prompt or
//! the reply, and only in memory: closing the app clears it.
use crate::model::Completer;
use serde::Serialize;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Instant;

const KEEP: usize = 200;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ActivityEntry {
    /// Local time, `YYYY-MM-DD HH:MM:SS`.
    pub at: String,
    pub task: String,
    pub ok: bool,
    pub millis: u64,
    pub prompt_chars: usize,
    pub reply_chars: usize,
    /// Why it failed, when it did.
    pub error: Option<String>,
}

#[derive(Clone, Default)]
pub struct ActivityLog(Arc<Mutex<VecDeque<ActivityEntry>>>);

impl ActivityLog {
    pub fn record(&self, e: ActivityEntry) {
        if let Ok(mut q) = self.0.lock() {
            q.push_front(e);
            q.truncate(KEEP);
        }
    }

    /// Newest first.
    pub fn entries(&self) -> Vec<ActivityEntry> {
        self.0
            .lock()
            .map(|q| q.iter().cloned().collect())
            .unwrap_or_default()
    }

    pub fn clear(&self) {
        if let Ok(mut q) = self.0.lock() {
            q.clear();
        }
    }
}

/// Wraps a completer so every call is logged.
pub struct Logged {
    pub inner: Arc<dyn Completer + Sync>,
    pub log: ActivityLog,
}

impl Logged {
    fn run(
        &self,
        task: &str,
        prompt: &str,
        f: impl FnOnce() -> Result<String, String>,
    ) -> Result<String, String> {
        let start = Instant::now();
        let out = f();
        self.log.record(ActivityEntry {
            at: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            task: task.to_string(),
            ok: out.is_ok(),
            millis: start.elapsed().as_millis() as u64,
            prompt_chars: prompt.chars().count(),
            reply_chars: out.as_ref().map(|r| r.chars().count()).unwrap_or(0),
            error: out.as_ref().err().cloned(),
        });
        out
    }
}

impl Completer for Logged {
    fn complete(&self, prompt: &str) -> Result<String, String> {
        self.run("whatif", prompt, || self.inner.complete(prompt))
    }
    fn complete_task(&self, task: &str, prompt: &str) -> Result<String, String> {
        self.run(task, prompt, || self.inner.complete_task(task, prompt))
    }
}
