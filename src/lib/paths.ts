/** Path from the folder of FROM (a file) to TO, with "/" separators, as org file: links expect. */
export function relativeTo(from: string, to: string): string {
  const parts = (p: string) => p.split(/[\\/]+/).filter(Boolean);
  const a = parts(from).slice(0, -1), b = parts(to);
  let i = 0;
  while (i < a.length && i < b.length - 1 && a[i] === b[i]) i++;
  return [...Array(a.length - i).fill(".."), ...b.slice(i)].join("/");
}

/** DIR (absolute) joined with the relative path P using SEP, "." and ".." steps resolved (never above the root),
 *  since the backend rejects paths with "..". */
export function resolvePath(dir: string, p: string, sep = "/"): string {
  const [root, ...rest] = dir.split(/[\\/]/);
  const out = rest.filter(Boolean);
  for (const s of p.split(/[\\/]+/)) {
    if (s === "..") out.pop();
    else if (s && s !== ".") out.push(s);
  }
  return [root, ...out].join(sep);
}

/** The existing journal entry before (dir -1) or after (+1) DATE (YYYY-MM-DD, need not exist) among FILES (any separators) in DAILY ("a/b", any separators). */
export function journalNeighbour(files: string[], daily: string, date: string, dir: 1 | -1): string | undefined {
  const d = daily.split(/[\\/]+/).filter(Boolean);
  const days = files.map((f) => ({ f, p: f.split(/[\\/]+/) })).filter(({ p }) =>
    /^\d{4}-\d{2}-\d{2}\.org$/.test(p[p.length - 1]) && p.length > d.length && d.every((s, i) => p[p.length - 1 - d.length + i] === s))
    .map(({ f, p }) => ({ f, n: p[p.length - 1].slice(0, 10) }));
  return days.filter((x) => (dir > 0 ? x.n > date : x.n < date)).sort((a, b) => dir * a.n.localeCompare(b.n))[0]?.f;
}

/** The existing journal entries (in DAILY) dated FROM..TO (YYYY-MM-DD, inclusive) among FILES, oldest first. */
export const weekJournals = (files: string[], daily: string, from: string, to: string) =>
  files.filter((f) => journalNeighbour([f], daily, "0000-00-00", 1)).map((f) => ({ f, n: f.split(/[\\/]+/).pop()!.slice(0, 10) }))
    .filter((x) => x.n >= from && x.n <= to).sort((a, b) => a.n.localeCompare(b.n)).map((x) => x.f);

/** Journal entries (in DAILY) on the same day of an earlier year (same month), then of an earlier month this year, newest first, relative to TODAY (YYYY-MM-DD).
 *  Months/years lacking that day (Feb 29, the 31st) simply have no entry to match. `ago` is e.g. "1 year ago" / "3 months ago". */
export function onThisDay(files: string[], daily: string, today: string): { path: string; date: string; ago: string }[] {
  const [ty, tm, td] = today.split("-").map(Number);
  const out: { path: string; date: string; ago: string }[] = [];
  for (const path of files) {
    if (!journalNeighbour([path], daily, "0000-00-00", 1)) continue; // not a YYYY-MM-DD.org in DAILY
    const date = path.split(/[\\/]+/).pop()!.slice(0, 10), [y, m, d] = date.split("-").map(Number);
    if (d !== td || date >= today) continue;
    const yrs = ty - y, mos = tm - m;
    if (yrs > 0 && mos === 0) out.push({ path, date, ago: `${yrs} year${yrs > 1 ? "s" : ""} ago` });
    else if (yrs === 0 && mos > 0) out.push({ path, date, ago: `${mos} month${mos > 1 ? "s" : ""} ago` });
  }
  return out.sort((a, b) => b.date.localeCompare(a.date));
}

/** How long ago the epoch-seconds time THEN is at NOW (epoch seconds): "just now" (also for the future), "5 min ago", "3 h ago", "yesterday", "4 days ago", else the date (YYYY-MM-DD, UTC). */
export function ago(then: number, now: number): string {
  const d = now - then;
  if (d < 60) return "just now";
  if (d < 3600) return `${Math.floor(d / 60)} min ago`;
  if (d < 86400) return `${Math.floor(d / 3600)} h ago`;
  if (d < 2 * 86400) return "yesterday";
  if (d < 7 * 86400) return `${Math.floor(d / 86400)} days ago`;
  return new Date(then * 1000).toISOString().slice(0, 10);
}

/** Folders ("a/b", "" for the notes folder itself) holding the notes FILES (relative, any separators), sorted;
 *  dot folders (like .trash), the attachments folder ATTACH and any folder at or under an IMAGES folder (at any depth, like the backend) are left out. */
export function noteFolders(files: string[], attach: string, images = ""): string[] {
  const a = folderName(attach), im = folderName(images);
  const inImages = (d: string) => !!im && d.split("/").some((_, k, p) => { const x = p.slice(0, k + 1).join("/"); return x === im || x.endsWith("/" + im); });
  const out = new Set([""]);
  for (const f of files) {
    const p = f.split(/[\\/]+/).slice(0, -1);
    for (let i = 1; i <= p.length; i++) out.add(p.slice(0, i).join("/"));
  }
  return [...out].filter((d) => !d.split("/").some((s) => s.startsWith(".")) && !(a && (d === a || d.startsWith(a + "/"))) && !inImages(d)).sort();
}

/** A typed folder NAME as "a/b" relative to the notes folder ("" for the folder itself), or null when it
 *  starts with a slash or drive, or has a ".." step that could leave the notes folder. */
export function folderName(name: string): string | null {
  const n = name.trim();
  if (/^([\\/]|[a-zA-Z]:)/.test(n)) return null;
  const parts = n.split(/[\\/]+/).filter((s) => s && s !== ".");
  return parts.includes("..") ? null : parts.join("/");
}

/** The changed paths of two coalesced syncs; undefined (check everything) wins. */
export const mergeChanged = (a?: string[], b?: string[]) => (a && b ? [...new Set([...a, ...b])] : undefined);

/** Whether every changed PATH is our own echo or not in REFRESH (the open files); Set lookups keep it linear. */
export const onlyEchoes = (paths: string[], refresh: string[], echo: Set<string>) => {
  const r = new Set(refresh);
  return paths.every((p) => echo.has(p) || !r.has(p));
};

/** Whether FILE is one of the changed PATHS or under one (a moved or deleted folder); no PATHS means everything changed. */
export const changed = (file: string, paths?: string[]) => !paths || paths.some((p) => file === p || (file.startsWith(p) && /[\\/]/.test(file[p.length])));


/** `:e NAME`: the note whose path (absolute, or relative to NOTES), file name or TITLES entry matches NAME, case-insensitively and `.org` optional. */
export function findNote(name: string, files: string[], titles: Map<string, string>, notes: string): string | undefined {
  const key = (s: string) => s.toLowerCase().replace(/\\/g, "/").replace(/\.org$/, "");
  const n = key(name.trim());
  const rel = (f: string) => (f.startsWith(notes) ? f.slice(notes.length + 1) : f);
  const file = (f: string) => key(f.split(/[\\/]/).pop()!);
  return files.find((f) => key(f) === n || key(rel(f)) === n)
    ?? files.find((f) => file(f) === n || file(f).replace(/^\d{14}-/, "").replace(/_/g, " ") === n)
    ?? files.find((f) => titles.get(f)?.toLowerCase() === name.trim().toLowerCase());
}

/** `:b #` / `<C-^>`: the index of the tab with key PREV (the one shown before), -1 if it is closed or already CUR. */
export const altTab = (prev: string, tabs: { key: string }[], cur: number): number => {
  const i = prev ? tabs.findIndex((t) => t.key === prev) : -1;
  return i === cur ? -1 : i;
};

/** `:b NAME`: the index of the open tab NAME picks: a 1-based number, a `findNote` match, else the first title containing NAME; -1 for none. */
export function findTab(name: string, tabs: { title: string; path?: string }[], notes: string): number {
  const n = name.trim(), q = n.toLowerCase();
  if (/^\d+$/.test(n)) return +n >= 1 && +n <= tabs.length ? +n - 1 : -1;
  const files = tabs.flatMap((t) => (t.path ? [t.path] : []));
  const f = findNote(n, files, new Map(tabs.flatMap((t) => (t.path ? [[t.path, t.title] as [string, string]] : []))), notes);
  if (f) return tabs.findIndex((t) => t.path === f);
  const i = tabs.findIndex((t) => t.title.toLowerCase() === q);
  return i >= 0 ? i : tabs.findIndex((t) => t.title.toLowerCase().includes(q));
}
