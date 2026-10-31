//! Page titles for links (`Space i U`, like org-cliplink).

/// Bytes of a page read for its title.
const MAX: usize = 64 * 1024;

/// The decoded, whitespace-collapsed text of the first `<title>` in HTML, or None.
pub fn extract_title(html: &str) -> Option<String> {
    let low = html.to_ascii_lowercase(); // ASCII-only, so byte offsets stay valid in HTML
    let open = low.find("<title")?;
    let start = open + low[open..].find('>')? + 1;
    let end = start + low[start..].find("</title").unwrap_or(low.len() - start);
    let t = decode(&html[start..end]).split_whitespace().collect::<Vec<_>>().join(" ");
    (!t.is_empty()).then_some(t)
}

/// HTML entities in S: the common named ones and `&#N;` / `&#xH;`; anything else is kept as written.
pub fn decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        let ent = rest[1..].find(';').filter(|&j| j <= 10).map(|j| &rest[1..=j]);
        let ch = ent.and_then(|e| match e {
            "amp" => Some('&'), "lt" => Some('<'), "gt" => Some('>'), "quot" => Some('"'), "apos" => Some('\''), "nbsp" => Some(' '),
            "ndash" => Some('–'), "mdash" => Some('—'), "hellip" => Some('…'), "lsquo" => Some('‘'), "rsquo" => Some('’'), "ldquo" => Some('“'), "rdquo" => Some('”'),
            _ => e.strip_prefix('#').and_then(|n| match n.strip_prefix(['x', 'X']) {
                Some(h) => u32::from_str_radix(h, 16).ok(),
                None => n.parse().ok(),
            }).and_then(char::from_u32),
        });
        match (ent, ch) {
            (Some(e), Some(c)) => { out.push(c); rest = &rest[e.len() + 2..]; }
            _ => { out.push('&'); rest = &rest[1..]; }
        }
    }
    out + rest
}

/// Only http(s) pages are fetched.
pub fn check_url(url: &str) -> Result<(), String> {
    let l = url.to_ascii_lowercase();
    if l.starts_with("http://") || l.starts_with("https://") { Ok(()) } else { Err("only http(s) links have a title to fetch".into()) }
}

/// A missing content type is read; otherwise only HTML is.
fn is_html(ct: Option<&str>) -> bool {
    ct.is_none_or(|c| c.to_ascii_lowercase().contains("html"))
}

/// The title of the page at URL (5 s timeout, redirects followed, first 64 KB). Errors leave the URL out.
#[tauri::command]
pub async fn fetch_title(url: String) -> Result<String, String> {
    check_url(&url)?;
    let body = get(&url, MAX, 5, is_html, "not an HTML page").await?;
    extract_title(&body).ok_or_else(|| "the page has no title".into())
}

/// The first MAX bytes of URL (SECS timeout, redirects followed) as text, if OK passes its content type
/// (else the error NOT). Errors leave the URL out.
pub async fn get(url: &str, max: usize, secs: u64, ok: fn(Option<&str>) -> bool, not: &str) -> Result<String, String> {
    if rustls::crypto::CryptoProvider::get_default().is_none() {
        let _ = rustls::crypto::ring::default_provider().install_default();
    }
    let e = |e: reqwest::Error| e.without_url().to_string();
    let mut resp = reqwest::Client::builder().user_agent("Margin").timeout(std::time::Duration::from_secs(secs)).build().map_err(e)?.get(url).send().await.map_err(e)?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status().as_u16()));
    }
    if !ok(resp.headers().get(reqwest::header::CONTENT_TYPE).and_then(|v| v.to_str().ok())) {
        return Err(not.into());
    }
    let mut body = Vec::new();
    while body.len() < max {
        match resp.chunk().await.map_err(e)? {
            Some(c) => body.extend_from_slice(&c),
            None => break,
        }
    }
    body.truncate(max);
    Ok(String::from_utf8_lossy(&body).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titles() {
        assert_eq!(extract_title("<html><head><TITLE>Hello</TITLE></head>").as_deref(), Some("Hello"));
        assert_eq!(extract_title("<title data-x=\"1\">\n  Rust &amp; Tauri &#8212; a\n\tguide &#x2F; &quot;x&quot; &bogus; &</title>").as_deref(), Some("Rust & Tauri — a guide / \"x\" &bogus; &"));
        assert_eq!(extract_title("<title>Caf\u{e9} &rsquo;s</title>").as_deref(), Some("Café ’s"), "non-ASCII text keeps its offsets");
        assert_eq!(extract_title("<title>cut off at the 64 KB limit"), Some("cut off at the 64 KB limit".into()));
        assert_eq!(extract_title("<title>  </title>"), None);
        assert_eq!(extract_title("<html><body>no title</body>"), None);
        assert_eq!(extract_title("<title"), None);
    }

    #[test]
    fn checks() {
        assert!(check_url("https://a.b/x").is_ok() && check_url("HTTP://a.b").is_ok());
        assert!(check_url("file:///etc/passwd").is_err() && check_url("mailto:a@b.c").is_err() && check_url("ftp://a.b").is_err());
        assert!(is_html(None) && is_html(Some("text/html; charset=utf-8")) && is_html(Some("application/xhtml+xml")));
        assert!(!is_html(Some("application/pdf")) && !is_html(Some("image/png")));
    }
}
