//! Ledger entries: plain list lines in `ledger.org` (notes dir), filed under `* YYYY-MM` headings:
//! `- 2026-10-25 -12.50 EUR lunch @anna #food [[id:…]]` (negative = expense, positive = income).
//! Dates stay unbracketed so entries never reach the agenda. Lines are only ever appended;
//! anything `parse_entry` doesn't understand is left as it is.
use chrono::{Datelike, NaiveDate};
use regex::Regex;
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, LazyLock};

static ENTRY: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s*-\s+(\d{4}-\d{2}-\d{2})\s+(\S+)\s+([A-Z]{3})(?:\s+(.*?))?\s*$").unwrap());
static WORD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?:^|\s)([#@])([\w][\w.-]*)").unwrap());
static WIKI: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\[\[.*?\]\]").unwrap());
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

/// Text of E without `@payee`, `#tag` words and `[[links]]`, lowercased: what a repeat purchase has in common.
fn plain(text: &str) -> String {
    let t = WIKI.replace_all(text, "");
    t.split_whitespace().filter(|w| !w.starts_with(['#', '@'])).collect::<Vec<_>>().join(" ").to_lowercase()
}

/// Whether W (without a sigil) is only digits, ignoring a trailing `.` or `-`: a card or reference number, not a name.
fn numeric(w: &str) -> bool {
    let w = w.trim_end_matches(['.', '-']);
    !w.is_empty() && w.bytes().all(|b| b.is_ascii_digit())
}

/// `plain` without number-only words, so a bank row's `KORT 1234` matches a ledger line's `KORT #1234` (#1217).
fn key(text: &str) -> String {
    plain(text).split_whitespace().filter(|w| !numeric(w)).collect::<Vec<_>>().join(" ")
}

/// Whether TEXT has a literal `#-` word: the user opted out of a category.
fn opted_out(text: &str) -> bool {
    text.split_whitespace().any(|w| w == "#-")
}

/// Give E the category of the latest entry in TEXT (the ledger file) with the same `@payee`, else the same plain text,
/// unless E has a `#category` or `#-` itself (#1205).
pub fn autocategory(text: &str, e: &mut Entry) {
    if e.category().is_some() || opted_out(&e.text) {
        return;
    }
    let (payee, plain_e) = (e.payee().map(str::to_lowercase), plain(&e.text));
    if payee.is_none() && plain_e.is_empty() {
        return;
    }
    let found = text.lines().rev().filter_map(parse_entry).filter(|o| !opted_out(&o.text)).find_map(|o| {
        let same = match &payee {
            Some(p) => o.payee().is_some_and(|q| q.to_lowercase() == *p),
            None => plain(&o.text) == plain_e,
        };
        o.category().filter(|_| same).map(str::to_string)
    });
    if let Some(c) = found {
        e.text = format!("{} #{c}", e.text).trim().into();
    }
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

/// A category's monthly budget and what the month spent of it (cents, in `currency`).
#[derive(Serialize, Debug, PartialEq)]
pub struct Budget {
    pub category: String,
    pub currency: String,
    pub budget: i64,
    pub used: i64,
}

/// SPECS (`food=400`, `food=399.50`) against the month's TOTALS: the default CURRENCY's net spending per category,
/// summed case-insensitively (refunds count back). Bad entries and budgets of 0 or less are skipped.
pub fn budgets(specs: &[String], totals: &[Total], currency: &str) -> Vec<Budget> {
    let currency = currency.trim().to_uppercase();
    let t = totals.iter().find(|t| t.currency == currency);
    specs
        .iter()
        .filter_map(|s| {
            let (name, amt) = s.split_once('=')?;
            let (name, budget) = (name.trim().trim_start_matches('#'), amount(amt.trim()).ok().filter(|b| *b > 0)?);
            let used = -t.map_or(0, |t| t.categories.iter().filter(|(c, _)| c.as_deref().is_some_and(|c| c.eq_ignore_ascii_case(name))).map(|(_, n)| n).sum::<i64>());
            Some(Budget { category: name.into(), currency: currency.clone(), budget, used })
        })
        .collect()
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

/// One currency's week for the weekly review (#1197), in cents: SPENT and LAST (the week before) are expenses as positive
/// sums, INCOME this week's income, TOP the `#category` with the most net spending this week (positive).
#[derive(Serialize, Debug, PartialEq)]
pub struct Week {
    pub currency: String,
    pub spent: i64,
    pub last: i64,
    pub income: i64,
    pub top: Option<(String, i64)>,
}

/// TEXT's week of 7 days from FROM against the 7 days before, per currency with entries in either (CURRENCY first, then by code).
pub fn week_summary(text: &str, from: NaiveDate, currency: &str) -> Vec<Week> {
    let (prev, to) = (from - chrono::Days::new(7), from + chrono::Days::new(6));
    let rows = rows(text, |e| e.date >= prev && e.date <= to);
    type Sums<'a> = (Week, BTreeMap<&'a str, i64>);
    let mut by: BTreeMap<(bool, &str), Sums> = BTreeMap::new();
    for r in &rows {
        let (w, cats) = by.entry((r.currency != currency.trim().to_uppercase(), &r.currency)).or_insert_with(|| (Week { currency: r.currency.clone(), spent: 0, last: 0, income: 0, top: None }, BTreeMap::new()));
        match (r.date < from, r.cents < 0) {
            (true, true) => w.last -= r.cents,
            (true, false) => {}
            (false, true) => w.spent -= r.cents,
            (false, false) => w.income += r.cents,
        }
        if let (false, Some(c)) = (r.date < from, &r.category) {
            *cats.entry(c).or_default() -= r.cents;
        }
    }
    by.into_values()
        .map(|(mut w, cats)| {
            // Most spent first; ties go to the first name (max_by_key keeps the last, so iterate reversed).
            w.top = cats.into_iter().rev().filter(|(_, n)| *n > 0).max_by_key(|(_, n)| *n).map(|(c, n)| (c.to_string(), n));
            w
        })
        .collect()
}

/// TEXT's calendar MONTH (`YYYY-MM`) against the month before, as `Week`s for the monthly review (#1220), from two `summarize` calls.
pub fn month_summary(text: &str, month: NaiveDate, currency: &str) -> Vec<Week> {
    let ym = |d: NaiveDate| d.format("%Y-%m").to_string();
    let (now, prev) = (summarize(text, &ym(month), currency).0, summarize(text, &ym(month - chrono::Months::new(1)), currency).0);
    let mut by: BTreeMap<(bool, String), Week> = BTreeMap::new();
    for (t, this) in prev.into_iter().map(|t| (t, false)).chain(now.into_iter().map(|t| (t, true))) {
        let w = by.entry((t.currency != currency.trim().to_uppercase(), t.currency.clone())).or_insert_with(|| Week { currency: t.currency.clone(), spent: 0, last: 0, income: 0, top: None });
        if !this {
            w.last = -t.expense;
            continue;
        }
        (w.spent, w.income) = (-t.expense, t.income);
        // Categories come largest first (ties by name), so the first net spending one is the top.
        w.top = t.categories.into_iter().find_map(|(c, n)| c.filter(|_| n < 0).map(|c| (c, -n)));
    }
    by.into_values().collect()
}

/// One currency's year (#1196), in cents per month (index 0 = January): INCOME, EXPENSE (negative) and each
/// category's net (None = uncategorised), largest yearly amount first.
#[derive(Serialize, Debug, PartialEq)]
pub struct Year {
    pub currency: String,
    pub income: [i64; 12],
    pub expense: [i64; 12],
    pub categories: Vec<(Option<String>, [i64; 12])>,
}

/// TEXT's entries dated in YEAR (`YYYY`) per currency, CURRENCY first, then by code (never converted), like `summarize`.
pub fn summarize_year(text: &str, year: &str, currency: &str) -> Vec<Year> {
    let rows = rows(text, |e| e.date.format("%Y").to_string() == year);
    type Sums = ([i64; 12], [i64; 12], BTreeMap<Option<String>, [i64; 12]>);
    let mut by: BTreeMap<(bool, &str), Sums> = BTreeMap::new();
    for r in &rows {
        let (t, m) = (by.entry((r.currency != currency.trim().to_uppercase(), &r.currency)).or_default(), r.date.month0() as usize);
        if r.cents > 0 { t.0[m] += r.cents } else { t.1[m] += r.cents }
        t.2.entry(r.category.clone()).or_default()[m] += r.cents;
    }
    by.into_iter()
        .map(|((_, cur), (income, expense, c))| {
            let mut categories: Vec<_> = c.into_iter().collect();
            categories.sort_by_key(|(_, m)| std::cmp::Reverse(m.iter().sum::<i64>().unsigned_abs())); // stable: ties stay by name
            Year { currency: cur.into(), income, expense, categories }
        })
        .collect()
}

/// TEXT's entries that KEEP accepts, oldest first.
pub fn rows(text: &str, keep: impl Fn(&Entry) -> bool) -> Vec<Row> {
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

/// (Reference for `project_totals`, which applies it per entry via an index.) Whether E belongs to PROJECT, the rule the project cards use for tasks: its `#category` is the project's name (any
/// case), or its `[[id:…]]` link points at a note or heading (IDS) in that category. An unknown ID counts as unlinked.
#[cfg(test)]
pub fn belongs(e: &Entry, project: &str, ids: &HashMap<String, String>) -> bool {
    let is = |c: &str| c.eq_ignore_ascii_case(project);
    e.category().is_some_and(is) || e.link().and_then(|l| ids.get(l.strip_prefix("id:")?.split("::").next()?.trim())).is_some_and(|c| is(c))
}

/// All-time income and expense of each of NAMES in TEXT, per currency (by code; only income, expense and net set).
/// Projects without entries are left out.
pub fn project_totals(text: &str, names: &[String], ids: &HashMap<String, String>) -> BTreeMap<String, Vec<Total>> {
    // Index names by lowercased (ASCII, like `belongs`) text once; each entry then looks up its category and linked category.
    let mut by_name: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, n) in names.iter().enumerate() {
        by_name.entry(n.to_ascii_lowercase()).or_default().push(i);
    }
    let mut sums: Vec<BTreeMap<String, Total>> = names.iter().map(|_| BTreeMap::new()).collect();
    for e in text.lines().filter_map(parse_entry) {
        let linked = e.link().and_then(|l| ids.get(l.strip_prefix("id:")?.split("::").next()?.trim()));
        let hits = |c: Option<&str>| c.and_then(|c| by_name.get(&c.to_ascii_lowercase())).into_iter().flatten().copied();
        let mut to: Vec<usize> = hits(e.category()).collect();
        for i in hits(linked.map(String::as_str)) {
            if !to.contains(&i) {
                to.push(i);
            }
        }
        for i in to {
            let t = sums[i].entry(e.currency.clone()).or_insert_with(|| Total { currency: e.currency.clone(), ..Default::default() });
            if e.cents > 0 { t.income += e.cents } else { t.expense += e.cents }
            t.net += e.cents;
        }
    }
    let mut out = BTreeMap::new();
    for (n, by) in names.iter().zip(sums) {
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
/// `date=1 amount=3 text=2+4 currency=5 delimiter=; decimal=,` (currency is optional; `delimiter=tab`; `dates=mdy`
/// or `ymd` for dates that aren't day first).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum DateOrder {
    #[default]
    Dmy,
    Mdy,
    Ymd,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Mapping {
    pub date: usize,
    pub amount: usize,
    /// The money-in column when the bank splits debit (`amount`) and credit: `amount=3-4`.
    pub amount2: Option<usize>,
    pub text: Vec<usize>,
    pub currency: Option<usize>,
    pub delim: char,
    pub decimal: char,
    pub dates: DateOrder,
}

impl Mapping {
    pub fn parse(spec: &str) -> Result<Mapping, String> {
        let mut m = Mapping { date: usize::MAX, amount: usize::MAX, amount2: None, text: vec![], currency: None, delim: ',', decimal: '.', dates: DateOrder::Dmy };
        let col = |v: &str| v.trim().parse::<usize>().ok().filter(|&n| n > 0).map(|n| n - 1).ok_or_else(|| format!("{v:?} isn't a column number (1, 2, …)"));
        for w in spec.split_whitespace() {
            let (k, v) = w.split_once('=').ok_or_else(|| format!("write {w:?} as key=value"))?;
            match k {
                "date" => m.date = col(v)?,
                "amount" => match v.split_once('-') {
                    Some((a, b)) => (m.amount, m.amount2) = (col(a)?, Some(col(b)?)),
                    None => (m.amount, m.amount2) = (col(v)?, None),
                },
                "text" => m.text = v.split('+').map(col).collect::<Result<_, _>>()?,
                "currency" => m.currency = Some(col(v)?),
                "delimiter" => m.delim = match v { "tab" => '\t', "," | ";" | "|" => v.chars().next().unwrap(), _ => return Err(format!("the delimiter {v:?} isn't , ; | or tab")) },
                "decimal" => m.decimal = match v { "." | "," => v.chars().next().unwrap(), _ => return Err(format!("the decimal separator {v:?} isn't . or ,")) },
                "dates" => m.dates = match v { "dmy" => DateOrder::Dmy, "mdy" => DateOrder::Mdy, "ymd" => DateOrder::Ymd, _ => return Err(format!("the date order {v:?} isn't dmy, mdy or ymd")) },
                _ => return Err(format!("unknown key {k:?}: use date, amount, text, currency, delimiter, decimal or dates")),
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
        let mut s = format!("date={} amount={}", n(self.date), n(self.amount) + &self.amount2.map_or(String::new(), |c| format!("-{}", n(c))));
        if !text.is_empty() {
            s += &format!(" text={text}");
        }
        if let Some(c) = self.currency {
            s += &format!(" currency={}", n(c));
        }
        s += &format!(" delimiter={} decimal={}", if self.delim == '\t' { "tab".into() } else { self.delim.to_string() }, self.decimal);
        match self.dates {
            DateOrder::Dmy => s,
            DateOrder::Mdy => s + " dates=mdy",
            DateOrder::Ymd => s + " dates=ymd",
        }
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
        let date = best(&[], &|v| bank_date(v, DateOrder::Dmy).or_else(|| bank_date(v, DateOrder::Mdy)).is_some()).unwrap_or(0);
        // month first when some second field is above 12 and no first field is
        let field = |r: &Vec<String>, i: usize| r.get(date).and_then(|v| v.trim().split(|c: char| !c.is_ascii_digit()).nth(i)?.parse::<u32>().ok()).unwrap_or(0);
        let dates = if rows.iter().all(|r| field(r, 0) <= 12) && rows.iter().any(|r| field(r, 1) > 12) { DateOrder::Mdy } else { DateOrder::Dmy };
        let amount = best(&[date], &|v| bank_amount(v, decimal).is_some()).unwrap_or(1.min(cols.saturating_sub(1)));
        let len = |c: usize| rows.iter().filter_map(|r| r.get(c)).map(|v| v.chars().count()).sum::<usize>();
        let text = (0..cols).filter(|c| ![date, amount].contains(c)).max_by_key(|&c| (len(c), std::cmp::Reverse(c))).into_iter().collect();
        Mapping { date, amount, amount2: None, text, currency: None, delim, decimal, dates }
    }
}

/// A bank's date: `2026-10-25`, `2026/10/25`, `20261025`, or with `. / -` separators `25.10.2026` (day first for
/// ORDER dmy, month first for mdy, year first for ymd), the year also as two digits; a time after it ignored.
fn bank_date(s: &str, order: DateOrder) -> Option<NaiveDate> {
    let s = s.trim().split(['T', ' ']).next()?;
    let mut fs = vec!["%Y-%m-%d".to_string(), "%Y/%m/%d".into(), "%Y%m%d".into()];
    for sep in ['.', '/', '-'] {
        // `%y` before `%Y`: chrono's `%Y` would take `26` as the year 26
        for y in ["y", "Y"] {
            fs.push(match order {
                DateOrder::Dmy => format!("%d{sep}%m{sep}%{y}"),
                DateOrder::Mdy => format!("%m{sep}%d{sep}%{y}"),
                DateOrder::Ymd => format!("%{y}{sep}%m{sep}%d"),
            });
        }
    }
    fs.iter().find_map(|f| NaiveDate::parse_from_str(s, f).ok().filter(|d| d.year() >= 1000))
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

/// The CSV's header line, normalized (BOM-stripped, trimmed, lowercased): the key a bank's column mapping is remembered by.
pub fn header_key(csv: &str) -> String {
    csv.trim_start_matches('\u{feff}').lines().next().unwrap_or_default().trim().to_lowercase()
}

/// The mapping spec remembered for CSV's header in the JSON store at STORE (missing or corrupt: none).
pub fn remembered(store: &std::path::Path, csv: &str) -> Option<String> {
    let map: HashMap<String, String> = serde_json::from_str(&std::fs::read_to_string(store).ok()?).ok()?;
    map.get(&header_key(csv)).cloned()
}

/// Remember SPEC for CSV's header in STORE, unless the first line is a transaction (a header-less export) or empty.
pub fn remember(store: &std::path::Path, csv: &str, spec: &str) -> Result<(), String> {
    let key = header_key(csv);
    let m = Mapping::parse(spec)?;
    if key.is_empty() || !read_csv(csv.lines().next().unwrap_or_default(), &m, "EUR", &[]).0.is_empty() {
        return Ok(());
    }
    let mut map: HashMap<String, String> = std::fs::read_to_string(store).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
    map.insert(key, spec.to_string());
    if let Some(d) = store.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    crate::config::write_atomic(store, serde_json::to_string(&map).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

/// A bank CSV read with M: the rows that hold a date and an amount as entries (in CURRENCY unless a currency column
/// says otherwise), each flagged true when it is already in KNOWN (ledger and inbox text) by date, amount and text (ignoring tags, payee and links).
/// A known entry matches once, so two equal coffees on a day stay two. The count of other rows comes last.
pub fn read_csv(csv: &str, m: &Mapping, currency: &str, known: &[&str]) -> (Vec<(Entry, bool)>, usize) {
    let mut seen: HashMap<(NaiveDate, i64, String), usize> = HashMap::new();
    for e in known.iter().flat_map(|t| t.lines()).filter_map(parse_entry) {
        *seen.entry((e.date, e.cents, key(&e.text))).or_default() += 1;
    }
    let (mut out, mut skipped) = (vec![], 0);
    for r in csv_rows(csv, m.delim) {
        let cell = |c: usize| r.get(c).map_or("", |s| s.as_str());
        let cents = match m.amount2 {
            // debit and credit columns: money in minus money out, each unsigned; a net of zero isn't an entry
            Some(c2) => {
                let side = |c: usize| bank_amount(cell(c), m.decimal).map(i64::abs);
                let (out, inc) = (side(m.amount), side(c2));
                (out.is_some() || inc.is_some()).then(|| inc.unwrap_or(0) - out.unwrap_or(0)).filter(|&c| c != 0)
            }
            None => bank_amount(cell(m.amount), m.decimal),
        };
        let (Some(date), Some(cents)) = (bank_date(cell(m.date), m.dates), cents) else {
            skipped += 1;
            continue;
        };
        let text = m.text.iter().map(|&c| cell(c)).collect::<Vec<_>>().join(" ");
        // `#1234` in bank text is a number, not a category: drop the sigil so it can't pre-empt the ledger's category
        let text = text.split_whitespace().map(|w| w.strip_prefix(['#', '@']).filter(|r| numeric(r)).unwrap_or(w)).collect::<Vec<_>>().join(" ");
        let cur = m.currency.map(|c| cell(c).trim().to_uppercase()).filter(|c| c.len() == 3 && c.bytes().all(|b| b.is_ascii_uppercase()));
        let e = Entry { date, cents, currency: cur.unwrap_or_else(|| currency.trim().to_uppercase()), text };
        let dupe = seen.get_mut(&(e.date, e.cents, key(&e.text))).filter(|n| **n > 0).map(|n| *n -= 1).is_some();
        out.push((e, dupe));
    }
    (out, skipped)
}

/// Give the new rows of ES (from `read_csv`) the category of the latest ledger entry in LEDGER with the same plain text
/// (as `autocategory`, #1211). Duplicates are left alone and rows don't feed each other.
pub fn categorize_import(ledger: &str, es: &mut [(Entry, bool)]) {
    let mut by: HashMap<String, String> = HashMap::new();
    for o in ledger.lines().filter_map(parse_entry).filter(|o| !opted_out(&o.text)) {
        let p = key(&o.text);
        if let (Some(c), false) = (o.category(), p.is_empty()) {
            by.insert(p, c.to_string());
        }
    }
    for (e, _) in es.iter_mut().filter(|x| !x.1) {
        if e.category().is_none() && !opted_out(&e.text) {
            if let Some(c) = by.get(&key(&e.text)) {
                e.text = format!("{} #{c}", e.text);
            }
        }
    }
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

/// Every open `REVIEW` proposal of INBOX that holds an entry line filed into LEDGER: (new inbox, new ledger, count).
pub fn accept_all(inbox: &str, ledger: &str, kw: &crate::org::Kw) -> Result<(String, String, usize), String> {
    let (mut inbox, mut ledger, mut n) = (inbox.to_string(), ledger.to_string(), 0);
    // Re-scan after each filing (line numbers shift); a filed heading is gone, so the loop ends.
    while let Some(h) = inbox.lines().enumerate().position(|(i, l)| {
        crate::org::heading_level(l) == Some(1)
            && crate::org::heading(&inbox, i, kw).is_some_and(|p| p.title == REVIEW && p.keyword.as_ref().is_some_and(|k| !kw.done.contains(k)))
            && inbox.lines().skip(i + 1).take_while(|l| crate::org::heading_level(l).is_none()).any(|l| parse_entry(l).is_some())
    }) {
        (inbox, ledger, _) = accept(&inbox, h, &ledger, kw)?;
        n += 1;
    }
    Ok((inbox, ledger, n))
}

// ---------------------------------------------------------------- recurring (#1182)

/// A recurring RULE `START +N[dwmy] amount [CUR] text…` (the quick-add body: unsigned is an expense): its first entry and step.
pub fn parse_rule(rule: &str, currency: &str) -> Result<(Entry, u32, char), String> {
    let bad = || format!("can't read the recurring entry {rule:?}: write it like 2026-11-01 +1m 1200 rent @landlord");
    let mut w = rule.split_whitespace();
    let start: NaiveDate = w.next().and_then(|d| d.parse().ok()).ok_or_else(bad)?;
    let step = w.next().and_then(|s| s.strip_prefix('+')).ok_or_else(bad)?;
    let unit = step.chars().last().filter(|c| "dwmy".contains(*c)).ok_or_else(bad)?;
    let n = step[..step.len() - 1].parse::<u32>().ok().filter(|&n| n > 0).ok_or_else(bad)?;
    let body = w.collect::<Vec<_>>().join(" ");
    if body.split_whitespace().next().is_some_and(|d| d.parse::<NaiveDate>().is_ok()) {
        return Err(bad());
    }
    let e = parse_quick(&body, start, currency).map_err(|e| format!("recurring entry {rule:?}: {e}"))?;
    Ok((e, n, unit))
}

/// The K-th date of a rule from START every N UNITs, always counted from START (so the 31st stays the month's last day).
fn nth(start: NaiveDate, n: u32, unit: char, k: u32) -> Option<NaiveDate> {
    let steps = k.checked_mul(n)?;
    match unit {
        'd' => start.checked_add_days(chrono::Days::new(steps.into())),
        'w' => start.checked_add_days(chrono::Days::new(u64::from(steps) * 7)),
        'm' => start.checked_add_months(chrono::Months::new(steps)),
        _ => start.checked_add_months(chrono::Months::new(steps.checked_mul(12)?)),
    }
}

/// Recurring entries due now: ENTRIES oldest first, the new watermarks MARKS (rule text → done through; current rules only)
/// and one error per rule that can't be read.
#[derive(Debug, Default, PartialEq)]
pub struct Due {
    pub entries: Vec<Entry>,
    pub marks: BTreeMap<String, NaiveDate>,
    pub errors: Vec<String>,
}

/// RULES' occurrences up to TODAY after each rule's watermark in MARKS (keyed by the exact rule text, so a new or edited
/// rule only fills in TODAY's month), less those already in KNOWN (ledger and inbox text) by date, amount and payee (else text).
pub fn recurring(rules: &[String], marks: &BTreeMap<String, NaiveDate>, known: &[&str], today: NaiveDate, currency: &str) -> Due {
    let key = |e: &Entry| (e.date, e.cents, e.payee().map_or_else(|| e.text.clone(), str::to_lowercase));
    let mut seen: std::collections::HashSet<_> = known.iter().flat_map(|t| t.lines()).filter_map(parse_entry).map(|e| key(&e)).collect();
    let mut due = Due::default();
    let month = today.with_day(1).and_then(|d| d.pred_opt()).unwrap_or(today);
    for r in rules.iter().map(|r| r.trim()).filter(|r| !r.is_empty()) {
        let (e, n, unit) = match parse_rule(r, currency) {
            Ok(x) => x,
            Err(err) => {
                due.errors.push(err);
                continue;
            }
        };
        let after = marks.get(r).copied().unwrap_or(month);
        for d in (0..).map_while(|k| nth(e.date, n, unit, k).filter(|d| *d <= today)).filter(|d| *d > after) {
            let x = Entry { date: d, ..e.clone() };
            if seen.insert(key(&x)) {
                due.entries.push(x);
            }
        }
        due.marks.insert(r.into(), after.max(today));
    }
    due.entries.sort_by_key(|e| e.date);
    due
}

/// The recurring rules' watermarks in the JSON store at STORE (missing or corrupt: none).
pub fn load_marks(store: &std::path::Path) -> BTreeMap<String, NaiveDate> {
    std::fs::read_to_string(store).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

pub fn save_marks(store: &std::path::Path, marks: &BTreeMap<String, NaiveDate>) -> Result<(), String> {
    if let Some(d) = store.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    crate::config::write_atomic(store, serde_json::to_string(marks).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
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
    fn autocategory_copies_the_latest_match_and_respects_explicit_choices() {
        let ledger = "* 2026-10\n- 2026-10-01 -5.00 EUR coffee @anna #old\n- 2026-10-02 -9.00 EUR coffee [[id:x][Y z]] @anna #cafe\n- 2026-10-03 -3.00 EUR bus #transit\n- 2026-10-04 -3.00 EUR bus @anna #-\n";
        let fill = |t: &str| { let mut e = parse_quick(t, d("2026-11-01"), "EUR").unwrap(); autocategory(ledger, &mut e); e.text };
        assert_eq!(fill("4 beans @Anna"), "beans @Anna #cafe"); // payee, case-insensitive, latest; the #- entry is skipped
        assert_eq!(fill("3 Bus"), "Bus #transit"); // text
        assert_eq!(fill("4 beans @anna #gift"), "beans @anna #gift");
        assert_eq!(fill("4 beans @anna #-"), "beans @anna #-");
        assert_eq!(fill("4 tea @bob"), "tea @bob");
        assert_eq!(fill("4"), "");
    }

    #[test]
    fn recurring_rules_parse_and_clamp_month_ends() {
        let (e, n, u) = parse_rule("2026-01-31  +1m 1200 rent @landlord #housing", "eur").unwrap();
        assert_eq!((format_entry(&e).as_str(), n, u), ("- 2026-01-31 -1200.00 EUR rent @landlord #housing", 1, 'm'));
        let dates = |r: &str, k: u32| { let (e, n, u) = parse_rule(r, "EUR").unwrap(); (0..k).map(|k| nth(e.date, n, u, k).unwrap().to_string()).collect::<Vec<_>>() };
        // Always counted from the start: the 31st comes back after a short month.
        assert_eq!(dates("2026-01-31 +1m 1 x", 4), ["2026-01-31", "2026-02-28", "2026-03-31", "2026-04-30"]);
        assert_eq!(dates("2024-02-29 +1y +5 USD gift", 5), ["2024-02-29", "2025-02-28", "2026-02-28", "2027-02-28", "2028-02-29"]);
        assert_eq!(dates("2026-11-01 +2w 9 gym", 3), ["2026-11-01", "2026-11-15", "2026-11-29"]);
        assert_eq!(dates("2026-11-01 +3d 9 x", 2), ["2026-11-01", "2026-11-04"]);
        for bad in ["", "rent", "2026-11-01 1200 rent", "2026-11-01 +0m 1 x", "2026-11-01 +1q 1 x", "2026-11-01 +m 1 x", "2026-11-01 +1é 1 x", "2026-11-01 +1m", "2026-11-01 +1m lunch", "2026-11-01 +1m 2026-11-02 5 x"] {
            assert!(parse_rule(bad, "EUR").is_err(), "{bad}");
        }
    }

    #[test]
    fn recurring_fills_from_the_watermark_and_never_resurrects() {
        let rules: Vec<String> = vec!["2026-01-31 +1m 1200 rent @landlord".into(), "bad rule".into(), " ".into()];
        let (today, none) = (d("2026-11-08"), BTreeMap::new());
        let lines = |due: &Due| due.entries.iter().map(format_entry).collect::<Vec<_>>();
        // First sight: only today's month (not January to October), up to today.
        let due = recurring(&rules, &none, &[], today, "EUR");
        assert_eq!(lines(&due), Vec::<String>::new()); // the 30th hasn't come yet
        assert_eq!((due.marks.len(), due.errors.len()), (1, 1));
        let due = recurring(&rules, &none, &[], d("2026-11-30"), "EUR");
        assert_eq!(lines(&due), ["- 2026-11-30 -1200.00 EUR rent @landlord"]);
        // After the watermark only: deleting or editing the November entry never brings it back; months missed come in.
        let marks = due.marks;
        assert_eq!(marks["2026-01-31 +1m 1200 rent @landlord"], d("2026-11-30"));
        assert!(recurring(&rules, &marks, &[], d("2026-12-01"), "EUR").entries.is_empty());
        let due = recurring(&rules, &marks, &[], d("2027-02-28"), "EUR");
        assert_eq!(lines(&due), ["- 2026-12-31 -1200.00 EUR rent @landlord", "- 2027-01-31 -1200.00 EUR rent @landlord", "- 2027-02-28 -1200.00 EUR rent @landlord"]);
        // Already in the ledger (another machine) or proposed in the inbox, by date, amount and payee: not again.
        let ledger = "* 2026-12\n- 2026-12-31 -1200.00 EUR December rent @Landlord\n";
        let inbox = "* TODO Review ledger import\n- 2027-01-31 -1200.00 EUR rent @landlord\n";
        assert_eq!(lines(&recurring(&rules, &marks, &[ledger, inbox], d("2027-02-28"), "EUR")), ["- 2027-02-28 -1200.00 EUR rent @landlord"]);
        // An edited rule is a new rule: a fresh watermark (today's month only), the old one dropped.
        let edited = vec!["2026-01-31 +1m 1250 rent @landlord".to_string()];
        let due = recurring(&edited, &marks, &[], d("2027-02-28"), "EUR");
        assert_eq!(lines(&due), ["- 2027-02-28 -1250.00 EUR rent @landlord"]);
        assert_eq!(due.marks.keys().collect::<Vec<_>>(), ["2026-01-31 +1m 1250 rent @landlord"]);
        // A rule starting in the future adds nothing yet.
        assert!(recurring(&["2027-01-01 +1m 5 x".into()], &none, &[], today, "EUR").entries.is_empty());
    }

    #[test]
    fn recurring_marks_round_trip() {
        let store = std::env::temp_dir().join(format!("margin-ledger-rec-{}", std::process::id())).join("ledger-recurring.json");
        assert!(load_marks(&store).is_empty());
        let marks = BTreeMap::from([("2026-11-01 +1m 5 x".to_string(), d("2026-11-08"))]);
        save_marks(&store, &marks).unwrap();
        assert_eq!(load_marks(&store), marks);
        std::fs::write(&store, "{nope").unwrap();
        assert!(load_marks(&store).is_empty());
        std::fs::remove_dir_all(store.parent().unwrap()).ok();
    }

    #[test]
    fn budgets_sum_categories_case_insensitively() {
        let text = "- 2026-10-01 -10 EUR a #food\n- 2026-10-02 -5.50 EUR b #Food\n- 2026-10-03 +2 EUR refund #food\n- 2026-10-04 -9 SEK c #food\n- 2026-10-05 -1 EUR d #fun\n";
        let (totals, _) = summarize(text, "2026-10", "EUR");
        let specs: Vec<String> = ["Food=20", "fun=0", "bad", "x=abc", "rent=399.50"].map(String::from).into();
        let b = budgets(&specs, &totals, "eur");
        assert_eq!(b.iter().map(|b| (b.category.as_str(), b.budget, b.used)).collect::<Vec<_>>(), [("Food", 2000, 1350), ("rent", 39950, 0)]);
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
    fn week_summary_compares_with_the_week_before() {
        let text = "- 2026-11-02 -10 EUR lunch #food\n- 2026-11-03 -30 EUR bus #transport\n- 2026-11-04 +5 EUR refund #fun\n- 2026-11-08 -20 EUR dinner #food\n- 2026-11-05 +100 EUR pay\n- 2026-10-30 -53 EUR old #food\n- 2026-10-26 +9 EUR old pay\n- 2026-10-25 -1 EUR too old\n- 2026-11-09 -1 EUR next week\n- 2026-10-28 -4 SEK fika\n";
        let w = week_summary(text, d("2026-11-02"), "eur");
        assert_eq!(w, [
            Week { currency: "EUR".into(), spent: 6000, last: 5300, income: 10500, top: Some(("food".into(), 3000)) },
            Week { currency: "SEK".into(), spent: 0, last: 400, income: 0, top: None },
        ]);
        // A tie goes to the first name; a refund-only category is never on top.
        let tie = week_summary("- 2026-11-02 -5 EUR x #b\n- 2026-11-02 -5 EUR y #a\n- 2026-11-03 +9 EUR z #c\n", d("2026-11-02"), "EUR");
        assert_eq!(tie[0].top, Some(("a".into(), 500)));
        assert!(week_summary(text, d("2027-01-04"), "EUR").is_empty());
    }

    #[test]
    fn month_summary_compares_with_the_month_before() {
        let text = "- 2026-11-02 -10 EUR lunch #food\n- 2026-11-03 -30 EUR bus #transport\n- 2026-11-04 +50 EUR refund #fun\n- 2026-11-30 -20 EUR dinner #food\n- 2026-10-30 -53 EUR old #food\n- 2026-09-30 -1 EUR too old\n- 2026-12-01 -1 EUR next month\n- 2026-10-28 -4 SEK fika\n";
        assert_eq!(month_summary(text, d("2026-11-01"), "eur"), [
            Week { currency: "EUR".into(), spent: 6000, last: 5300, income: 5000, top: Some(("food".into(), 3000)) },
            Week { currency: "SEK".into(), spent: 0, last: 400, income: 0, top: None },
        ]);
        // January looks back at December of the year before.
        assert_eq!(month_summary("- 2026-12-31 -7 EUR x\n", d("2027-01-01"), "EUR")[0].last, 700);
        assert!(month_summary(text, d("2027-03-01"), "EUR").is_empty());
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
    fn summarize_year_groups_months_by_currency_and_category() {
        let text = "- 2026-01-05 +1000 EUR pay #work\n- 2026-01-09 -20 EUR lunch #food\n- 2026-03-01 +5 EUR refund #food\n- 2026-03-02 -3 EUR bus\n- 2026-12-31 -100 SEK fika #food\n- 2025-12-31 -9 EUR old #food\n- 2027-01-01 -9 EUR new\n";
        let y = summarize_year(text, "2026", "eur");
        assert_eq!(y.iter().map(|y| y.currency.as_str()).collect::<Vec<_>>(), ["EUR", "SEK"]);
        let mut m = [0; 12];
        (m[0], m[2]) = (100000, 500);
        assert_eq!(y[0].income, m);
        (m[0], m[2]) = (-2000, -300);
        assert_eq!(y[0].expense, m);
        let cat = |c: Option<&str>| y[0].categories.iter().find(|(k, _)| k.as_deref() == c).unwrap().1;
        assert_eq!(y[0].categories.iter().map(|(c, _)| c.as_deref()).collect::<Vec<_>>(), [Some("work"), Some("food"), None]);
        assert_eq!((cat(Some("food"))[0], cat(Some("food"))[2], cat(None)[2]), (-2000, 500, -300));
        assert_eq!((y[1].expense[11], y[1].categories.len()), (-10000, 1));
        assert!(summarize_year(text, "2024", "EUR").is_empty());
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
        // One entry for two projects (tag and link differ) counts for both; same project twice counts once; case-twin names each get totals.
        let t = project_totals("- 2026-10-01 -10 EUR a #acme [[id:other]]\n- 2026-10-02 -5 EUR b #acme [[id:note]]\n", &["acme".into(), "Acme".into(), "x".into()], &ids);
        assert_eq!((t["acme"][0].expense, t["Acme"][0].expense, t["x"][0].expense), (-1500, -1500, -1000));
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
            assert_eq!(bank_date(s, DateOrder::Dmy), Some(d("2026-10-25")), "{s}");
        }
        assert_eq!(bank_date("Date", DateOrder::Dmy), None);
        let (dmy, mdy, ymd) = (DateOrder::Dmy, DateOrder::Mdy, DateOrder::Ymd);
        assert_eq!(bank_date("25.10.26", dmy), Some(d("2026-10-25"))); // not the year 26
        assert_eq!(bank_date("25/10/26", dmy), Some(d("2026-10-25")));
        assert_eq!(bank_date("03/10/2026", dmy), Some(d("2026-10-03")));
        assert_eq!(bank_date("03/10/2026", mdy), Some(d("2026-03-10")));
        assert_eq!(bank_date("10/25/26", mdy), Some(d("2026-10-25")));
        assert_eq!(bank_date("26-10-25", ymd), Some(d("2026-10-25")));
        assert_eq!(bank_date("26-10-25", dmy), Some(d("2025-10-26")));
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
        let us = Mapping::guess("Description,Date,Amount\nCoffee,10/25/2026,-1.00\nTea,03/10/2026,-3.50\n");
        assert_eq!(us.spec(), "date=2 amount=3 text=1 delimiter=, decimal=. dates=mdy");
        assert_eq!(Mapping::parse(&us.spec()), Ok(us));
        assert!(Mapping::parse("date=1 amount=2 dates=x").is_err());
        assert_eq!(Mapping::guess("Description,Date,Amount\nCoffee,10/25/2026,x\nTea shop,25/10/2026,-3.50\n").spec(), "date=2 amount=3 text=1 delimiter=, decimal=.");
    }

    #[test]
    fn debit_and_credit_columns() {
        let m = Mapping::parse("date=1 text=2 amount=3-4 delimiter=; decimal=,").unwrap();
        assert_eq!((m.amount, m.amount2), (2, Some(3)));
        assert_eq!(Mapping::parse(&m.spec()), Ok(m.clone()));
        assert!(Mapping::parse("date=1 amount=3-x").is_err());
        let csv = "Datum;Text;Ut;In\n2026-10-03;Shop;412,50;\n2026-10-04;Pay;;25 000,00\n2026-10-05;Refund;-3,00;\n2026-10-06;Both;5,00;5,00\n2026-10-07;Net;2,00;10,00\n2026-10-08;Empty;;\n";
        let (es, skipped) = read_csv(csv, &m, "eur", &[]);
        let got: Vec<_> = es.iter().map(|(e, _)| format_entry(e)).collect();
        assert_eq!(got, ["- 2026-10-03 -412.50 EUR Shop", "- 2026-10-04 +25000.00 EUR Pay", "- 2026-10-05 -3.00 EUR Refund", "- 2026-10-07 +8.00 EUR Net"]);
        assert_eq!(skipped, 3); // the header, net zero and both empty
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
    fn read_csv_dedupes_ignoring_tags_payee_and_links() {
        let m = Mapping::parse("date=1 amount=3 text=2 delimiter=; decimal=,").unwrap();
        let csv = "Datum;Text;Belopp\n2026-10-03;ICA STOCKHOLM KORT #1234;-10\n2026-10-03;Other;-10\n";
        let ledger = "* 2026-10\n- 2026-10-03 -10.00 EUR ica stockholm @ica #food [[id:x]]\n";
        let (es, _) = read_csv(csv, &m, "eur", &[ledger]);
        assert_eq!(es.iter().map(|x| x.1).collect::<Vec<_>>(), [false, false]);
        let ledger = "* 2026-10\n- 2026-10-03 -10.00 EUR ICA Stockholm kort @ica #food [[id:x]]\n";
        let (es, _) = read_csv(csv, &m, "eur", &[ledger]);
        assert_eq!(es.iter().map(|x| x.1).collect::<Vec<_>>(), [true, false]);
    }

    #[test]
    fn import_ignores_numeric_sigils() {
        let m = Mapping::parse("date=1 amount=3 text=2 delimiter=; decimal=,").unwrap();
        let csv = "Datum;Text;Belopp\n2026-10-03;X KORT #1234;-10\n2026-10-04;SWISH @0701234567;-5\n";
        let ledger = "* 2026-10\n- 2026-09-01 -9.00 EUR x kort #food\n- 2026-10-03 -10.00 EUR x kort #1234\n";
        let (es, _) = read_csv(csv, &m, "eur", &[ledger]);
        assert_eq!(es[0].0.text, "X KORT 1234");
        assert!(es[0].1, "old line with #1234 still counts as a duplicate");
        assert_eq!(es[1].0.payee(), None);
        let ledger = "* 2026-10\n- 2026-09-01 -9.00 EUR x kort #food\n";
        let (mut es, _) = read_csv(csv, &m, "eur", &[ledger]);
        categorize_import(ledger, &mut es);
        assert_eq!(es[0].0.text, "X KORT 1234 #food");
        assert_eq!(es[0].0.category(), Some("food"));
    }

    #[test]
    fn import_gets_category_from_ledger() {
        let m = Mapping::parse("date=1 amount=3 text=2 delimiter=; decimal=,").unwrap();
        let csv = "Datum;Text;Belopp\n2026-10-03;ICA STOCKHOLM;-10\n2026-10-03;New Shop;-5\n2026-10-04;ica stockholm #fun;-7\n";
        let ledger = "* 2026-10\n- 2026-09-01 -9.00 EUR ica stockholm #snacks\n- 2026-10-01 -9.00 EUR ICA Stockholm @ica #food\n";
        let (mut es, _) = read_csv(csv, &m, "eur", &[ledger]);
        categorize_import(ledger, &mut es);
        let got: Vec<_> = es.iter().map(|x| x.0.text.as_str()).collect();
        assert_eq!(got, ["ICA STOCKHOLM #food", "New Shop", "ica stockholm #fun"]);
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
    fn column_mapping_is_remembered_by_header() {
        let dir = std::env::temp_dir().join(format!("margin-ledger-memo-{}", std::process::id()));
        let store = dir.join("ledger-import.json");
        let csv = "\u{feff} Datum;Belopp;Text \n2026-10-02;-5,00;kaffe\n";
        assert_eq!(header_key(csv), "datum;belopp;text");
        assert_eq!(remembered(&store, csv), None);
        remember(&store, csv, "date=1 amount=2 text=3 delimiter=; decimal=,").unwrap();
        assert_eq!(remembered(&store, "Datum;BELOPP;text\n"), Some("date=1 amount=2 text=3 delimiter=; decimal=,".into()));
        assert_eq!(remembered(&store, "other;header\n"), None);
        // A header-less export (first line is a transaction) is never stored.
        remember(&store, "2026-10-02;-5,00;kaffe\n", "date=1 amount=2 text=3 delimiter=; decimal=,").unwrap();
        assert_eq!(remembered(&store, "2026-10-02;-5,00;kaffe\n"), None);
        // A corrupt store is ignored and overwritten.
        std::fs::write(&store, "{nope").unwrap();
        assert_eq!(remembered(&store, csv), None);
        remember(&store, csv, "date=1 amount=2 delimiter=;").unwrap();
        assert!(remembered(&store, csv).is_some());
        std::fs::remove_dir_all(dir).ok();
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
        // accept_all files only open entry-holding proposals and leaves everything else.
        let e = |t| format!("- 2026-10-02 -5.00 EUR {t}");
        let inbox = format!("* TODO Review ledger import\n{}\n* TODO buy milk\n* TODO Review ledger import\n{}\n* DONE Review ledger import\n{}\n* TODO Review ledger import\nno entry\n", e("a"), e("b"), e("c"));
        let (rest, ledger, n) = accept_all(&inbox, "", &kw()).unwrap();
        assert_eq!(n, 2);
        assert_eq!(rest, format!("* TODO buy milk\n* DONE Review ledger import\n{}\n* TODO Review ledger import\nno entry\n", e("c")));
        assert!(ledger.contains(" a") && ledger.contains(" b") && !ledger.contains(" c"));
        assert_eq!(accept_all(&rest, &ledger, &kw()).unwrap().2, 0);
        assert_eq!(propose("", &[], &kw()), "");
    }
}
