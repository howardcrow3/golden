//! Plaid's sign-in window is a web page, so Golden opens one in the browser. The sync service
//! serves that page (`GET /link?token=…`); when the person finishes, the page sends them back with
//! `golden://plaid?public_token=…&institution=…`, which the operating system hands to the app.

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

/// The address to open in the browser for a link token.
pub fn page_url(service_base: &str, link_token: &str) -> String {
    format!(
        "{}/link?token={}",
        service_base.trim_end_matches('/'),
        encode(link_token)
    )
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkReturn {
    pub public_token: String,
    pub institution: String,
}

/// Find a finished sign-in among the links the operating system handed over.
pub fn parse_return(urls: &[String]) -> Option<LinkReturn> {
    urls.iter().find_map(|u| {
        let rest = u.strip_prefix("golden://plaid")?;
        let query = rest.strip_prefix('?')?;
        let mut token = None;
        let mut inst = String::new();
        for pair in query.split('&') {
            let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
            match k {
                "public_token" => token = Some(decode(v)),
                "institution" => inst = decode(v),
                _ => {}
            }
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
