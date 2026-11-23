//! `golden://activate?key=<licence>` links, as put in the purchase email. The operating system
//! hands the link to Golden, which activates the licence without any copying and pasting.
//! Parsing is deliberately strict: only the activation action and a plain licence token get through.

/// A licence token is two base64url parts joined by a dot; nothing else is accepted.
const MAX_LEN: usize = 4096;

fn plain_token(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= MAX_LEN
        && s.matches('.').count() == 1
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}

/// The licence token inside one activation link, or None for any other link.
pub fn token_from_link(link: &str) -> Option<String> {
    let rest = link.trim().strip_prefix("golden://")?;
    let (action, query) = rest.split_once('?')?;
    if action.trim_end_matches('/') != "activate" {
        return None;
    }
    let query = query.split('#').next().unwrap_or("");
    query
        .split('&')
        .find_map(|kv| kv.strip_prefix("key="))
        .filter(|k| plain_token(k))
        .map(str::to_string)
}

/// The first activation token among the links the OS passed in.
pub fn token_from_links<S: AsRef<str>>(links: &[S]) -> Option<String> {
    links.iter().find_map(|l| token_from_link(l.as_ref()))
}
