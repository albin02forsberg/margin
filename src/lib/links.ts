/** Org link targets: split `file:path::search` and resolve the search part to a line. */

/** `file:a.org::*Head` -> {file: "a.org", search: "*Head"}; `*Head` / `#id` alone -> {file: "", search} (current note). */
export function splitTarget(t: string): { file: string; search: string } {
  if (!t.startsWith("file:") && /^[*#]/.test(t)) return { file: "", search: t };
  const s = t.replace(/^file:/, "");
  const i = s.indexOf("::");
  return i < 0 ? { file: s, search: "" } : { file: s.slice(0, i), search: s.slice(i + 2) };
}

/** Link T as `transclude_text` takes it, for the peek panel (#849): bare paths get `file:`; null for URLs and other schemes. */
export function peekTarget(t: string): string | null {
  if (/^(id|file):|^[*#]/.test(t)) return t;
  return /^[a-z][\w+.-]+:/i.test(t) ? null : `file:${t}`;
}

const KW = "TODO|DONE|NEXT|WAIT\\w*|CANCEL\\w*";
const esc = (s: string) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
/** Heading regex capturing the title after an optional keyword (from KEYWORDS, else the defaults), priority and tags. */
const head = (keywords?: string[]) => new RegExp(`^\\*+\\s+(?:(?:${keywords?.length ? keywords.map(esc).join("|") : KW})\\s+)?(?:\\[#[A-Z0-9]+\\]\\s+)?(.*?)(?:\\s+:[\\w@#%:]+:)?\\s*$`);
/** Title of heading LINE (keyword, priority and tags stripped), or null when LINE isn't a heading. */
export const headTitle = (line: string, keywords?: string[]) => head(keywords).exec(line)?.[1] ?? null;
/** A heading title without statistics cookies (`[2/5]`, `[40%]`) and with collapsed whitespace. */
export const bareTitle = (s: string) => s.replace(/\[\d*(?:\/\d*|%)\]/g, " ").replace(/\s+/g, " ").trim();
/** Title for comparison, as org does: bare and lowercase. Mirrored by `notes::heading_norm` in Rust. */
export const norm = (s: string) => bareTitle(s).toLowerCase();

/** Headings of TARGETS a `::*Title` link can reach, with their bare title: not those whose `path:line` is in WITH_ID
 *  (they get `[[id:]]` links), nor empty titles, titles with brackets (they would break `[[…][…]]`), or a title
 *  repeated earlier in its file (org follows the first match). */
export function linkHeadings<T extends { path: string; line: number | null; title: string }>(targets: T[], withId: Set<string>) {
  const seen = new Set<string>(), out: (T & { bare: string })[] = [];
  for (const t of targets) {
    const bare = bareTitle(t.title), key = `${t.path}\0${bare.toLowerCase()}`;
    if (t.line == null || !bare) continue;
    if (seen.has(key)) continue;
    seen.add(key);
    if (!/[[\]]/.test(bare) && !withId.has(`${t.path}:${t.line}`)) out.push({ ...t, bare });
  }
  return out;
}

/** 0-based line a search string points at (`*Heading`, `#custom-id`, line number, or plain text), or null.
 *  KEYWORDS are the todo/done keywords a `*Heading` may carry (defaults to TODO, DONE, NEXT, WAIT*, CANCEL*). */
export function findTarget(text: string, search: string, keywords?: string[]): number | null {
  const lines = text.split(/\r?\n/);
  if (!search) return null;
  if (/^\d+$/.test(search)) return Math.min(Math.max(+search, 1), lines.length) - 1;
  if (search[0] === "*") {
    const want = norm(search.slice(1)), re = head(keywords);
    const i = lines.findIndex((l) => { const m = re.exec(l); return !!m && norm(m[1]) === want; });
    return i < 0 ? null : i;
  }
  if (search[0] === "#") {
    const id = search.slice(1).trim();
    const re = /^\s*:CUSTOM_ID:\s*(\S+)\s*$/i;
    for (let i = 0; i < lines.length; i++) {
      if (re.exec(lines[i])?.[1] !== id) continue;
      for (let j = i - 1; j >= 0; j--) if (/^\*+\s/.test(lines[j])) return j;
      return i;
    }
    return null;
  }
  const i = lines.findIndex((l) => l.includes(search));
  return i < 0 ? null : i;
}

/** Link titles for renaming heading LINE (0-based) of BEFORE to its title in AFTER: {old: normalised old title,
 *  new: bare new title} to rewrite `::*Old` links with, a reason to leave them alone, or null when there is nothing
 *  to do (not a heading, or only case/cookies changed). Links are left alone when the old title isn't unique in
 *  the note (they were ambiguous already), or the new one is empty, has brackets or is another heading's title. */
export const NOT_A_LINK = "the new title can't be a link";
/** The title change of the heading at LINE when the new title has brackets (so links are left alone) but is otherwise a safe rename: for re-anchoring bookmarks. */
export function bracketRetitle(before: string, after: string, line: number, keywords?: string[]): { old: string; new: string } | null {
  if (retitlePlan(before, after, line, keywords) !== NOT_A_LINK) return null;
  const re = head(keywords), t = after.split(/\r?\n/)[line], m = re.exec(t);
  const o = re.exec(before.split(/\r?\n/)[line]);
  return m && o ? { old: norm(o[1]), new: bareTitle(m[1]) } : null;
}
export function retitlePlan(before: string, after: string, line: number, keywords?: string[]): { old: string; new: string } | string | null {
  const re = head(keywords), titles = (s: string) => s.split(/\r?\n/).map((l) => re.exec(l)?.[1] ?? null);
  const b = titles(before), a = titles(after), o = b[line], t = a[line];
  if (o == null || t == null) return null;
  const old = norm(o), bare = bareTitle(t);
  if (!old || norm(bare) === old) return null;
  if (b.filter((x) => x != null && norm(x) === old).length > 1) return "the old title isn't unique in the note";
  if (!bare || /[[\]]/.test(bare)) return NOT_A_LINK;
  if (a.some((x, i) => i !== line && x != null && norm(x) === norm(bare))) return "another heading has that title";
  return { old, new: bare };
}
