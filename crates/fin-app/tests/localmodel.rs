use fin_app::localmodel::{parse_endpoint, response_body, Style};
use fin_app::App;
use fin_core::NaiveDate;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 16).unwrap()
}

/// A one-shot fake model server: reads one request, sends `reply`, returns the request text.
fn serve(reply: Vec<u8>) -> (u16, thread::JoinHandle<String>) {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    let h = thread::spawn(move || {
        let (mut s, _) = l.accept().unwrap();
        let mut buf = vec![];
        let mut chunk = [0u8; 4096];
        loop {
            let n = s.read(&mut chunk).unwrap();
            buf.extend_from_slice(&chunk[..n]);
            let text = String::from_utf8_lossy(&buf).to_string();
            if let Some(i) = text.find("\r\n\r\n") {
                let len = text[..i]
                    .lines()
                    .find_map(|l| {
                        l.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .map(|v| v.trim().parse::<usize>().unwrap())
                    })
                    .unwrap_or(0);
                if buf.len() >= i + 4 + len {
                    break;
                }
            }
        }
        s.write_all(&reply).unwrap();
        String::from_utf8_lossy(&buf).to_string()
    });
    (port, h)
}

fn ok(body: &str) -> Vec<u8> {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    )
    .into_bytes()
}

const CHANGE: &str = r#"{"changes":[{"kind":"retireAt","person":0,"age":60}]}"#;

fn ollama_reply() -> Vec<u8> {
    ok(&serde_json::json!({ "response": CHANGE }).to_string())
}

#[test]
fn only_loopback_http_addresses_with_a_known_route_are_accepted() {
    let ep = parse_endpoint("http://127.0.0.1:11434/api/generate").unwrap();
    assert_eq!(
        (ep.host.as_str(), ep.port, ep.style),
        ("127.0.0.1", 11434, Style::Ollama)
    );
    assert_eq!(
        parse_endpoint("http://localhost:8080/v1/chat/completions")
            .unwrap()
            .style,
        Style::OpenAi
    );
    assert_eq!(
        parse_endpoint("http://[::1]:9/v1/chat/completions")
            .unwrap()
            .host,
        "::1"
    );
    for bad in [
        "https://127.0.0.1:11434/api/generate",
        "http://example.com/api/generate",
        "http://192.168.1.5:11434/api/generate",
        "http://127.0.0.1.evil.com/api/generate",
        "http://user@127.0.0.1/api/generate",
        "http://127.0.0.1:abc/api/generate",
        "http://127.0.0.1:11434/other",
        "127.0.0.1:11434/api/generate",
        "",
    ] {
        assert!(parse_endpoint(bad).is_err(), "{bad} should be refused");
    }
}

#[test]
fn response_bodies_are_read_plain_or_chunked_and_errors_are_named() {
    assert_eq!(response_body(&ok("{}")).unwrap(), b"{}");
    let chunked =
        b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n3\r\nabc\r\n2\r\nde\r\n0\r\n\r\n";
    assert_eq!(response_body(chunked).unwrap(), b"abcde");
    let err = response_body(b"HTTP/1.1 404 Not Found\r\n\r\nnope").unwrap_err();
    assert!(err.contains("404"), "{err}");
    assert!(response_body(b"garbage").is_err());
    assert!(response_body(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\nzz\r\n").is_err());
}

#[test]
fn a_running_ollama_server_answers_the_whatif_chat() {
    let (port, req) = serve(ollama_reply());
    let mut a = App::in_memory().unwrap();
    a.set_model(&format!("http://127.0.0.1:{port}/api/generate"), "llama3.2")
        .unwrap();
    // The rule phrases can't read this; only the model's answer can produce a change.
    let prep = a
        .prepare_whatif("make me stop working earlier please", today())
        .unwrap();
    assert_eq!(prep.changes.len(), 1);
    let sent = req.join().unwrap();
    assert!(sent.starts_with("POST /api/generate HTTP/1.1"));
    assert!(sent.contains("\"model\":\"llama3.2\"") && sent.contains("\"stream\":false"));
    assert!(sent.contains("make me stop working earlier please"));
}

#[test]
fn an_openai_style_server_works_too() {
    let reply =
        ok(&serde_json::json!({ "choices": [{ "message": { "content": CHANGE } }] }).to_string());
    let (port, req) = serve(reply);
    let mut a = App::in_memory().unwrap();
    a.set_model(
        &format!("http://localhost:{port}/v1/chat/completions"),
        "local",
    )
    .unwrap();
    assert_eq!(
        a.prepare_whatif("make me stop working earlier please", today())
            .unwrap()
            .changes
            .len(),
        1
    );
    assert!(req.join().unwrap().contains("\"messages\""));
}

#[test]
fn the_prompt_sent_to_the_model_holds_no_balances() {
    let (port, req) = serve(ollama_reply());
    let mut a = App::in_memory().unwrap();
    a.set_model(&format!("http://127.0.0.1:{port}/api/generate"), "m")
        .unwrap();
    let mut h = a.household().unwrap();
    h.taxable_cents = 98_765_432;
    h.people[0].birth_date = "1975-03-09".into();
    a.save_household(&h).unwrap();
    a.prepare_whatif("make me stop working earlier please", today())
        .unwrap();
    let sent = req.join().unwrap();
    assert!(!sent.contains("98765432") && !sent.contains("987654"));
    assert!(!sent.contains("1975-03-09"));
}

#[test]
fn a_server_that_is_off_or_misbehaving_falls_back_to_the_phrase_rules() {
    // Nothing listening: bind then drop to find a free port.
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let mut a = App::in_memory().unwrap();
    a.set_model(&format!("http://127.0.0.1:{port}/api/generate"), "m")
        .unwrap();
    assert_eq!(
        a.prepare_whatif("what if I retire at 65", today())
            .unwrap()
            .changes
            .len(),
        1
    );
    // A server that answers nonsense also falls back.
    let (port, _h) = serve(ok(r#"{"response":"I think you should relax"}"#));
    a.set_model(&format!("http://127.0.0.1:{port}/api/generate"), "m")
        .unwrap();
    assert_eq!(
        a.prepare_whatif("what if I retire at 65", today())
            .unwrap()
            .changes
            .len(),
        1
    );
}

#[test]
fn settings_are_validated_saved_cleared_and_reloaded() {
    let mut a = App::in_memory().unwrap();
    assert_eq!(a.model_settings().unwrap().endpoint, "");
    assert!(a.set_model("http://example.com/api/generate", "m").is_err());
    assert!(a
        .set_model("http://127.0.0.1:11434/api/generate", "  ")
        .is_err());
    let s = a
        .set_model("http://127.0.0.1:11434/api/generate", " llama3.2 ")
        .unwrap();
    assert_eq!(
        (s.endpoint.as_str(), s.name.as_str()),
        ("http://127.0.0.1:11434/api/generate", "llama3.2")
    );
    a.refresh_interpreter();
    let cleared = a.set_model("", "").unwrap();
    assert_eq!((cleared.endpoint.as_str(), cleared.name.as_str()), ("", ""));
    // With no model the built-in phrases still work.
    assert_eq!(
        a.prepare_whatif("what if I retire at 65", today())
            .unwrap()
            .changes
            .len(),
        1
    );
}

#[test]
fn changing_the_model_is_blocked_after_the_trial_like_other_edits() {
    let a = App::in_memory().unwrap();
    a.db().set_setting("first_run", "2026-01-01").unwrap();
    let mut a = a.with_clock(today);
    assert!(a
        .set_model("http://127.0.0.1:11434/api/generate", "m")
        .is_err());
}
