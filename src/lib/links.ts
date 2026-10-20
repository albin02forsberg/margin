/** Org link targets: split `file:path::search` and resolve the search part to a line. */

/** `file:a.org::*Head` -> {file: "a.org", search: "*Head"}; `*Head` / `#id` alone -> {file: "", search} (current note). */
export function splitTarget(t: string): { file: string; search: string } {
  if (!t.startsWith("file:") && /^[*#]/.test(t)) return { file: "", search: t };
  const s = t.replace(/^file:/, "");
  const i = s.indexOf("::");
  return i < 0 ? { file: s, search: "" } : { file: s.slice(0, i), search: s.slice(i + 2) };
}

const KW = "TODO|DONE|NEXT|WAIT\\w*|CANCEL\\w*";
const esc = (s: string) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
/** Heading regex capturing the title after an optional keyword (from KEYWORDS, else the defaults), priority and tags. */
const head = (keywords?: string[]) => new RegExp(`^\\*+\\s+(?:(?:${keywords?.length ? keywords.map(esc).join("|") : KW})\\s+)?(?:\\[#[A-Z0-9]+\\]\\s+)?(.*?)(?:\\s+:[\\w@#%:]+:)?\\s*$`);
/** A heading title without statistics cookies (`[2/5]`, `[40%]`) and with collapsed whitespace. */
export const bareTitle = (s: string) => s.replace(/\[\d*(?:\/\d*|%)\]/g, " ").replace(/\s+/g, " ").trim();
/** Title for comparison, as org does: bare and lowercase. */
const norm = (s: string) => bareTitle(s).toLowerCase();

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
