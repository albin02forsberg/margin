//! Ledger entries: plain list lines in `ledger.org` (notes dir), filed under `* YYYY-MM` headings:
//! `- 2026-10-25 -12.50 EUR lunch @anna #food [[id:…]]` (negative = expense, positive = income).
//! Dates stay unbracketed so entries never reach the agenda. Lines are only ever appended;
//! anything `parse_entry` doesn't understand is left as it is.
// link() is for the exports (#790, #791).
#![cfg_attr(not(test), allow(dead_code))]
use chrono::NaiveDate;
use regex::Regex;
use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::LazyLock;

static ENTRY: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*-\s+(\d{4}-\d{2}-\d{2})\s+(\S+)\s+([A-Z]{3})(?:\s+(.*?))?\s*$").unwrap());
static WORD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?:^|\s)([#@])([\w][\w.-]*)").unwrap());
static LINK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\[\[([^\]]+)\]").unwrap());

#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub date: NaiveDate,
    /// Signed amount in minor units (cents), so sums never pick up float error.
    pub cents: i64,
    pub currency: String,
    /// Free text, `@payee`, `#category` and links as written.
    pub text: String,
}

impl Entry {
    fn word(&self, sigil: &str) -> Option<&str> {
        WORD.captures_iter(&self.text).find(|c| &c[1] == sigil).map(|c| c.get(2).unwrap().as_str().trim_end_matches(['.', '-']))
    }
    /// The first `#tag`.
    pub fn category(&self) -> Option<&str> {
        self.word("#")
    }
    /// The first `@person`.
    pub fn payee(&self) -> Option<&str> {
        self.word("@")
    }
    /// The first `[[link]]` target, e.g. `id:…` or `file:…`.
    pub fn link(&self) -> Option<&str> {
        LINK.captures(&self.text).map(|c| c.get(1).unwrap().as_str())
    }
}

/// "12.50", "-12,5", "+1200" as signed cents; at most 2 decimals, no thousands separators, not zero.
fn amount(s: &str) -> Result<i64, String> {
    let bad = || format!("can't read the amount {s:?}: write it like 12.50 or 12,50");
    let (sign, num) = match s.strip_prefix('-') {
        Some(r) => (-1, r),
        None => (1, s.strip_prefix('+').unwrap_or(s)),
    };
    let (int, frac) = num.split_once(['.', ',']).unwrap_or((num, ""));
    let digits = |d: &str| d.bytes().all(|b| b.is_ascii_digit());
    if int.is_empty() || !digits(int) || !digits(frac) || frac.len() > 2 || (num.len() > int.len() && frac.is_empty()) {
        return Err(bad());
    }
    let cents = int.parse::<i64>().ok().and_then(|i| i.checked_mul(100)).and_then(|i| i.checked_add(format!("{frac:0<2}").parse::<i64>().unwrap())).ok_or_else(bad)?;
    if cents == 0 {
        return Err("the amount can't be zero".into());
    }
    Ok(sign * cents)
}

/// LINE as an entry, or None (headings, notes, hand-written lines in another shape).
pub fn parse_entry(line: &str) -> Option<Entry> {
    let c = ENTRY.captures(line)?;
    Some(Entry { date: c[1].parse().ok()?, cents: amount(&c[2]).ok()?, currency: c[3].into(), text: c.get(4).map_or("", |m| m.as_str()).into() })
}

/// E as a ledger line (always signed, two decimals).
pub fn format_entry(e: &Entry) -> String {
    let sign = if e.cents < 0 { '-' } else { '+' };
    let a = e.cents.unsigned_abs();
    let line = format!("- {} {sign}{}.{:02} {}", e.date, a / 100, a % 100, e.currency);
    if e.text.is_empty() { line } else { format!("{line} {}", e.text) }
}

/// Quick-add INPUT `[YYYY-MM-DD] [+|-]AMOUNT [CUR] text…`: unsigned is an expense, `+` income;
/// the date defaults to TODAY and the currency to CURRENCY.
pub fn parse_quick(input: &str, today: NaiveDate, currency: &str) -> Result<Entry, String> {
    let mut w: Vec<&str> = input.split_whitespace().collect();
    let date = match w.first().and_then(|d| d.parse::<NaiveDate>().ok()) {
        Some(d) => {
            w.remove(0);
            d
        }
        None => today,
    };
    if w.is_empty() {
        return Err("write an amount, e.g. 12.50 lunch @anna #food".into());
    }
    let a = w.remove(0);
    let cents = amount(a)?;
    let cents = if a.starts_with('+') { cents } else { -cents.abs() };
    let cur = match w.first() {
        Some(c) if c.len() == 3 && c.bytes().all(|b| b.is_ascii_uppercase()) => w.remove(0).to_string(),
        _ => currency.trim().to_uppercase(),
    };
    if cur.len() != 3 || !cur.bytes().all(|b| b.is_ascii_uppercase()) {
        return Err(format!("the currency {cur:?} isn't a 3-letter code like EUR"));
    }
    Ok(Entry { date, cents, currency: cur, text: w.join(" ") })
}

/// TEXT (the ledger file) with E filed at the end of its `* YYYY-MM` heading (added when missing).
pub fn add(text: &str, e: &Entry, kw: &crate::org::Kw) -> String {
    crate::org::capture_insert(text, Some(&e.date.format("%Y-%m").to_string()), &format_entry(e), kw)
}

/// One currency's totals for a month, in cents: EXPENSE is negative, NET = INCOME + EXPENSE.
/// CATEGORIES (None = uncategorised) and PAYEES are net sums, largest amount first.
#[derive(Serialize, Debug, PartialEq, Default)]
pub struct Total {
    pub currency: String,
    pub income: i64,
    pub expense: i64,
    pub net: i64,
    pub categories: Vec<(Option<String>, i64)>,
    pub payees: Vec<(String, i64)>,
}

/// A month's entry, at 0-based LINE of the ledger file.
#[derive(Serialize, Debug, PartialEq)]
pub struct Row {
    pub line: usize,
    pub date: NaiveDate,
    pub cents: i64,
    pub currency: String,
    pub text: String,
    pub category: Option<String>,
    pub payee: Option<String>,
}

/// TEXT's entries dated in MONTH (`YYYY-MM`, whatever heading they sit under), oldest first, and their
/// totals per currency (CURRENCY first, then by code; never converted).
pub fn summarize(text: &str, month: &str, currency: &str) -> (Vec<Total>, Vec<Row>) {
    let mut rows: Vec<Row> = text.lines().enumerate().filter_map(|(line, l)| {
        let e = parse_entry(l).filter(|e| e.date.format("%Y-%m").to_string() == month)?;
        let (category, payee) = (e.category().map(str::to_string), e.payee().map(str::to_string));
        Some(Row { line, date: e.date, cents: e.cents, currency: e.currency, text: e.text, category, payee })
    }).collect();
    rows.sort_by_key(|r| (r.date, r.line));
    type Sums = (i64, i64, BTreeMap<Option<String>, i64>, BTreeMap<String, i64>);
    let mut by: BTreeMap<(bool, &str), Sums> = BTreeMap::new();
    for r in &rows {
        let t = by.entry((r.currency != currency.trim().to_uppercase(), &r.currency)).or_default();
        if r.cents > 0 { t.0 += r.cents } else { t.1 += r.cents }
        *t.2.entry(r.category.clone()).or_default() += r.cents;
        if let Some(p) = &r.payee {
            *t.3.entry(p.clone()).or_default() += r.cents;
        }
    }
    fn ranked<K: Ord>(m: BTreeMap<K, i64>) -> Vec<(K, i64)> {
        let mut v: Vec<_> = m.into_iter().collect();
        v.sort_by_key(|(_, c)| std::cmp::Reverse(c.unsigned_abs())); // stable: ties stay by name
        v
    }
    let totals = by.into_iter().map(|((_, cur), (income, expense, c, p))| Total { currency: cur.into(), income, expense, net: income + expense, categories: ranked(c), payees: ranked(p) }).collect();
    (totals, rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn d(s: &str) -> NaiveDate {
        s.parse().unwrap()
    }
    fn kw() -> crate::org::Kw {
        crate::org::Kw { todo: vec!["TODO".into()], done: vec!["DONE".into()] }
    }

    #[test]
    fn entries_round_trip_and_hand_edits_parse() {
        let e = parse_entry("- 2026-10-25 -12.50 EUR lunch @anna #food [[id:abc]]").unwrap();
        assert_eq!((e.cents, e.currency.as_str(), e.category(), e.payee(), e.link()), (-1250, "EUR", Some("food"), Some("anna"), Some("id:abc")));
        assert_eq!(parse_entry(&format_entry(&e)), Some(e));
        let h = parse_entry("  -   2026-10-01    +1200,5   SEK   invoice  #work  ").unwrap();
        assert_eq!((h.cents, h.text.as_str()), (120050, "invoice  #work"));
        assert_eq!(format_entry(&h), "- 2026-10-01 +1200.50 SEK invoice  #work");
        assert_eq!(parse_entry(&format_entry(&h)), Some(h));
        assert_eq!(format_entry(&parse_entry("- 2026-10-01 7 EUR").unwrap()), "- 2026-10-01 +7.00 EUR");
    }

    #[test]
    fn other_lines_are_not_entries() {
        for l in ["* 2026-10", "- 2026-10-25 lunch", "- <2026-10-25> -1 EUR x", "- 2026-13-01 -1 EUR x", "- 2026-10-25 -1,234.50 EUR x", "- 2026-10-25 0 EUR x", "- 2026-10-25 -1 eur x", "note", ""] {
            assert_eq!(parse_entry(l), None, "{l}");
        }
        // # and @ only start a word: links and e-mail addresses aren't categories or payees.
        let e = parse_entry("- 2026-10-25 -1 EUR mail a@b.se [[file:x.org::#top]]").unwrap();
        assert_eq!((e.category(), e.payee()), (None, None));
        assert_eq!(parse_entry("- 2026-10-25 -1 EUR @anna. paid").unwrap().payee(), Some("anna"));
    }

    #[test]
    fn quick_add_reads_sign_date_and_currency() {
        let t = d("2026-10-25");
        let q = |s| parse_quick(s, t, "eur");
        assert_eq!(q("12.50 lunch @anna #food"), Ok(Entry { date: t, cents: -1250, currency: "EUR".into(), text: "lunch @anna #food".into() }));
        assert_eq!(q("+1200 USD   invoice").map(|e| (e.cents, e.currency)), Ok((120000, "USD".into())));
        assert_eq!(q("-3,5 coffee").map(|e| e.cents), Ok(-350));
        assert_eq!(q("2026-10-01 9 taxi").map(|e| (e.date, e.cents)), Ok((d("2026-10-01"), -900)));
        assert_eq!(q("9 EUROS").map(|e| (e.currency, e.text)), Ok(("EUR".into(), "EUROS".into())));
        for bad in ["", "lunch 12", "0 free", "1,234.50 tv", "1.234 tv", "12. x", ".5 x", "1e3 x"] {
            assert!(q(bad).is_err(), "{bad}");
        }
        assert!(parse_quick("1 x", t, "euro").is_err());
    }

    #[test]
    fn add_files_under_the_month_heading() {
        let e = |s| parse_quick(s, d("2026-10-25"), "EUR").unwrap();
        let t = add("#+title: ledger\n", &e("12.50 lunch"), &kw());
        assert_eq!(t, "#+title: ledger\n* 2026-10\n- 2026-10-25 -12.50 EUR lunch\n");
        let t = add(&format!("{t}not an entry  #kept\n* 2026-11\n- 2026-11-01 -1.00 EUR x\n"), &e("+5 refund"), &kw());
        assert_eq!(t, "#+title: ledger\n* 2026-10\n- 2026-10-25 -12.50 EUR lunch\nnot an entry  #kept\n- 2026-10-25 +5.00 EUR refund\n* 2026-11\n- 2026-11-01 -1.00 EUR x\n");
        assert!(add(&t, &e("2026-12-01 2 y"), &kw()).ends_with("- 2026-11-01 -1.00 EUR x\n* 2026-12\n- 2026-12-01 -2.00 EUR y\n"));
    }

    #[test]
    fn summarize_groups_a_month_by_currency() {
        let text = "* 2026-10\n- 2026-10-25 -12.50 EUR lunch @anna #food\n- 2026-10-02 +1200 EUR pay @acme #work\nbroken -1 EUR\n- 2026-10-03 -2.50 EUR bus\n- 2026-10-04 -7 EUR dinner @anna #food\n- 2026-10-05 -100 SEK fika #food\n* 2026-11\n- 2026-11-01 -1 EUR later\n- 2026-10-31 +0.5 USD late filing\n";
        let (t, rows) = summarize(text, "2026-10", "eur");
        assert_eq!(rows.iter().map(|r| r.line).collect::<Vec<_>>(), [2, 4, 5, 6, 1, 9]);
        assert_eq!((rows[0].category.as_deref(), rows[0].payee.as_deref(), rows[1].category.as_deref()), (Some("work"), Some("acme"), None));
        assert_eq!(t.iter().map(|t| t.currency.as_str()).collect::<Vec<_>>(), ["EUR", "SEK", "USD"]);
        assert_eq!((t[0].income, t[0].expense, t[0].net), (120000, -2200, 117800));
        assert_eq!(t[0].categories, [(Some("work".into()), 120000), (Some("food".into()), -1950), (None, -250)]);
        assert_eq!(t[0].payees, [("acme".into(), 120000), ("anna".into(), -1950)]);
        assert_eq!((t[1].income, t[1].expense, t[1].net, t[2].net), (0, -10000, -10000, 50));
        assert_eq!(summarize(text, "2026-09", "EUR"), (vec![], vec![]));
        assert_eq!(summarize("", "2026-10", "EUR"), (vec![], vec![]));
    }
}
