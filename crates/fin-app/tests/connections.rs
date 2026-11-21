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
