use fin_app::mcp::handle;
use fin_app::App;
use serde_json::{json, Value};

fn app() -> App {
    let mut app = App::in_memory().unwrap();
    app.import_statement(
        "Date,Description,Amount\n11/01/2026,ACME PAYROLL,5000.00\n11/03/2026,RENT PAYMENT LANDLORD,-1500.00\n"
            .as_bytes(),
        "Checking",
        "checking",
    )
    .unwrap();
    app
}

fn ask(app: &App, v: Value) -> Value {
    serde_json::from_str(&handle(app, &v.to_string()).expect("a reply")).unwrap()
}

fn call(app: &App, name: &str, args: Value) -> Value {
    ask(
        app,
        json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":args}}),
    )
}

fn payload(r: &Value) -> Value {
    serde_json::from_str(r["result"]["content"][0]["text"].as_str().unwrap()).unwrap()
}

#[test]
fn initialize_announces_tools() {
    let r = ask(
        &app(),
        json!({"jsonrpc":"2.0","id":7,"method":"initialize","params":{}}),
    );
    assert_eq!(r["id"], 7);
    assert_eq!(r["result"]["serverInfo"]["name"], "golden");
    assert!(r["result"]["capabilities"]["tools"].is_object());
}

#[test]
fn notifications_get_no_reply() {
    let a = app();
    assert!(handle(
        &a,
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#
    )
    .is_none());
}

#[test]
fn the_tool_list_is_read_only() {
    let r = ask(
        &app(),
        json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}),
    );
    let names: Vec<&str> = r["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"balance_sheet") && names.contains(&"cash_flow"));
    for forbidden in [
        "import", "delete", "set_", "save", "restore", "activate", "apply",
    ] {
        assert!(
            names.iter().all(|n| !n.contains(forbidden)),
            "{forbidden} must not be exposed"
        );
    }
    assert!(r["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .all(|t| t["inputSchema"]["type"] == "object"));
}

#[test]
fn tools_return_real_numbers() {
    let a = app();
    let accts = payload(&call(&a, "accounts", json!({})));
    assert_eq!(accts[0]["balanceCents"], 350_000);
    let sheet = payload(&call(&a, "balance_sheet", json!({})));
    assert_eq!(sheet["netWorthCents"], 350_000);
    let flow = payload(&call(
        &a,
        "cash_flow",
        json!({"from":"2026-11-01","to":"2026-11-30","grain":"month"}),
    ));
    assert_eq!(flow[0]["incomeCents"], 500_000);
    assert_eq!(flow[0]["spendingCents"], 150_000);
    let alloc = payload(&call(
        &a,
        "suggest_allocation",
        json!({"yearlySavingsCents": 100_000}),
    ));
    assert_eq!(alloc["yearlySavingsCents"], 100_000);
}

#[test]
fn tool_failures_are_readable_results_not_protocol_errors() {
    let a = app();
    let r = call(
        &a,
        "cash_flow",
        json!({"from":"nope","to":"2026-11-30","grain":"month"}),
    );
    assert_eq!(r["result"]["isError"], true);
    assert!(r.get("error").is_none());
    let r = call(&a, "cash_flow", json!({"grain":"month"}));
    assert!(r["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("Missing"));
    let r = call(&a, "forecast", json!({"months": 99}));
    assert_eq!(r["result"]["isError"], true);
    let r = call(&a, "wipe_everything", json!({}));
    assert!(r["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("Unknown tool"));
}

#[test]
fn bad_requests_get_protocol_errors() {
    let a = app();
    let r: Value = serde_json::from_str(&handle(&a, "{nope").unwrap()).unwrap();
    assert_eq!(r["error"]["code"], -32700);
    let r = ask(
        &a,
        json!({"jsonrpc":"2.0","id":2,"method":"does/not/exist"}),
    );
    assert_eq!(r["error"]["code"], -32601);
    let r = ask(&a, json!({"jsonrpc":"2.0","id":3}));
    assert_eq!(r["error"]["code"], -32600);
}

#[test]
fn calling_tools_never_changes_the_data() {
    let a = app();
    let before = a.accounts().unwrap();
    for t in ["accounts", "balance_sheet"] {
        call(&a, t, json!({}));
    }
    assert_eq!(a.accounts().unwrap(), before);
}

#[test]
fn serve_answers_each_line_and_skips_notifications() {
    let a = app();
    let input = "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}\n\n{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/list\"}\n";
    let mut out = Vec::new();
    fin_app::mcp::serve(&a, input.as_bytes(), &mut out).unwrap();
    let lines: Vec<&str> = std::str::from_utf8(&out).unwrap().lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("\"id\":1"));
    assert!(lines[1].contains("balance_sheet"));
}

#[test]
fn data_dir_follows_each_os() {
    use fin_app::mcp::data_dir;
    use std::path::PathBuf;
    let env = |k: &str| match k {
        "HOME" => Some("/Users/h".to_string()),
        "APPDATA" => Some("C:\\Users\\h\\AppData\\Roaming".to_string()),
        _ => None,
    };
    assert_eq!(
        data_dir("macos", "com.highturtle.golden", env),
        Some(PathBuf::from(
            "/Users/h/Library/Application Support/com.highturtle.golden"
        ))
    );
    assert_eq!(
        data_dir("windows", "com.highturtle.golden", env),
        Some(PathBuf::from("C:\\Users\\h\\AppData\\Roaming").join("com.highturtle.golden"))
    );
    assert_eq!(
        data_dir("linux", "x", env),
        Some(PathBuf::from("/Users/h/.local/share/x"))
    );
    assert_eq!(data_dir("macos", "x", |_| None), None);
}

#[test]
fn recurring_is_offered_and_works() {
    let a = app();
    let r = call(
        &a,
        "recurring",
        json!({"from":"2026-01-01","to":"2026-12-31"}),
    );
    assert_eq!(r["result"]["isError"], false);
    assert!(payload(&r).is_array());
}

#[test]
fn debt_plan_is_offered_and_works() {
    let a = app();
    let r = call(&a, "debt_plan", json!({"extraCents": 0}));
    assert_eq!(r["result"]["isError"], false);
    assert!(payload(&r)["debts"].is_array());
}

#[test]
fn insights_are_offered_and_work() {
    let a = app();
    let r = call(&a, "insights", json!({}));
    assert_eq!(r["result"]["isError"], false);
    assert!(payload(&r).is_array());
}

#[test]
fn net_worth_history_is_offered_and_checks_its_input() {
    let a = app();
    let r = call(&a, "net_worth_history", json!({"months": 3}));
    assert_eq!(payload(&r).as_array().unwrap().len(), 3);
    assert_eq!(
        call(&a, "net_worth_history", json!({"months": 0}))["result"]["isError"],
        true
    );
}
