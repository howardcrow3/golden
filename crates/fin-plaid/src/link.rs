//! Plaid's sign-in window is a web page, so Golden opens one in the browser. The sync service
//! serves that page (`GET /link?token=…`); when the person finishes, the page sends them back with
//! `golden://plaid?public_token=…&institution=…&state=…`, which the operating system hands to the app.
//!
//! Any web page can open a `golden://plaid` link, so the app makes a random `state` for each
//! sign-in it starts, hands it to the page, and accepts a return only if it carries that same
//! `state`. A page that wasn't started from this app can't link its own bank into someone's copy.

/// Keep letters, digits and `-._~`; escape everything else.
pub fn encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

pub fn decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < b.len() && hex(b[i + 1]).is_some() && hex(b[i + 2]).is_some() => {
                out.push(hex(b[i + 1]).unwrap() * 16 + hex(b[i + 2]).unwrap());
                i += 3;
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex(c: u8) -> Option<u8> {
    (c as char).to_digit(16).map(|d| d as u8)
}

/// A fresh random value that ties a return to the sign-in this app started.
pub fn new_state() -> String {
    use std::hash::{BuildHasher, Hasher};
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    // Each RandomState is keyed from the operating system's randomness.
    let mut out = String::new();
    for i in 0..2u64 {
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u128(nanos);
        h.write_u64(i);
        out.push_str(&format!("{:016x}", h.finish()));
    }
    out
}

/// The address to open in the browser for a link token and the `state` the page must send back.
pub fn page_url(service_base: &str, link_token: &str, state: &str) -> String {
    format!(
        "{}/link?token={}&state={}",
        service_base.trim_end_matches('/'),
        encode(link_token),
        encode(state)
    )
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkReturn {
    pub public_token: String,
    pub institution: String,
}

/// Find a finished sign-in among the links the operating system handed over. Only a link that
/// carries `expected_state` counts; an empty `expected_state` (no sign-in under way) accepts none.
pub fn parse_return(urls: &[String], expected_state: &str) -> Option<LinkReturn> {
    if expected_state.is_empty() {
        return None;
    }
    urls.iter().find_map(|u| {
        let rest = u.strip_prefix("golden://plaid")?;
        let query = rest.strip_prefix('?')?;
        let mut token = None;
        let mut inst = String::new();
        let mut state = String::new();
        for pair in query.split('&') {
            let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
            match k {
                "public_token" => token = Some(decode(v)),
                "institution" => inst = decode(v),
                "state" => state = decode(v),
                _ => {}
            }
        }
        if state != expected_state {
            return None;
        }
        let public_token = token.filter(|t| !t.is_empty())?;
        Some(LinkReturn {
            public_token,
            institution: if inst.trim().is_empty() {
                "Bank".into()
            } else {
                inst
            },
        })
    })
}
