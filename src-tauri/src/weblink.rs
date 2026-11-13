//! Page titles for links (`Space i U`, like org-cliplink), and web link checks (`Space n W`, #1325).

use crate::org::OrgFile;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};

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
    tls();
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

/// Installs the rustls crypto provider once, before the first request.
fn tls() {
    if rustls::crypto::CryptoProvider::get_default().is_none() {
        let _ = rustls::crypto::ring::default_provider().install_default();
    }
}

/// One web link in notes: the first note and 0-based line it's on, and how many times it appears.
#[derive(serde::Serialize, Debug, PartialEq)]
pub struct WebLink {
    pub url: String,
    pub path: PathBuf,
    pub line: usize,
    pub count: usize,
}

static WEB: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"\[\[((?i:https?)://[^\]\s]+)\](?:\[[^\]]*\])?\]").unwrap());

/// `[[http(s)://…]]` and `[[http(s)://…][desc]]` links in FILES outside verbatim blocks (sources SKIP leaves
/// out aren't scanned), one per URL, in note and line order.
pub fn web_links(files: &[Arc<OrgFile>], skip: impl Fn(&Path) -> bool) -> Vec<WebLink> {
    let mut out: Vec<WebLink> = vec![];
    let mut at = std::collections::HashMap::<String, usize>::new();
    let mut files: Vec<_> = files.iter().filter(|f| !skip(&f.path)).collect();
    files.sort_by(|a, b| a.path.cmp(&b.path));
    for f in files {
        let blocks = crate::notes::verbatim_lines(&f.text);
        for (i, l) in f.text.lines().enumerate().filter(|(i, _)| !blocks.iter().any(|r| r.contains(i))) {
            for c in WEB.captures_iter(l) {
                match at.get(&c[1]) {
                    Some(&j) => out[j].count += 1,
                    None => {
                        at.insert(c[1].to_string(), out.len());
                        out.push(WebLink { url: c[1].to_string(), path: f.path.clone(), line: i, count: 1 });
                    }
                }
            }
        }
    }
    out
}

/// What a checked link turned out to be.
#[derive(serde::Serialize, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Status {
    Ok,
    Dead { reason: String },
    Moved { to: String },
}

/// Redirects followed before a link counts as dead.
const HOPS: usize = 5;

/// The status of a link from its HTTP HOPS (status, absolute redirect target) and the request error ERR that
/// ended them, if any: a final 2xx is ok, or moved to where its leading permanent redirects (301/308) end.
pub fn verdict(hops: &[(u16, Option<String>)], err: Option<String>) -> Status {
    let dead = |r: String| Status::Dead { reason: r };
    let Some((last, next)) = hops.last().filter(|_| err.is_none()) else { return dead(err.unwrap_or_else(|| "no response".into())) };
    if (300..400).contains(last) {
        return dead(if next.is_some() { format!("more than {HOPS} redirects") } else { format!("HTTP {last} without a new location") });
    }
    if !(200..300).contains(last) {
        return dead(format!("HTTP {last}"));
    }
    match hops.iter().take_while(|(c, _)| matches!(c, 301 | 308)).last() {
        Some((_, Some(to))) => Status::Moved { to: to.clone() },
        _ => Status::Ok,
    }
}

/// Checks the http(s) URL (8 s per request): HEAD, or GET when HEAD fails with an HTTP error (hosts that
/// refuse HEAD), following up to HOPS redirects itself so permanent ones show as moved. Errors leave the URL out.
#[tauri::command]
pub async fn link_status(url: String) -> Result<Status, String> {
    check_url(&url)?;
    tls();
    let e = |e: reqwest::Error| e.without_url().to_string();
    let client = reqwest::Client::builder().user_agent("Margin").timeout(std::time::Duration::from_secs(8)).redirect(reqwest::redirect::Policy::none()).build().map_err(e)?;
    let mut url = reqwest::Url::parse(&url).map_err(|e| e.to_string())?;
    let mut hops = vec![];
    for _ in 0..=HOPS {
        let mut r = client.head(url.clone()).send().await;
        if r.as_ref().is_ok_and(|r| r.status().as_u16() >= 400) {
            r = client.get(url.clone()).send().await;
        }
        let r = match r {
            Ok(r) => r,
            Err(x) => return Ok(verdict(&hops, Some(e(x)))),
        };
        let loc = r.headers().get(reqwest::header::LOCATION).and_then(|v| v.to_str().ok());
        let next = r.status().is_redirection().then(|| loc.and_then(|l| url.join(l).ok())).flatten();
        hops.push((r.status().as_u16(), next.as_ref().map(|u| u.to_string())));
        match next {
            Some(n) => url = n,
            None => break,
        }
    }
    Ok(verdict(&hops, None))
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

    #[test]
    fn web_links_dedupe() {
        let f = |p: &str, t: &str| Arc::new(crate::org::parse(Path::new(p), t, &crate::org::Kw { todo: vec![], done: vec![] }));
        let files = [
            f("/n/b.org", "[[https://a.org/x][A]] and [[http://b.org]]\n#+begin_src\n[[https://c.org]]\n#+end_src\n[[file:x.org]] https://bare.org\n"),
            f("/n/a.org", "* h\n[[HTTPS://a.org/x]] [[https://a.org/x]]\n"),
            f("/n/.trash/c.org", "[[https://d.org]]\n"),
        ];
        let v = web_links(&files, |p| p.starts_with("/n/.trash"));
        let got: Vec<_> = v.iter().map(|w| (w.url.as_str(), w.path.to_str().unwrap(), w.line, w.count)).collect();
        assert_eq!(got, [("HTTPS://a.org/x", "/n/a.org", 1, 1), ("https://a.org/x", "/n/a.org", 1, 2), ("http://b.org", "/n/b.org", 0, 1)]);
    }

    #[test]
    fn verdicts() {
        let h = |c: u16, to: Option<&str>| (c, to.map(String::from));
        let dead = |r: &str| Status::Dead { reason: r.into() };
        assert_eq!(verdict(&[h(200, None)], None), Status::Ok);
        assert_eq!(verdict(&[h(404, None)], None), dead("HTTP 404"));
        assert_eq!(verdict(&[], Some("timed out".into())), dead("timed out"));
        assert_eq!(verdict(&[h(301, Some("https://b/"))], Some("dns".into())), dead("dns"));
        assert_eq!(verdict(&[h(301, Some("https://b/")), h(308, Some("https://c/")), h(200, None)], None), Status::Moved { to: "https://c/".into() });
        assert_eq!(verdict(&[h(301, Some("https://b/")), h(302, Some("https://login/")), h(200, None)], None), Status::Moved { to: "https://b/".into() }, "a temporary hop isn't followed into the new URL");
        assert_eq!(verdict(&[h(302, Some("https://b/")), h(200, None)], None), Status::Ok);
        assert_eq!(verdict(&[h(301, Some("https://b/")), h(410, None)], None), dead("HTTP 410"));
        assert_eq!(verdict(&[h(302, None)], None), dead("HTTP 302 without a new location"));
        assert_eq!(verdict(&vec![h(302, Some("https://a/")); HOPS + 1], None), dead("more than 5 redirects"));
    }

    /// link_status against a local server: HEAD refused (405) falls back to GET, a 301 is followed and reported.
    #[test]
    fn checks_a_server() {
        use std::io::{Read, Write};
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        std::thread::spawn(move || {
            for s in l.incoming() {
                let mut s = s.unwrap();
                let (mut buf, mut req) = ([0; 1024], vec![]);
                while !req.windows(4).any(|w| w == b"\r\n\r\n") {
                    let n = s.read(&mut buf).unwrap();
                    req.extend_from_slice(&buf[..n]);
                }
                let req = String::from_utf8_lossy(&req);
                let res = match req.split_whitespace().take(2).collect::<Vec<_>>()[..] {
                    ["HEAD", _] => "405 Method Not Allowed",
                    [_, "/old"] => "301 Moved Permanently\r\nLocation: /new",
                    [_, "/new"] => "200 OK",
                    _ => "404 Not Found",
                };
                let _ = write!(s, "HTTP/1.1 {res}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
            }
        });
        let at = |p: &str| tauri::async_runtime::block_on(link_status(format!("http://127.0.0.1:{port}{p}"))).unwrap();
        assert_eq!(at("/new"), Status::Ok);
        assert_eq!(at("/old"), Status::Moved { to: format!("http://127.0.0.1:{port}/new") });
        assert_eq!(at("/gone"), Status::Dead { reason: "HTTP 404".into() });
        assert!(tauri::async_runtime::block_on(link_status("mailto:a@b.c".into())).is_err());
    }
}
