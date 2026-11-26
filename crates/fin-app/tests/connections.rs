use fin_app::App;

#[test]
fn private_by_default_and_lists_every_outside_service() {
    let app = App::in_memory().unwrap();
    let c = app.connections().unwrap();
    assert!(c.private);
    assert!(c.items.iter().all(|i| !i.in_use));
    let names: Vec<_> = c.items.iter().map(|i| i.name.as_str()).collect();
    assert!(names.iter().any(|n| n.contains("download")));
    assert!(names.iter().any(|n| n.contains("Plaid")));
    assert!(names.iter().any(|n| n.contains("feedback")));
    assert!(c
        .items
        .iter()
        .all(|i| !i.sends.is_empty() && !i.when.is_empty()));
}

#[test]
fn a_local_model_address_is_in_use_but_stays_private() {
    let mut app = App::in_memory().unwrap();
    app.set_model("http://127.0.0.1:11434/api/generate", "llama3")
        .unwrap();
    let c = app.connections().unwrap();
    let local = c.items.iter().find(|i| i.host == "this computer").unwrap();
    assert!(local.in_use);
    assert!(c.private, "a model on this computer sends nothing out");
}

#[test]
fn a_linked_bank_makes_the_plaid_connection_in_use_and_not_private() {
    let app = App::in_memory().unwrap();
    app.db()
        .set_setting(
            "bank.items",
            r#"[{"itemId":"i","institution":"Chase","cursor":"","status":"active"}]"#,
        )
        .unwrap();
    let c = app.connections().unwrap();
    let plaid = c.items.iter().find(|i| i.name.contains("Plaid")).unwrap();
    assert!(plaid.in_use);
    assert!(!c.private);
}

#[test]
fn a_paused_bank_is_not_in_use() {
    let app = App::in_memory().unwrap();
    app.db()
        .set_setting(
            "bank.items",
            r#"[{"itemId":"i","institution":"Chase","cursor":"","status":"paused"}]"#,
        )
        .unwrap();
    assert!(app.connections().unwrap().private);
}
