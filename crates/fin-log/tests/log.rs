use fin_log::{sanitize, RotatingFile};

#[test]
fn amounts_numbers_emails_and_keys_are_scrubbed() {
    assert_eq!(sanitize("paid $1,234.56 today"), "paid [amt] today");
    assert_eq!(sanitize("balance ($50.00)"), "balance ([amt])");
    assert_eq!(sanitize("account 123456789 failed"), "account [num] failed");
    assert_eq!(sanitize("ssn 123-45-6789"), "ssn [num]");
    assert_eq!(
        sanitize("mail me at a.b@example.com."),
        "mail me at [email]"
    );
    assert_eq!(sanitize("token=abc123 ok"), "token=[redacted] ok");
    assert_eq!(sanitize("password: hunter2 ok"), "password: [redacted] ok");
    assert_eq!(
        sanitize("sent aBcDeFgHiJkLmNoPqRsTuVwX1 now"),
        "sent [secret] now"
    );
}

#[test]
fn ordinary_text_dates_and_small_numbers_survive() {
    let m = "imported 42 rows from checking.csv on 2026-11-24 (3 duplicates)";
    assert_eq!(sanitize(m), m);
    assert_eq!(sanitize(""), "");
    assert_eq!(sanitize("  spaced   out  "), "  spaced   out  ");
}

#[test]
fn urls_keep_their_scheme() {
    assert_eq!(
        sanitize("GET https://example.com/link ok"),
        "GET https://example.com/link ok"
    );
}

#[test]
fn rotation_keeps_the_newest_files_and_drops_the_oldest() {
    let dir = tempfile::tempdir().unwrap();
    let mut f = RotatingFile::open(dir.path(), "t", 100, 2).unwrap();
    for i in 0..30 {
        f.append(&format!("line {i:02} ........................"))
            .unwrap();
    }
    let names: Vec<String> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(names.contains(&"t.log".to_string()));
    assert!(names.contains(&"t.1.log".to_string()));
    assert!(names.contains(&"t.2.log".to_string()));
    assert!(!names.contains(&"t.3.log".to_string()));
    for n in &names {
        assert!(
            std::fs::metadata(dir.path().join(n)).unwrap().len() <= 200,
            "{n}"
        );
    }
}

#[test]
fn tail_returns_the_last_lines_oldest_first_across_files() {
    let dir = tempfile::tempdir().unwrap();
    let mut f = RotatingFile::open(dir.path(), "t", 100, 3).unwrap();
    for i in 0..12 {
        f.append(&format!("line {i:02} ........................"))
            .unwrap();
    }
    let t = f.tail(5);
    let lines: Vec<&str> = t.lines().collect();
    assert_eq!(lines.len(), 5);
    assert!(lines[0].starts_with("line 07"));
    assert!(lines[4].starts_with("line 11"));
}

#[test]
fn reopening_appends_and_counts_existing_size() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut f = RotatingFile::open(dir.path(), "t", 1000, 2).unwrap();
        f.append("first").unwrap();
    }
    let mut f = RotatingFile::open(dir.path(), "t", 1000, 2).unwrap();
    f.append("second").unwrap();
    assert_eq!(f.tail(10), "first\nsecond");
}
