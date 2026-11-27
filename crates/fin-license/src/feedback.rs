//! "Send feedback": the body and signature for the High Turtle feedback service
//! (`https://feedback.turtlebox.studio/v1/feedback`, see howardcrow3/highturtle `feedback/README.md`).
//!
//! Each request carries `x-ht-app`, `x-ht-timestamp` (unix seconds) and `x-ht-signature`: hex
//! HMAC-SHA256 with this app's key over `"<timestamp>.<raw body>"`. The key is compiled in from the
//! `FEEDBACK_KEY` build variable (a GitHub Actions secret) and is never committed. Sending lives in
//! the desktop shell, which owns the HTTP client.
use hmac::{Hmac, Mac};
use sha2::Sha256;

pub const FEEDBACK_URL: &str = "https://feedback.turtlebox.studio/v1/feedback";
pub const APP_ID: &str = "golden";

fn unhex(s: &str) -> Option<Vec<u8>> {
    let s = s.trim();
    if !s.len().is_multiple_of(2) {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok())
        .collect()
}

/// The signature header value for one request. `key_hex` is the app key as stored by the service.
pub fn sign(key_hex: &str, timestamp: i64, body: &str) -> Option<String> {
    let mut mac = Hmac::<Sha256>::new_from_slice(&unhex(key_hex)?).ok()?;
    mac.update(format!("{timestamp}.{body}").as_bytes());
    Some(
        mac.finalize()
            .into_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect(),
    )
}

/// A sensible-looking email address, matching the service's own check: one `@`, no spaces, a dotted
/// domain of letters, digits and hyphens, and a top-level domain of at least 2 letters.
pub fn email_ok(e: &str) -> bool {
    let Some((local, domain)) = e.split_once('@') else {
        return false;
    };
    let bad_local = |c: char| c.is_whitespace() || c.is_control() || "@\"<>()[],;:\\".contains(c);
    let label_ok = |l: &&str| {
        !l.is_empty()
            && l.len() <= 63
            && !l.starts_with('-')
            && !l.ends_with('-')
            && l.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
    };
    let labels: Vec<&str> = domain.split('.').collect();
    let tld = labels.last().copied().unwrap_or("");
    e.len() <= 254
        && !local.is_empty()
        && local.len() <= 64
        && !local.chars().any(bad_local)
        && !local.starts_with('.')
        && !local.ends_with('.')
        && !local.contains("..")
        && labels.len() >= 2
        && labels.iter().all(label_ok)
        && (2..=63).contains(&tld.len())
        && tld.chars().all(|c| c.is_ascii_alphabetic())
}

/// What the person typed, checked the same way the service checks it. Returns the JSON body to send.
pub fn body(
    rating: Option<u8>,
    message: &str,
    category: Option<&str>,
    email: Option<&str>,
    anonymous: bool,
    app_version: &str,
    platform: &str,
) -> Result<String, String> {
    let message = message.trim();
    if rating.is_some_and(|r| !(1..=5).contains(&r)) {
        return Err("Choose from 1 to 5 stars.".into());
    }
    if rating.is_none() && message.is_empty() {
        return Err("Give a star rating or write a few words.".into());
    }
    if message.chars().count() > 5000 {
        return Err("Please keep it under 5,000 characters.".into());
    }
    let mut v =
        serde_json::json!({ "app": APP_ID, "appVersion": app_version, "platform": platform });
    if let Some(r) = rating {
        v["rating"] = r.into();
    }
    if !message.is_empty() {
        v["message"] = message.into();
    }
    if let Some(c) = category.filter(|c| ["feature", "issue", "praise", "other"].contains(c)) {
        v["category"] = c.into();
    }
    if anonymous {
        v["anonymous"] = true.into();
    } else if let Some(e) = email.map(str::trim).filter(|e| !e.is_empty()) {
        if !email_ok(e) {
            return Err(
                "That email address doesn’t look right. Fix it, clear it, or tick Send anonymously."
                    .into(),
            );
        }
        v["email"] = e.into();
    }
    Ok(v.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_matches_the_service() {
        // Value computed by the service's own signer (feedback/src/core.mjs signBody) for these inputs.
        let sig = sign(
            &"ab".repeat(32),
            1_795_694_400,
            r#"{"app":"backer","rating":5}"#,
        )
        .unwrap();
        assert_eq!(
            sig,
            "d2523b2c438bb71b1a763eaf45b3eeea2041afaf961bdf15c202c045455d54df"
        );
        assert!(sign("not hex", 1, "{}").is_none());
        assert!(sign("abc", 1, "{}").is_none());
    }

    #[test]
    fn body_rules() {
        assert!(body(None, "  ", None, None, false, "1.0", "macos").is_err());
        assert!(body(Some(0), "", None, None, false, "1.0", "macos").is_err());
        assert!(body(Some(6), "", None, None, false, "1.0", "macos").is_err());
        assert!(body(None, &"x".repeat(5001), None, None, false, "1.0", "macos").is_err());
        let v: serde_json::Value = serde_json::from_str(
            &body(
                Some(4),
                " Track my HSA ",
                Some("feature"),
                Some(" "),
                false,
                "1.0",
                "macos",
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            v,
            serde_json::json!({"app":"golden","rating":4,"message":"Track my HSA","category":"feature","appVersion":"1.0","platform":"macos"})
        );
        let v: serde_json::Value = serde_json::from_str(
            &body(
                None,
                "hi",
                Some("bogus"),
                Some("a@b.co"),
                false,
                "1.0",
                "linux",
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(v["email"], "a@b.co");
        assert!(v.get("category").is_none());
    }

    #[test]
    fn email_and_anonymous() {
        for good in [
            "a@b.co",
            "first.last+tag@mail.example.com",
            "x_y-z@sub-domain.example.io",
        ] {
            assert!(email_ok(good), "{good}");
        }
        for bad in [
            "nope",
            "a@b",
            "a@b.c",
            "a b@c.co",
            "a@b..co",
            ".a@b.co",
            "a..b@c.co",
            "a@-b.co",
            "<a@b.co>",
            "a@b@c.co",
            "a@b.c0",
        ] {
            assert!(!email_ok(bad), "{bad}");
        }
        assert!(body(None, "hi", None, Some("nope"), false, "1.0", "macos").is_err());
        // Anonymous: the address isn't checked or sent, and the service is told so.
        let v: serde_json::Value = serde_json::from_str(
            &body(
                None,
                "hi",
                None,
                Some("me@example.com"),
                true,
                "1.0",
                "macos",
            )
            .unwrap(),
        )
        .unwrap();
        assert!(v.get("email").is_none());
        assert_eq!(v["anonymous"], true);
        assert!(body(None, "hi", None, Some("nope"), true, "1.0", "macos").is_ok());
        // Not anonymous: no flag at all, the same body older builds sent.
        let v: serde_json::Value = serde_json::from_str(
            &body(
                None,
                "hi",
                None,
                Some("me@example.com"),
                false,
                "1.0",
                "macos",
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(v["email"], "me@example.com");
        assert!(v.get("anonymous").is_none());
    }
}
