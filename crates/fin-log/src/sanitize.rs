/// Words that mark the value after them as a secret (`token=abc`, `password: x`).
const SECRET_KEYS: &[&str] = &[
    "password",
    "passphrase",
    "secret",
    "token",
    "apikey",
    "api_key",
    "key",
    "licence",
    "license",
    "authorization",
];

fn is_date(t: &str) -> bool {
    let b = t.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter()
            .enumerate()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
}

fn clean_token(t: &str) -> String {
    let edge = |c: char| {
        matches!(
            c,
            '"' | '\'' | '(' | ')' | '[' | ']' | '{' | '}' | ',' | ';'
        )
    };
    let core = t.trim_matches(edge);
    if core.is_empty() {
        return t.to_string();
    }
    let lead = &t[..t.find(core).unwrap_or(0)];
    let trail = &t[lead.len() + core.len()..];
    let replaced = clean_core(core);
    format!("{lead}{replaced}{trail}")
}

fn clean_core(core: &str) -> String {
    // key=value or key:value with a secret-looking key.
    if let Some(i) = core.find(['=', ':']) {
        let key = core[..i].to_ascii_lowercase();
        if i > 0
            && i + 1 < core.len()
            && SECRET_KEYS.iter().any(|k| key.contains(k))
            && !core[i + 1..].starts_with("//")
        {
            return format!("{}{}[redacted]", &core[..i], &core[i..=i]);
        }
    }
    if core.contains('@') && core.contains('.') {
        return "[email]".into();
    }
    let money_prefix = core.starts_with('$') || core.starts_with("-$") || core.starts_with("($");
    if money_prefix && core.chars().any(|c| c.is_ascii_digit()) {
        return "[amt]".into();
    }
    if is_date(core) {
        return core.into();
    }
    let digits = core.chars().filter(|c| c.is_ascii_digit()).count();
    if digits >= 6 {
        return "[num]".into();
    }
    let secretish = core.len() >= 24
        && core
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "+/_=.-".contains(c))
        && core.chars().any(|c| c.is_ascii_digit())
        && core.chars().any(|c| c.is_ascii_alphabetic());
    if secretish {
        return "[secret]".into();
    }
    core.into()
}

/// Replace anything that could be a number from someone's finances or a credential:
/// dollar amounts, runs of six or more digits (account, routing and tax numbers, ids),
/// e-mail addresses, long key-like strings, and the value after `password=`, `token:` and the like.
/// Dates (2026-11-24) are kept.
pub fn sanitize(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut token = String::new();
    // Set after a bare "password:" so the next word is hidden too.
    let mut hide_next = false;
    let mut flush = |token: &mut String, out: &mut String| {
        if token.is_empty() {
            return;
        }
        if hide_next {
            out.push_str("[redacted]");
            hide_next = false;
        } else {
            out.push_str(&clean_token(token));
            let lower = token.to_ascii_lowercase();
            hide_next = (lower.ends_with(':') || lower.ends_with('='))
                && SECRET_KEYS.iter().any(|k| lower.contains(k));
        }
        token.clear();
    };
    for c in s.chars() {
        if c.is_whitespace() {
            flush(&mut token, &mut out);
            out.push(c);
        } else {
            token.push(c);
        }
    }
    flush(&mut token, &mut out);
    out
}
