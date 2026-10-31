//! Feeds (#981): new RSS and Atom items as reading-list TODOs. This fetches and picks them; the frontend
//! files them like a capture and only then marks them seen (`feeds_seen.json` in data_dir).

use crate::config::Feed;
use crate::weblink::{check_url, decode};
use serde::Serialize;
use std::collections::{BTreeMap, HashSet};
use std::path::Path;

/// Bytes of a feed read; a cut-off last item is skipped.
const MAX: usize = 2 * 1024 * 1024;
/// Items filed from a feed fetched for the first time; the older ones are only marked seen.
pub const FIRST: usize = 20;
/// Guids remembered per feed (more if the feed itself lists more).
const KEEP: usize = 500;
pub const FILE: &str = "feeds_seen.json";

/// Seen guids per feed URL, oldest first.
pub type Seen = BTreeMap<String, Vec<String>>;

#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub guid: String,
    pub title: String,
    pub link: String,
}

/// An org entry to file under HEADING in FILE (a capture_path pattern).
#[derive(Serialize, Debug, PartialEq)]
pub struct New {
    pub file: String,
    pub heading: String,
    pub entry: String,
}

/// What `fetch_feeds` found: entries to file, every fetched feed's guids (for `mark_feeds_seen` once
/// filed) and one error per feed that failed.
#[derive(Serialize, Debug, Default)]
pub struct Fetched {
    pub items: Vec<New>,
    pub seen: Seen,
    pub errors: Vec<String>,
}

/// CDATA sections in S as escaped text, so tag scanning skips them and `decode` restores them.
fn uncdata(s: &str) -> String {
    let (mut out, mut rest) = (String::with_capacity(s.len()), s);
    while let Some(i) = rest.find("<![CDATA[") {
        out.push_str(&rest[..i]);
        let body = &rest[i + 9..];
        let end = body.find("]]>").unwrap_or(body.len());
        out += &body[..end].replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
        rest = &body[(end + 3).min(body.len())..];
    }
    out + rest
}

/// The first `<NAME …>` tag in S (not `<NAMEx` or `<ns:NAME`): its attributes and the offset after its `>`.
fn tag<'a>(s: &'a str, name: &str) -> Option<(&'a str, usize)> {
    let pat = format!("<{name}");
    let mut from = 0;
    while let Some(i) = s[from..].find(&pat).map(|i| from + i + pat.len()) {
        if s[i..].starts_with(|c: char| c == '>' || c == '/' || c.is_whitespace()) {
            let end = i + s[i..].find('>')?;
            return Some((&s[i..end], end + 1));
        }
        from = i;
    }
    None
}

/// Attribute NAME's decoded value in ATTRS (`name="v"` or `name='v'`).
fn attr(attrs: &str, name: &str) -> Option<String> {
    let mut from = 0;
    while let Some(i) = attrs[from..].find(name).map(|i| from + i) {
        from = i + name.len();
        let v = attrs[from..].trim_start().strip_prefix('=').map(str::trim_start);
        if let Some((q, v)) = v.and_then(|v| v.chars().next().filter(|q| *q == '"' || *q == '\'').map(|q| (q, &v[1..]))).filter(|_| attrs[..i].ends_with(char::is_whitespace)) {
            return Some(decode(&v[..v.find(q)?]).trim().to_string());
        }
    }
    None
}

/// The decoded, whitespace-collapsed text of the first `<NAME>` element in S; None if empty or missing.
fn text(s: &str, name: &str) -> Option<String> {
    let (_, start) = tag(s, name).filter(|(a, _)| !a.ends_with('/'))?;
    let end = start + s[start..].find(&format!("</{name}"))?;
    let t = decode(&s[start..end]).split_whitespace().collect::<Vec<_>>().join(" ");
    (!t.is_empty()).then_some(t)
}

/// An item's link: Atom's `<link href>` (rel alternate or none first), else RSS's `<link>url</link>`.
fn link(item: &str) -> Option<String> {
    let (mut other, mut rest) = (None, item);
    while let Some((attrs, after)) = tag(rest, "link") {
        if let Some(h) = attr(attrs, "href") {
            if attr(attrs, "rel").is_none_or(|r| r == "alternate") {
                return Some(h);
            }
            other.get_or_insert(h);
        }
        rest = &rest[after..];
    }
    other.or_else(|| text(item, "link"))
}

/// The items of an RSS (2.0 or 1.0) or Atom feed, in its order (newest first, as feeds list them).
/// Items without an http(s) link are skipped; the guid (`<guid>`, `<id>`) falls back to the link, the title to it too.
pub fn parse(body: &str) -> Vec<Item> {
    let body = uncdata(body);
    let mut out = vec![];
    for name in ["item", "entry"] {
        let (close, mut rest) = (format!("</{name}>"), body.as_str());
        while let Some((attrs, start)) = tag(rest, name) {
            if attrs.ends_with('/') {
                rest = &rest[start..];
                continue;
            }
            let Some(end) = rest[start..].find(&close).map(|e| start + e) else { break };
            let b = &rest[start..end];
            if let Some(link) = link(b).filter(|l| check_url(l).is_ok()) {
                let guid = text(b, "guid").or_else(|| text(b, "id")).unwrap_or_else(|| link.clone());
                out.push(Item { guid, title: text(b, "title").unwrap_or_else(|| link.clone()), link });
            }
            rest = &rest[end..];
        }
    }
    out
}

/// The feeds of an OPML subscription list as (name, url): outlines with an http(s) `xmlUrl` at any nesting depth,
/// the name from `title`, else `text`, else the URL; a URL listed twice counts once.
pub fn opml(body: &str) -> Vec<(String, String)> {
    let (mut out, mut rest) = (vec![], body);
    while let Some((attrs, after)) = tag(rest, "outline") {
        rest = &rest[after..];
        let Some(url) = attr(attrs, "xmlUrl").filter(|u| check_url(u).is_ok()) else { continue };
        if out.iter().all(|(_, u)| *u != url) {
            let name = attr(attrs, "title").filter(|t| !t.is_empty()).or_else(|| attr(attrs, "text").filter(|t| !t.is_empty()));
            out.push((name.unwrap_or_else(|| url.clone()), url));
        }
    }
    out
}

/// IT as a reading-list TODO stamped NOW, its guid in `:FEED_ID:`.
pub fn entry(it: &Item, now: chrono::NaiveDateTime) -> String {
    let link = it.link.replace('[', "%5B").replace(']', "%5D").replace(' ', "%20");
    let title = it.title.replace('[', "(").replace(']', ")");
    format!("* TODO Read [[{link}][{title}]] :read:\n:PROPERTIES:\n:FEED_ID: {}\n:END:\n{}", it.guid, now.format("[%Y-%m-%d %a %H:%M]"))
}

/// ITEMS (newest first) whose guid isn't in SEEN, oldest first for filing; only the newest FIRST of a feed never
/// fetched before (SEEN None), so subscribing doesn't flood the list.
pub fn fresh(items: &[Item], seen: Option<&Vec<String>>) -> Vec<Item> {
    let mut guids = HashSet::new();
    let mut v: Vec<Item> = items.iter().filter(|i| guids.insert(&i.guid) && seen.is_none_or(|s| !s.contains(&i.guid))).take(if seen.is_some() { usize::MAX } else { FIRST }).cloned().collect();
    v.reverse();
    v
}

/// SEEN with each feed's current guids in NEW (newest first) moved to its end, oldest guids dropped past KEEP.
pub fn mark(seen: &mut Seen, new: Seen) {
    for (url, guids) in new {
        let s = seen.entry(url).or_default();
        for g in guids.iter().rev() {
            s.retain(|x| x != g);
            s.push(g.clone());
        }
        let n = s.len().saturating_sub(KEEP.max(guids.len()));
        s.drain(..n);
    }
}

/// The new entries of FEEDS given each one's fetched body (or error) in RESULTS; a link filed by an earlier
/// feed is skipped (but marked seen). A feed's empty FILE is the INBOX.
pub fn pick(feeds: &[Feed], results: Vec<Result<String, String>>, seen: &Seen, inbox: &str, now: chrono::NaiveDateTime) -> Fetched {
    let (mut f, mut links) = (Fetched::default(), HashSet::new());
    for (feed, r) in feeds.iter().zip(results) {
        let name = if feed.name.trim().is_empty() { &feed.url } else { &feed.name };
        let body = match r {
            Ok(b) if b.contains("<rss") || b.contains("<feed") || b.contains("<rdf:RDF") => b,
            r => {
                f.errors.push(format!("{name}: {}", r.err().unwrap_or_else(|| "not an RSS or Atom feed".into())));
                continue;
            }
        };
        let items = parse(&body);
        for it in fresh(&items, seen.get(&feed.url)) {
            if links.insert(it.link.clone()) {
                let file = if feed.file.trim().is_empty() { inbox } else { feed.file.trim() };
                f.items.push(New { file: file.into(), heading: feed.heading.clone(), entry: entry(&it, now) });
            }
        }
        f.seen.entry(feed.url.clone()).or_default().extend(items.into_iter().map(|i| i.guid));
    }
    f
}

/// Fetch FEEDS (10 s timeout each, the first 2 MB) and pick their new items.
pub async fn fetch(feeds: &[Feed], seen: &Seen, inbox: &str, now: chrono::NaiveDateTime) -> Fetched {
    let ok = |ct: Option<&str>| ct.is_none_or(|c| ["xml", "rss", "atom", "text/plain"].iter().any(|k| c.to_ascii_lowercase().contains(k)));
    let mut results = vec![];
    for feed in feeds {
        results.push(match check_url(&feed.url) {
            Ok(()) => crate::weblink::get(&feed.url, MAX, 10, ok, "not an RSS or Atom feed").await,
            Err(_) => Err("only http(s) feeds are fetched".into()),
        });
    }
    pick(feeds, results, seen, inbox, now)
}

/// DATA's feeds_seen.json; missing is empty.
pub fn load(data: &Path) -> Result<Seen, String> {
    let path = data.join(FILE);
    match crate::timeclock::read_or_empty(&path)? {
        t if t.is_empty() => Ok(Seen::new()),
        t => serde_json::from_str(&t).map_err(|e| format!("{}: {e}", path.display())),
    }
}

/// Add NEW to DATA's feeds_seen.json (mark).
pub fn save(data: &Path, new: Seen) -> Result<(), String> {
    let mut seen = load(data)?;
    mark(&mut seen, new);
    std::fs::create_dir_all(data).map_err(|e| e.to_string())?;
    crate::config::write_atomic(&data.join(FILE), serde_json::to_string(&seen).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

/// Stamp DATA's feeds_last.txt with the unix time of this fetch.
pub fn stamp(data: &Path) -> Result<(), String> {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|e| e.to_string())?.as_secs();
    crate::config::write_atomic(&data.join("feeds_last.txt"), now.to_string()).map_err(|e| e.to_string())
}

/// The unix time `stamp` wrote (None if never).
pub fn last(data: &Path) -> Option<u64> {
    std::fs::read_to_string(data.join("feeds_last.txt")).ok()?.trim().parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const RSS: &str = r#"<?xml version="1.0"?><rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom"><channel>
<title>Blog</title><link>https://blog.example/</link><atom:link href="https://blog.example/feed" rel="self"/>
<item><title><![CDATA[Rust & <Tauri> [v2]]]></title><link>https://blog.example/a</link><guid isPermaLink="false">a-1</guid>
<description><![CDATA[<p>has <item> and <link>https://x.example/</link> inside</p>]]></description></item>
<item><title>Caf&#233; &amp; more</title><link>
  https://blog.example/b
</link></item>
<item><title>No link</title><guid>c</guid></item>
<item><title>Mail</title><link>mailto:a@b.c</link></item>
</channel></rss>"#;

    const ATOM: &str = r#"<?xml version="1.0" encoding="utf-8"?><feed xmlns="http://www.w3.org/2005/Atom"><title>Atom</title>
<link href="https://at.example/" rel="alternate"/>
<entry><title type="html">A &lt;b&gt;bold&lt;/b&gt; claim</title><link rel="edit" href="https://at.example/edit/1"/><link rel='alternate' type="text/html" href='https://at.example/1?a=1&amp;b=2'/><id>urn:uuid:1</id></entry>
<entry><title>Only edit</title><link rel="edit" href="https://at.example/edit/2"/><id>urn:uuid:2</id></entry>
<entry><title>Cut off at the size cap</title><link href="https://at.example/3"/>"#;

    #[test]
    fn opml_flattens_categories() {
        let x = r#"<opml version="2.0"><body>
<outline text="Tech" title="Tech"><outline type="rss" text="A &amp; B" xmlUrl="https://a.example/feed"/>
<outline type="rss" text="dup" title="Dup" xmlUrl="https://a.example/feed"/></outline>
<outline text="No url"/><outline text="Mail" xmlUrl="mailto:a@b.c"/><outline text="File" xmlUrl="file:///etc/passwd"/>
<outline type="rss" xmlUrl='https://c.example/rss' title="C"/><outline xmlUrl="https://d.example/x"/></body></opml>"#;
        assert_eq!(opml(x), [("A & B".into(), "https://a.example/feed".into()), ("C".into(), "https://c.example/rss".into()), ("https://d.example/x".into(), "https://d.example/x".into())]);
        assert!(opml("<opml><body><outline").is_empty());
    }

    fn items(n: usize) -> Vec<Item> {
        (0..n).map(|i| Item { guid: format!("g{i}"), title: format!("t{i}"), link: format!("https://x.example/{i}") }).collect()
    }

    #[test]
    fn rss() {
        let v = parse(RSS);
        assert_eq!(v.len(), 2, "{v:?}");
        assert_eq!(v[0], Item { guid: "a-1".into(), title: "Rust & <Tauri> [v2]".into(), link: "https://blog.example/a".into() });
        assert_eq!(v[1], Item { guid: "https://blog.example/b".into(), title: "Café & more".into(), link: "https://blog.example/b".into() }, "no guid: the link");
    }

    #[test]
    fn atom() {
        let v = parse(ATOM);
        assert_eq!(v.len(), 2, "{v:?}");
        assert_eq!(v[0], Item { guid: "urn:uuid:1".into(), title: "A <b>bold</b> claim".into(), link: "https://at.example/1?a=1&b=2".into() });
        assert_eq!(v[1].link, "https://at.example/edit/2", "another rel if there is nothing better");
        assert!(parse("<html><body><item/></body></html>").is_empty());
    }

    #[test]
    fn entries() {
        let now = "2026-10-31T09:05:00".parse().unwrap();
        let it = Item { guid: "a-1".into(), title: "Rust & <Tauri> [v2]".into(), link: "https://x.example/a b[1]".into() };
        assert_eq!(entry(&it, now), "* TODO Read [[https://x.example/a%20b%5B1%5D][Rust & <Tauri> (v2)]] :read:\n:PROPERTIES:\n:FEED_ID: a-1\n:END:\n[2026-10-31 Sat 09:05]");
    }

    #[test]
    fn fresh_and_mark() {
        let all = items(30);
        let first = fresh(&all, None);
        assert_eq!(first.len(), FIRST, "a new feed files only its newest");
        assert_eq!((first[0].guid.as_str(), first[FIRST - 1].guid.as_str()), ("g19", "g0"), "oldest first");
        let mut seen = Seen::new();
        mark(&mut seen, Seen::from([("u".into(), all.iter().map(|i| i.guid.clone()).collect())]));
        assert!(fresh(&all, seen.get("u")).is_empty(), "all 30 were marked, not just the 20 filed");
        let mut more = items(32);
        more.rotate_right(2); // g30, g31 are the newest
        more.push(more[0].clone());
        assert_eq!(fresh(&more, seen.get("u")).iter().map(|i| i.guid.as_str()).collect::<Vec<_>>(), ["g31", "g30"], "new ones once, oldest first");
        // Past KEEP the oldest guids not in the feed go.
        let mut seen = Seen::from([("u".into(), (0..KEEP).map(|i| format!("old{i}")).collect())]);
        mark(&mut seen, Seen::from([("u".into(), vec!["n1".into(), "old0".into()])]));
        let s = &seen["u"];
        assert_eq!((s.len(), s[0].as_str(), &s[KEEP - 2..]), (KEEP, "old2", &["old0".to_string(), "n1".into()][..]));
        let mut seen = Seen::new();
        mark(&mut seen, Seen::from([("u".into(), (0..KEEP + 5).map(|i| format!("g{i}")).collect())]));
        assert_eq!(seen["u"].len(), KEEP + 5, "a feed longer than KEEP keeps all its guids");
    }

    #[test]
    fn picks() {
        let now = "2026-10-31T09:05:00".parse().unwrap();
        let feed = |name: &str, url: &str, file: &str| Feed { name: name.into(), url: url.into(), file: file.into(), heading: "Reading".into() };
        let feeds = [feed("A", "https://a/rss", ""), feed("", "https://b/rss", "news.org"), feed("C", "https://c/rss", ""), feed("D", "https://d", "")];
        let dup = RSS.replace("a-1", "other-guid");
        let results = vec![Ok(RSS.to_string()), Ok(dup.clone()), Err("HTTP 404".into()), Ok("<html>hi</html>".into())];
        let f = pick(&feeds, results, &Seen::new(), "inbox.org", now);
        assert_eq!(f.items.len(), 2, "the second feed's links were filed by the first: {:?}", f.items);
        assert!(f.items.iter().all(|n| n.file == "inbox.org" && n.heading == "Reading"));
        assert!(f.items[0].entry.contains("[[https://blog.example/b][Café & more]]"), "oldest first: {}", f.items[0].entry);
        assert_eq!(f.seen["https://b/rss"], ["other-guid", "https://blog.example/b"], "skipped duplicates are still seen");
        assert!(!f.seen.contains_key("https://c/rss") && !f.seen.contains_key("https://d"));
        assert_eq!(f.errors, ["C: HTTP 404", "D: not an RSS or Atom feed"]);
        // Second run with the seen state: nothing again.
        let mut seen = Seen::new();
        mark(&mut seen, f.seen);
        let f = pick(&feeds[..2], vec![Ok(RSS.to_string()), Ok(dup)], &seen, "inbox.org", now);
        assert!(f.items.is_empty() && f.errors.is_empty(), "{f:?}");
    }

    #[test]
    fn seen_file() {
        let d = std::env::temp_dir().join(format!("margin-feeds-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        assert!(load(&d).unwrap().is_empty());
        save(&d, Seen::from([("u".into(), vec!["b".into(), "a".into()])])).unwrap();
        save(&d, Seen::from([("v".into(), vec!["x".into()])])).unwrap();
        assert_eq!(load(&d).unwrap(), Seen::from([("u".into(), vec!["a".into(), "b".into()]), ("v".into(), vec!["x".into()])]));
        std::fs::remove_dir_all(&d).unwrap();
    }
}
