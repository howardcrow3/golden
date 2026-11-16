//! A read-only Model Context Protocol server core (JSON-RPC 2.0, one message per line).
//!
//! `handle` takes one request line and returns the response line, or `None` for notifications.
//! It is deliberately read-only: no tool can import, change, delete or unlock anything, so an
//! assistant pointed at Golden can look at the numbers but never alter them. Transport (stdio)
//! lives in the caller.
use crate::App;
use serde_json::{json, Value};

const PROTOCOL: &str = "2024-11-05";

fn tools() -> Value {
    let s = |props: Value, req: &[&str]| json!({"type":"object","properties":props,"required":req});
    json!([
        {"name":"accounts","description":"List accounts with balances (cents).","inputSchema":s(json!({}),&[])},
        {"name":"balance_sheet","description":"Assets, debts and net worth (cents).","inputSchema":s(json!({}),&[])},
        {"name":"cash_flow","description":"Income, spending and net per period between two dates.",
         "inputSchema":s(json!({"from":{"type":"string","description":"YYYY-MM-DD"},"to":{"type":"string","description":"YYYY-MM-DD"},
                                "grain":{"type":"string","enum":["week","month","quarter","year"]}}),&["from","to","grain"])},
        {"name":"categories","description":"Spending (or income) by category for the period containing a day.",
         "inputSchema":s(json!({"day":{"type":"string"},"grain":{"type":"string","enum":["week","month","quarter","year"]},"income":{"type":"boolean"}}),&["day","grain"])},
        {"name":"budget_status","description":"Budget vs spending per category for the period containing a day.",
         "inputSchema":s(json!({"day":{"type":"string"},"grain":{"type":"string","enum":["week","month","quarter","year"]}}),&["day","grain"])},
        {"name":"forecast","description":"Monthly projection from the household's own history.",
         "inputSchema":s(json!({"months":{"type":"integer","minimum":1,"maximum":60}}),&["months"])},
        {"name":"recurring","description":"Recurring payments and paychecks found in a date range, biggest yearly amount first.",
         "inputSchema":s(json!({"from":{"type":"string"},"to":{"type":"string"}}),&["from","to"])},
        {"name":"suggest_allocation","description":"Suggested split of a yearly savings amount (cents).",
         "inputSchema":s(json!({"yearlySavingsCents":{"type":"integer","minimum":0}}),&["yearlySavingsCents"])}
    ])
}

fn text<'a>(a: &'a Value, k: &str) -> Result<&'a str, String> {
    a.get(k)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("Missing text argument \"{k}\"."))
}

fn call(app: &App, name: &str, a: &Value) -> Result<Value, String> {
    match name {
        "accounts" => ser(app.accounts()?),
        "balance_sheet" => ser(app.balance_sheet()?),
        "cash_flow" => {
            ser(app.cash_flow_str(text(a, "from")?, text(a, "to")?, text(a, "grain")?)?)
        }
        "categories" => ser(app.categories_str(
            text(a, "day")?,
            text(a, "grain")?,
            a.get("income").and_then(Value::as_bool).unwrap_or(false),
        )?),
        "budget_status" => ser(app.budget_status(text(a, "day")?, text(a, "grain")?)?),
        "recurring" => ser(app.recurring(text(a, "from")?, text(a, "to")?)?),
        "forecast" => {
            let m = a
                .get("months")
                .and_then(Value::as_u64)
                .ok_or("Missing number argument \"months\".")?;
            ser(app.forecast(u32::try_from(m).map_err(|_| "Choose between 1 and 60 months.")?)?)
        }
        "suggest_allocation" => {
            let c = a
                .get("yearlySavingsCents")
                .and_then(Value::as_i64)
                .ok_or("Missing number argument \"yearlySavingsCents\".")?;
            ser(app.suggest_allocation(c)?)
        }
        _ => Err(format!("Unknown tool \"{name}\".")),
    }
}

fn ser<T: serde::Serialize>(x: T) -> Result<Value, String> {
    serde_json::to_value(x).map_err(|e| e.to_string())
}

fn reply(id: &Value, result: Value) -> String {
    json!({"jsonrpc":"2.0","id":id,"result":result}).to_string()
}

fn error(id: &Value, code: i64, message: &str) -> String {
    json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":message}}).to_string()
}

/// Handle one JSON-RPC line. `None` means nothing to send back (a notification).
pub fn handle(app: &App, line: &str) -> Option<String> {
    let req: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(_) => return Some(error(&Value::Null, -32700, "Parse error")),
    };
    let Some(method) = req.get("method").and_then(Value::as_str) else {
        return Some(error(
            req.get("id").unwrap_or(&Value::Null),
            -32600,
            "Invalid request",
        ));
    };
    let Some(id) = req.get("id") else {
        return None; // notification, e.g. notifications/initialized
    };
    let params = req.get("params").cloned().unwrap_or(Value::Null);
    Some(match method {
        "initialize" => reply(
            id,
            json!({
                "protocolVersion": PROTOCOL,
                "capabilities": {"tools": {}},
                "serverInfo": {"name": "golden", "version": env!("CARGO_PKG_VERSION")}
            }),
        ),
        "ping" => reply(id, json!({})),
        "tools/list" => reply(id, json!({"tools": tools()})),
        "tools/call" => {
            let name = params.get("name").and_then(Value::as_str).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(json!({}));
            match call(app, name, &args) {
                Ok(v) => reply(
                    id,
                    json!({"content":[{"type":"text","text":v.to_string()}],"isError":false}),
                ),
                // Tool failures are results the model can read, not protocol errors.
                Err(e) => reply(
                    id,
                    json!({"content":[{"type":"text","text":e}],"isError":true}),
                ),
            }
        }
        _ => error(id, -32601, "Method not found"),
    })
}

/// Where the desktop app keeps its data, so `golden --mcp` finds the same database the window
/// uses. Mirrors the per-OS folder Tauri picks for the app identifier.
pub fn data_dir(
    os: &str,
    identifier: &str,
    env: impl Fn(&str) -> Option<String>,
) -> Option<std::path::PathBuf> {
    use std::path::PathBuf;
    let base = match os {
        "macos" => PathBuf::from(env("HOME")?).join("Library/Application Support"),
        "windows" => PathBuf::from(env("APPDATA")?),
        _ => match env("XDG_DATA_HOME") {
            Some(x) if !x.is_empty() => PathBuf::from(x),
            _ => PathBuf::from(env("HOME")?).join(".local/share"),
        },
    };
    Some(base.join(identifier))
}

/// Serve MCP over any line-oriented reader/writer (stdin/stdout in practice).
pub fn serve(
    app: &App,
    input: impl std::io::BufRead,
    mut output: impl std::io::Write,
) -> std::io::Result<()> {
    for line in input.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if let Some(reply) = handle(app, &line) {
            writeln!(output, "{reply}")?;
            output.flush()?;
        }
    }
    Ok(())
}
