//! Ledger entries: plain list lines in `ledger.org` (notes dir), filed under `* YYYY-MM` headings:
//! `- 2026-10-25 -12.50 EUR lunch @anna #food [[id:…]]` (negative = expense, positive = income).
//! Dates stay unbracketed so entries never reach the agenda. Lines are only ever appended;
//! anything `parse_entry` doesn't understand is left as it is.
use chrono::NaiveDate;
use regex::Regex;
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, LazyLock};

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
    let line = format!("- {} {} {}", e.date, signed(e.cents), e.currency);
    if e.text.is_empty() { line } else { format!("{line} {}", e.text) }
}

/// CENTS as `-12.50` / `+1200.00`.
fn signed(cents: i64) -> String {
    let a = cents.unsigned_abs();
    format!("{}{}.{:02}", if cents < 0 { '-' } else { '+' }, a / 100, a % 100)
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
    let rows = rows(text, |e| e.date.format("%Y-%m").to_string() == month);
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

/// TEXT's entries that KEEP accepts, oldest first.
fn rows(text: &str, keep: impl Fn(&Entry) -> bool) -> Vec<Row> {
    let mut rows: Vec<Row> = text.lines().enumerate().filter_map(|(line, l)| {
        let e = parse_entry(l).filter(&keep)?;
        let (category, payee) = (e.category().map(str::to_string), e.payee().map(str::to_string));
        Some(Row { line, date: e.date, cents: e.cents, currency: e.currency, text: e.text, category, payee })
    }).collect();
    rows.sort_by_key(|r| (r.date, r.line));
    rows
}

/// Every note and heading `:ID:` in FILES with its org category, for resolving entries' `[[id:…]]` links.
pub fn id_categories(files: &[Arc<crate::org::OrgFile>]) -> HashMap<String, String> {
    let mut m = HashMap::new();
    for f in files {
        m.extend(f.id.iter().map(|id| (id.clone(), f.category.clone())));
        m.extend(f.headlines.iter().filter_map(|h| Some((h.id.clone()?, h.category.clone()))));
    }
    m
}

/// Whether E belongs to PROJECT, the rule the project cards use for tasks: its `#category` is the project's name (any
/// case), or its `[[id:…]]` link points at a note or heading (IDS) in that category. An unknown ID counts as unlinked.
pub fn belongs(e: &Entry, project: &str, ids: &HashMap<String, String>) -> bool {
    let is = |c: &str| c.eq_ignore_ascii_case(project);
    e.category().is_some_and(is) || e.link().and_then(|l| ids.get(l.strip_prefix("id:")?.split("::").next()?.trim())).is_some_and(|c| is(c))
}

/// All-time income and expense of each of NAMES in TEXT, per currency (by code; only income, expense and net set).
/// Projects without entries are left out.
pub fn project_totals(text: &str, names: &[String], ids: &HashMap<String, String>) -> BTreeMap<String, Vec<Total>> {
    let entries: Vec<Entry> = text.lines().filter_map(parse_entry).collect();
    let mut out = BTreeMap::new();
    for n in names {
        let mut by: BTreeMap<&str, Total> = BTreeMap::new();
        for e in entries.iter().filter(|e| belongs(e, n, ids)) {
            let t = by.entry(&e.currency).or_insert_with(|| Total { currency: e.currency.clone(), ..Default::default() });
            if e.cents > 0 { t.income += e.cents } else { t.expense += e.cents }
            t.net += e.cents;
        }
        if !by.is_empty() {
            out.insert(n.clone(), by.into_values().collect());
        }
    }
    out
}

/// TEXT's entries whose `@payee` is NAME (any case, leading `@` optional), oldest first.
pub fn payments(text: &str, name: &str) -> Vec<Row> {
    let name = name.trim().trim_start_matches('@');
    if name.is_empty() {
        return vec![];
    }
    rows(text, |e| e.payee().is_some_and(|p| p.eq_ignore_ascii_case(name)))
}

// ---------------------------------------------------------------- CSV (#791)

/// CSV TEXT split on DELIM into rows of fields. `"…"` quotes (`""` for a quote) may hold the delimiter and line
/// breaks; a leading BOM, `\r` and blank lines are dropped. Hand-written: no CSV crate for this much.
pub fn csv_rows(text: &str, delim: char) -> Vec<Vec<String>> {
    let (mut rows, mut row, mut f, mut q) = (vec![], vec![], String::new(), false);
    let mut it = text.trim_start_matches('\u{feff}').chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '"' if q && it.peek() == Some(&'"') => {
                f.push('"');
                it.next();
            }
            '"' if q => q = false,
            '"' if f.is_empty() => q = true,
            c if q => f.push(c),
            '\r' => {}
            c if c == delim => row.push(std::mem::take(&mut f)),
            '\n' => {
                row.push(std::mem::take(&mut f));
                if row.len() > 1 || !row[0].is_empty() { rows.push(std::mem::take(&mut row)) } else { row.clear() }
            }
            c => f.push(c),
        }
    }
    if !f.is_empty() || !row.is_empty() {
        row.push(f);
        rows.push(row);
    }
    rows
}

/// One CSV field, quoted when it holds a comma, quote or line break.
fn csv_field(s: &str) -> String {
    if s.contains([',', '"', '\n', '\r']) { format!("\"{}\"", s.replace('"', "\"\"")) } else { s.into() }
}

/// TEXT's entries dated FROM..=TO as CSV (date, amount, currency, category, payee, link, text), oldest first.
pub fn to_csv(text: &str, from: NaiveDate, to: NaiveDate) -> String {
    let mut es: Vec<Entry> = text.lines().filter_map(parse_entry).filter(|e| (from..=to).contains(&e.date)).collect();
    es.sort_by_key(|e| e.date);
    let mut out = String::from("date,amount,currency,category,payee,link,text\n");
    for e in &es {
        let f = [e.date.to_string(), signed(e.cents), e.currency.clone(), e.category().unwrap_or_default().into(), e.payee().unwrap_or_default().into(), e.link().unwrap_or_default().into(), e.text.clone()];
        out += &f.iter().map(|s| csv_field(s)).collect::<Vec<_>>().join(",");
        out.push('\n');
    }
    out
}

/// Which columns (0-based) of a bank CSV hold what, and how it is written. As text (`spec`, 1-based):
/// `date=1 amount=3 text=2+4 currency=5 delimiter=; decimal=,` (currency is optional; `delimiter=tab`).
#[derive(Debug, Clone, PartialEq)]
pub struct Mapping {
    pub date: usize,
    pub amount: usize,
    pub text: Vec<usize>,
    pub currency: Option<usize>,
    pub delim: char,
    pub decimal: char,
}

impl Mapping {
    pub fn parse(spec: &str) -> Result<Mapping, String> {
        let mut m = Mapping { date: usize::MAX, amount: usize::MAX, text: vec![], currency: None, delim: ',', decimal: '.' };
        let col = |v: &str| v.trim().parse::<usize>().ok().filter(|&n| n > 0).map(|n| n - 1).ok_or_else(|| format!("{v:?} isn't a column number (1, 2, …)"));
        for w in spec.split_whitespace() {
            let (k, v) = w.split_once('=').ok_or_else(|| format!("write {w:?} as key=value"))?;
            match k {
                "date" => m.date = col(v)?,
                "amount" => m.amount = col(v)?,
                "text" => m.text = v.split('+').map(col).collect::<Result<_, _>>()?,
                "currency" => m.currency = Some(col(v)?),
                "delimiter" => m.delim = match v { "tab" => '\t', "," | ";" | "|" => v.chars().next().unwrap(), _ => return Err(format!("the delimiter {v:?} isn't , ; | or tab")) },
                "decimal" => m.decimal = match v { "." | "," => v.chars().next().unwrap(), _ => return Err(format!("the decimal separator {v:?} isn't . or ,")) },
                _ => return Err(format!("unknown key {k:?}: use date, amount, text, currency, delimiter or decimal")),
            }
        }
        if m.date == usize::MAX || m.amount == usize::MAX {
            return Err("say which columns hold the date and the amount, e.g. date=1 amount=3".into());
        }
        Ok(m)
    }

    pub fn spec(&self) -> String {
        let n = |c: usize| (c + 1).to_string();
        let text = self.text.iter().map(|&c| n(c)).collect::<Vec<_>>().join("+");
        let mut s = format!("date={} amount={}", n(self.date), n(self.amount));
        if !text.is_empty() {
            s += &format!(" text={text}");
        }
        if let Some(c) = self.currency {
            s += &format!(" currency={}", n(c));
        }
        format!("{s} delimiter={} decimal={}", if self.delim == '\t' { "tab".into() } else { self.delim.to_string() }, self.decimal)
    }

    /// A first guess for CSV: the delimiter most used on the first line, then the first column of dates, the first
    /// other column of amounts, the column of longest text; a comma decimal with `;`.
    pub fn guess(csv: &str) -> Mapping {
        let first = csv.trim_start_matches('\u{feff}').lines().next().unwrap_or_default();
        let delim = [',', ';', '\t', '|'].into_iter().max_by_key(|&d| (first.matches(d).count(), d == ',')).unwrap();
        let decimal = if delim == ';' { ',' } else { '.' };
        let rows: Vec<Vec<String>> = csv_rows(csv, delim).into_iter().take(20).collect();
        let cols = rows.iter().map(Vec::len).max().unwrap_or(0);
        let hits = |c: usize, ok: &dyn Fn(&str) -> bool| rows.iter().filter(|r| r.get(c).is_some_and(|v| ok(v))).count();
        let best = |skip: &[usize], ok: &dyn Fn(&str) -> bool| (0..cols).filter(|c| !skip.contains(c)).map(|c| (hits(c, ok), c)).filter(|x| x.0 > 0).max_by_key(|&(h, c)| (h, std::cmp::Reverse(c))).map(|x| x.1);
        let date = best(&[], &|v| bank_date(v).is_some()).unwrap_or(0);
        let amount = best(&[date], &|v| bank_amount(v, decimal).is_some()).unwrap_or(1.min(cols.saturating_sub(1)));
        let len = |c: usize| rows.iter().filter_map(|r| r.get(c)).map(|v| v.chars().count()).sum::<usize>();
        let text = (0..cols).filter(|c| ![date, amount].contains(c)).max_by_key(|&c| (len(c), std::cmp::Reverse(c))).into_iter().collect();
        Mapping { date, amount, text, currency: None, delim, decimal }
    }
}

/// A bank's date: `2026-10-25`, `2026/10/25`, `20261025`, `25.10.2026`, `25/10/2026` or `25-10-2026` (day first),
/// a time after it ignored.
fn bank_date(s: &str) -> Option<NaiveDate> {
    let s = s.trim().split(['T', ' ']).next()?;
    ["%Y-%m-%d", "%Y/%m/%d", "%Y%m%d", "%d.%m.%Y", "%d/%m/%Y", "%d-%m-%Y"].iter().find_map(|f| NaiveDate::parse_from_str(s, f).ok())
}

/// A bank's amount as signed cents, with DECIMAL as the decimal separator: `-1 234,50`, `1,234.50`, `(12.50)` and
/// `12.50-` are negative; spaces, the other separator, `'` and currency signs are dropped.
fn bank_amount(s: &str, decimal: char) -> Option<i64> {
    let s: String = s.chars().filter(|c| c.is_ascii_digit() || "+-().,".contains(*c)).filter(|&c| c == decimal || !".,".contains(c)).map(|c| if c == decimal { '.' } else { c }).collect();
    let (neg, s) = match s.strip_prefix('(').and_then(|r| r.strip_suffix(')')) {
        Some(r) => (true, r),
        None => (false, s.as_str()),
    };
    let s = match s.strip_suffix('-') {
        Some(r) => format!("-{r}"),
        None => s.into(),
    };
    let c = amount(&s).ok()?;
    Some(if neg { -c.abs() } else { c })
}

/// A bank CSV read with M: the rows that hold a date and an amount as entries (in CURRENCY unless a currency column
/// says otherwise), each flagged true when it is already in KNOWN (ledger and inbox text) by date, amount and text.
/// A known entry matches once, so two equal coffees on a day stay two. The count of other rows comes last.
pub fn read_csv(csv: &str, m: &Mapping, currency: &str, known: &[&str]) -> (Vec<(Entry, bool)>, usize) {
    let mut seen: HashMap<(NaiveDate, i64, String), usize> = HashMap::new();
    for e in known.iter().flat_map(|t| t.lines()).filter_map(parse_entry) {
        *seen.entry((e.date, e.cents, e.text)).or_default() += 1;
    }
    let (mut out, mut skipped) = (vec![], 0);
    for r in csv_rows(csv, m.delim) {
        let cell = |c: usize| r.get(c).map_or("", |s| s.as_str());
        let (Some(date), Some(cents)) = (bank_date(cell(m.date)), bank_amount(cell(m.amount), m.decimal)) else {
            skipped += 1;
            continue;
        };
        let text = m.text.iter().map(|&c| cell(c)).collect::<Vec<_>>().join(" ").split_whitespace().collect::<Vec<_>>().join(" ");
        let cur = m.currency.map(|c| cell(c).trim().to_uppercase()).filter(|c| c.len() == 3 && c.bytes().all(|b| b.is_ascii_uppercase()));
        let e = Entry { date, cents, currency: cur.unwrap_or_else(|| currency.trim().to_uppercase()), text };
        let dupe = seen.get_mut(&(e.date, e.cents, e.text.clone())).filter(|n| **n > 0).map(|n| *n -= 1).is_some();
        out.push((e, dupe));
    }
    (out, skipped)
}

/// The heading title of an import proposal in the inbox.
pub const REVIEW: &str = "Review ledger import";

/// INBOX with each of ES appended as a `* TODO Review ledger import` heading holding the entry line.
pub fn propose(inbox: &str, es: &[Entry], kw: &crate::org::Kw) -> String {
    let todo = kw.todo.first().map_or("TODO", String::as_str);
    let mut t = inbox.to_string();
    if !t.is_empty() && !t.ends_with('\n') {
        t.push('\n');
    }
    for e in es {
        t += &format!("* {todo} {REVIEW}\n{}\n", format_entry(e));
    }
    t
}

/// The entry under the heading at or above LINE of INBOX filed into LEDGER under its month, and that heading's subtree
/// removed: (new inbox, new ledger, the entry line).
pub fn accept(inbox: &str, line: usize, ledger: &str, kw: &crate::org::Kw) -> Result<(String, String, String), String> {
    let mut lines: Vec<String> = inbox.lines().map(String::from).collect();
    let h = (0..=line).rev().find(|&i| lines.get(i).is_some_and(|l| crate::org::heading_level(l).is_some())).ok_or("put the cursor on a ledger import heading")?;
    let (a, b) = crate::org::subtree_range(&lines, h);
    let e = lines[a..b].iter().find_map(|l| parse_entry(l)).ok_or("this heading has no ledger entry line")?;
    lines.drain(a..b);
    let mut t = lines.join("\n");
    if !t.is_empty() && inbox.ends_with('\n') {
        t.push('\n');
    }
    Ok((t, add(ledger, &e, kw), format_entry(&e)))
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

    #[test]
    fn projects_match_by_tag_or_id_link() {
        let kw = kw();
        let f = |p: &str, t: &str| Arc::new(crate::org::parse(std::path::Path::new(p), t, &kw));
        let files = [f("/n/acme.org", ":PROPERTIES:\n:ID: note\n:END:\n* Kickoff\n:PROPERTIES:\n:ID: kick\n:END:\n"), f("/n/x.org", "* Other\n:PROPERTIES:\n:ID: other\n:END:\n")];
        let ids = id_categories(&files);
        assert_eq!((ids["note"].as_str(), ids["kick"].as_str(), ids["other"].as_str()), ("acme", "acme", "x"));
        let e = |l: &str| parse_entry(l).unwrap();
        assert!(belongs(&e("- 2026-10-02 -40 EUR train [[id:kick][Kickoff]]"), "ACME", &ids));
        assert!(!belongs(&e("- 2026-10-05 -1 EUR gone [[id:deleted]]"), "acme", &ids)); // deleted ID: unlinked
        assert!(!belongs(&e("- 2026-10-05 -1 EUR #acmeish"), "acme", &ids));
        assert!(!belongs(&e("- 2026-10-05 -1 EUR no link or tag"), "acme", &ids));
        let text = "* 2026-10\n- 2026-10-01 +900 EUR invoice #Acme\n- 2026-10-02 -40 EUR train [[id:kick][Kickoff]]\n- 2026-10-03 -300 EUR hotel [[id:note]]\n- 2026-10-04 -5 SEK fika #acme\n- 2026-10-05 -1 EUR gone [[id:deleted]]\n- 2026-10-06 -2 EUR other [[id:other]] #food\n";
        let t = project_totals(text, &["Acme".into(), "x".into(), "none".into()], &ids);
        assert_eq!(t.keys().collect::<Vec<_>>(), ["Acme", "x"]);
        assert_eq!(t["Acme"].iter().map(|t| (t.currency.as_str(), t.income, t.expense, t.net)).collect::<Vec<_>>(), [("EUR", 90000, -34000, 56000), ("SEK", 0, -500, -500)]);
        assert_eq!((t["x"].len(), t["x"][0].expense), (1, -200));
    }

    #[test]
    fn payments_are_the_payee_s_entries() {
        let text = "- 2026-10-05 -7 EUR dinner @Anna\n- 2026-10-01 -12.50 EUR lunch @anna #food\n- 2026-10-02 -1 EUR @annabel\nnote @anna\n";
        assert_eq!(payments(text, "@ANNA").iter().map(|r| (r.line, r.cents)).collect::<Vec<_>>(), [(1, -1250), (0, -700)]);
        assert_eq!(payments(text, " @ ").len(), 0);
    }

    #[test]
    fn csv_rows_handle_quotes_bom_and_delimiters() {
        let r = csv_rows("\u{feff}a,\"b, c\",\"say \"\"hi\"\"\"\r\n\r\n\"two\nlines\",,x\nlast", ',');
        assert_eq!(r, [vec!["a", "b, c", "say \"hi\""], vec!["two\nlines", "", "x"], vec!["last"]]);
        assert_eq!(csv_rows("1;2,5;x\n", ';'), [vec!["1", "2,5", "x"]]);
        assert_eq!(csv_rows("", ','), Vec::<Vec<String>>::new());
    }

    #[test]
    fn bank_amounts_and_dates() {
        let a = |s, d| bank_amount(s, d);
        let got = [a("-12,50", ','), a("1 234,50", ','), a("1.234,50", ','), a("(12.50)", '.'), a("1,234.50", '.'), a("12.50-", '.'), a("€ 3", '.'), a("+7.5", '.')];
        assert_eq!(got, [-1250, 123450, 123450, -1250, 123450, -1250, 300, 750].map(Some));
        for bad in ["", "Amount", "0", "1.234", "2026-10-01"] {
            assert_eq!(a(bad, '.'), None, "{bad}");
        }
        for s in ["2026-10-25", "2026/10/25", "20261025", "25.10.2026", "25/10/2026", "2026-10-25 13:00", "2026-10-25T13:00:00"] {
            assert_eq!(bank_date(s), Some(d("2026-10-25")), "{s}");
        }
        assert_eq!(bank_date("Date"), None);
    }

    #[test]
    fn mapping_spec_round_trips_and_guesses() {
        let m = Mapping::parse("date=1 amount=3 text=2+4 currency=5 delimiter=; decimal=,").unwrap();
        assert_eq!((m.date, m.amount, &m.text, m.currency, m.delim, m.decimal), (0, 2, &vec![1, 3], Some(4), ';', ','));
        assert_eq!(Mapping::parse(&m.spec()), Ok(m));
        assert_eq!(Mapping::parse("amount=2 delimiter=tab").unwrap_err(), "say which columns hold the date and the amount, e.g. date=1 amount=3");
        assert!(Mapping::parse("date=0 amount=1").is_err() && Mapping::parse("date=1 amount=2 x=3").is_err() && Mapping::parse("date=1 amount=2 decimal=;").is_err());
        let g = Mapping::guess("\u{feff}Bokföringsdag;Text;Belopp;Saldo\n2026-10-03;ICA Kvantum Malmö;-412,50;1 000,00\n2026-10-04;Lön;25 000,00;26 000,00\n");
        assert_eq!(g.spec(), "date=1 amount=3 text=2 delimiter=; decimal=,");
        assert_eq!(Mapping::guess("Description,Date,Amount\nCoffee,10/25/2026,x\nTea shop,25/10/2026,-3.50\n").spec(), "date=2 amount=3 text=1 delimiter=, decimal=.");
    }

    #[test]
    fn read_csv_skips_duplicates_in_ledger_and_inbox() {
        let m = Mapping::parse("date=1 amount=3 text=2 delimiter=; decimal=,").unwrap();
        let csv = "Datum;Text;Belopp\n2026-10-03;ICA  Kvantum;-412,50\n2026-10-04;Coffee;-3\n2026-10-04;Coffee;-3\n2026-10-05;\"Rent; Oct\";(9 000,00)\n2026-10-06;Inbox one;+5\nbad;row;x\n";
        let ledger = "* 2026-10\n- 2026-10-04 -3.00 EUR Coffee\n";
        let inbox = "* TODO Review ledger import\n- 2026-10-06 +5.00 EUR Inbox one\n";
        let (es, skipped) = read_csv(csv, &m, "eur", &[ledger, inbox]);
        let got: Vec<_> = es.iter().map(|(e, d)| (format_entry(e), *d)).collect();
        assert_eq!(got, [
            ("- 2026-10-03 -412.50 EUR ICA Kvantum".to_string(), false),
            ("- 2026-10-04 -3.00 EUR Coffee".into(), true),
            ("- 2026-10-04 -3.00 EUR Coffee".into(), false), // the second coffee of the day is new
            ("- 2026-10-05 -9000.00 EUR Rent; Oct".into(), false),
            ("- 2026-10-06 +5.00 EUR Inbox one".into(), true),
        ]);
        assert_eq!(skipped, 2); // the header and the bad row
    }

    #[test]
    fn export_round_trips_through_import() {
        let text = "* 2026-10\n- 2026-10-25 -12.50 EUR lunch, \"good\" @anna #food [[id:abc]]\n- 2026-10-01 +1200 SEK pay #work\n- 2026-09-30 -1 EUR before\n- 2026-11-01 -1 EUR after\n";
        let csv = to_csv(text, d("2026-10-01"), d("2026-10-31"));
        assert_eq!(csv, "date,amount,currency,category,payee,link,text\n2026-10-01,+1200.00,SEK,work,,,pay #work\n2026-10-25,-12.50,EUR,food,anna,id:abc,\"lunch, \"\"good\"\" @anna #food [[id:abc]]\"\n");
        let m = Mapping::parse("date=1 amount=2 text=7 currency=3").unwrap();
        let back: Vec<Entry> = read_csv(&csv, &m, "USD", &[]).0.into_iter().map(|x| x.0).collect();
        let mut want: Vec<Entry> = text.lines().filter_map(parse_entry).filter(|e| e.date.format("%Y-%m").to_string() == "2026-10").collect();
        want.reverse();
        assert_eq!(back, want);
    }

    #[test]
    fn proposals_go_to_the_inbox_and_accept_moves_one() {
        let e = |s| parse_quick(s, d("2026-10-25"), "EUR").unwrap();
        let inbox = propose("* TODO Call\n", &[e("12.50 lunch"), e("+5 refund")], &kw());
        assert_eq!(inbox, "* TODO Call\n* TODO Review ledger import\n- 2026-10-25 -12.50 EUR lunch\n* TODO Review ledger import\n- 2026-10-25 +5.00 EUR refund\n");
        let (rest, ledger, line) = accept(&inbox, 2, "#+title: ledger\n", &kw()).unwrap();
        assert_eq!(rest, "* TODO Call\n* TODO Review ledger import\n- 2026-10-25 +5.00 EUR refund\n");
        assert_eq!((ledger.as_str(), line.as_str()), ("#+title: ledger\n* 2026-10\n- 2026-10-25 -12.50 EUR lunch\n", "- 2026-10-25 -12.50 EUR lunch"));
        assert_eq!(accept(&inbox, 0, "", &kw()).unwrap_err(), "this heading has no ledger entry line");
        assert!(accept("no heading\n", 0, "", &kw()).is_err());
        assert_eq!(propose("", &[], &kw()), "");
    }
}
