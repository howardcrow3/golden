//! A language model that runs on this computer, reached over loopback HTTP.
//!
//! Private mode means nothing leaves the machine, so the only endpoints accepted are `http://`
//! addresses on 127.0.0.1, localhost or ::1. Two common local servers are understood: Ollama
//! (`/api/generate`) and anything OpenAI-compatible such as llama.cpp or LM Studio
//! (`/v1/chat/completions`). The model is only ever asked the what-if question (see `model.rs`
//! for exactly what it is shown); if the server is off or answers badly the chat quietly falls
//! back to the built-in phrase understanding.
use crate::model::{Completer, ModelInterpreter};
use crate::App;
use serde::Serialize;
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;

const ENDPOINT_KEY: &str = "model_endpoint";
const NAME_KEY: &str = "model_name";
const MAX_RESPONSE: u64 = 1_000_000;
const TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    Ollama,
    OpenAi,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoint {
    pub host: String,
    pub port: u16,
    pub path: String,
    pub style: Style,
}

const ENDPOINT_HELP: &str = "Use an address on this computer, such as http://127.0.0.1:11434/api/generate (Ollama) or http://127.0.0.1:8080/v1/chat/completions.";

/// Check and split a model address. Only plain-HTTP loopback addresses are accepted.
pub fn parse_endpoint(url: &str) -> Result<Endpoint, String> {
    let rest = url
        .trim()
        .strip_prefix("http://")
        .ok_or_else(|| format!("The model address must start with http://. {ENDPOINT_HELP}"))?;
    let (authority, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, "/"),
    };
    if authority.contains('@') || authority.is_empty() {
        return Err(format!("That isn't a usable address. {ENDPOINT_HELP}"));
    }
    let (host, port) = if let Some(h) = authority.strip_prefix('[') {
        let (h, after) = h
            .split_once(']')
            .ok_or_else(|| format!("That isn't a usable address. {ENDPOINT_HELP}"))?;
        (h.to_string(), after.strip_prefix(':'))
    } else {
        match authority.split_once(':') {
            Some((h, p)) => (h.to_string(), Some(p)),
            None => (authority.to_string(), None),
        }
    };
    if !matches!(
        host.to_ascii_lowercase().as_str(),
        "127.0.0.1" | "localhost" | "::1"
    ) {
        return Err(format!(
            "Golden only talks to a model running on this computer, so private mode stays private. {ENDPOINT_HELP}"
        ));
    }
    let port = match port {
        None => 80,
        Some(p) => p
            .parse::<u16>()
            .map_err(|_| format!("The port in that address isn't a number. {ENDPOINT_HELP}"))?,
    };
    let route = path.split(['?', '#']).next().unwrap_or("/");
    let style = if route.ends_with("/api/generate") {
        Style::Ollama
    } else if route.ends_with("/chat/completions") {
        Style::OpenAi
    } else {
        return Err(format!(
            "I don't know that kind of model address. {ENDPOINT_HELP}"
        ));
    };
    Ok(Endpoint {
        host,
        port,
        path: route.to_string(),
        style,
    })
}

fn decode_chunked(mut body: &[u8]) -> Result<Vec<u8>, String> {
    let bad = || "The model sent a reply I couldn't read.".to_string();
    let mut out = vec![];
    loop {
        let line_end = body.windows(2).position(|w| w == b"\r\n").ok_or_else(bad)?;
        let size_text = std::str::from_utf8(&body[..line_end]).map_err(|_| bad())?;
        let size = usize::from_str_radix(size_text.split(';').next().unwrap_or("").trim(), 16)
            .map_err(|_| bad())?;
        body = &body[line_end + 2..];
        if size == 0 {
            return Ok(out);
        }
        if body.len() < size + 2 {
            return Err(bad());
        }
        out.extend_from_slice(&body[..size]);
        body = &body[size + 2..];
    }
}

/// Split a raw HTTP/1.1 response into its body, requiring a 200.
pub fn response_body(raw: &[u8]) -> Result<Vec<u8>, String> {
    let split = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or("The model sent a reply I couldn't read.")?;
    let head = String::from_utf8_lossy(&raw[..split]).to_string();
    let body = &raw[split + 4..];
    let status_line = head.lines().next().unwrap_or("");
    if status_line.split_whitespace().nth(1) != Some("200") {
        return Err(format!("The model answered \"{}\".", status_line.trim()));
    }
    if head
        .to_ascii_lowercase()
        .contains("transfer-encoding: chunked")
    {
        decode_chunked(body)
    } else {
        Ok(body.to_vec())
    }
}

fn post(ep: &Endpoint, json_body: &str) -> Result<Vec<u8>, String> {
    let down = || "I couldn't reach the model. Is it running?".to_string();
    // Resolve the name ourselves and keep only loopback results, so a hosts-file trick can't
    // point "localhost" somewhere else.
    let addrs: Vec<SocketAddr> = (ep.host.as_str(), ep.port)
        .to_socket_addrs()
        .map_err(|_| down())?
        .filter(|a| a.ip().is_loopback())
        .collect();
    let mut stream = addrs
        .iter()
        .find_map(|a| TcpStream::connect_timeout(a, Duration::from_secs(3)).ok())
        .ok_or_else(down)?;
    stream.set_read_timeout(Some(TIMEOUT)).ok();
    stream.set_write_timeout(Some(TIMEOUT)).ok();
    let request = format!(
        "POST {} HTTP/1.1\r\nHost: {}:{}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        ep.path,
        ep.host,
        ep.port,
        json_body.len(),
        json_body
    );
    stream.write_all(request.as_bytes()).map_err(|_| down())?;
    let mut raw = vec![];
    stream
        .take(MAX_RESPONSE)
        .read_to_end(&mut raw)
        .map_err(|_| "The model took too long to answer.".to_string())?;
    response_body(&raw)
}

/// Asks a local model server for a completion.
struct Shared(std::sync::Arc<dyn Completer + Sync>);

impl Completer for Shared {
    fn complete(&self, prompt: &str) -> Result<String, String> {
        self.0.complete(prompt)
    }
    fn complete_task(&self, task: &str, prompt: &str) -> Result<String, String> {
        self.0.complete_task(task, prompt)
    }
}

pub struct LocalCompleter {
    endpoint: Endpoint,
    model: String,
}

impl LocalCompleter {
    pub fn new(endpoint: Endpoint, model: &str) -> LocalCompleter {
        LocalCompleter {
            endpoint,
            model: model.trim().to_string(),
        }
    }

    pub fn request_body(&self, prompt: &str) -> String {
        match self.endpoint.style {
            Style::Ollama => json!({
                "model": self.model, "prompt": prompt, "stream": false,
                "format": "json", "options": { "temperature": 0 }
            }),
            Style::OpenAi => json!({
                "model": self.model, "temperature": 0,
                "messages": [{ "role": "user", "content": prompt }]
            }),
        }
        .to_string()
    }

    pub fn reply_text(&self, body: &[u8]) -> Result<String, String> {
        let v: Value = serde_json::from_slice(body)
            .map_err(|_| "The model sent a reply I couldn't read.".to_string())?;
        let text = match self.endpoint.style {
            Style::Ollama => v.get("response"),
            Style::OpenAi => v.pointer("/choices/0/message/content"),
        };
        text.and_then(Value::as_str)
            .map(String::from)
            .ok_or_else(|| "The model's reply had no text in it.".to_string())
    }
}

impl Completer for LocalCompleter {
    fn complete(&self, prompt: &str) -> Result<String, String> {
        let body = post(&self.endpoint, &self.request_body(prompt))?;
        self.reply_text(&body)
    }
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelSettings {
    pub endpoint: String,
    pub name: String,
}

impl App {
    pub fn model_settings(&self) -> Result<ModelSettings, String> {
        let get = |k: &str| {
            self.db
                .get_setting(k)
                .map_err(|e| e.to_string())
                .map(|v| v.unwrap_or_default())
        };
        Ok(ModelSettings {
            endpoint: get(ENDPOINT_KEY)?,
            name: get(NAME_KEY)?,
        })
    }

    /// Save which local model to use (an empty address turns it off) and switch the what-if chat
    /// to it straight away.
    pub fn set_model(&mut self, endpoint: &str, name: &str) -> Result<ModelSettings, String> {
        self.require_edit()?;
        let endpoint = endpoint.trim();
        if !endpoint.is_empty() {
            parse_endpoint(endpoint)?;
            if name.trim().is_empty() {
                return Err("Give the model's name, for example llama3.2.".into());
            }
        }
        self.db
            .set_setting(ENDPOINT_KEY, endpoint)
            .map_err(|e| e.to_string())?;
        self.db
            .set_setting(NAME_KEY, name.trim())
            .map_err(|e| e.to_string())?;
        self.refresh_interpreter();
        self.model_settings()
    }

    /// Use a model running inside the app (or stop using it with `None`).
    pub fn set_embedded_model(&mut self, c: Option<std::sync::Arc<dyn Completer + Sync>>) {
        self.embedded = c;
        self.refresh_interpreter();
    }

    /// Every model run this session, newest first (sizes and timings only, never the text).
    pub fn ai_activity(&self) -> Vec<crate::activity::ActivityEntry> {
        self.activity.entries()
    }

    pub fn clear_ai_activity(&self) {
        self.activity.clear();
    }

    /// Use the saved local model for the what-if chat and the AI helpers, or the built-in phrases
    /// when none is set (or the saved address is no longer acceptable). Call once after opening
    /// the database.
    pub fn refresh_interpreter(&mut self) {
        let ai: Option<std::sync::Arc<dyn Completer + Sync>> = match &self.embedded {
            Some(c) => Some(c.clone()),
            None => self.model_settings().ok().and_then(|s| {
                let ep = parse_endpoint(&s.endpoint).ok()?;
                (!s.name.trim().is_empty())
                    .then(|| std::sync::Arc::new(LocalCompleter::new(ep, &s.name)) as _)
            }),
        };
        let ai: Option<std::sync::Arc<dyn Completer + Sync>> = ai.map(|inner| {
            std::sync::Arc::new(crate::activity::Logged {
                inner,
                log: self.activity.clone(),
            }) as _
        });
        self.interpreter = match &ai {
            Some(c) => Box::new(ModelInterpreter::new(Box::new(Shared(c.clone())))),
            None => Box::new(crate::chat::RuleInterpreter),
        };
        self.ai = ai;
    }
}
