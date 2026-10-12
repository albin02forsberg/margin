/** Path from the folder of FROM (a file) to TO, with "/" separators, as org file: links expect. */
export function relativeTo(from: string, to: string): string {
  const parts = (p: string) => p.split(/[\\/]+/).filter(Boolean);
  const a = parts(from).slice(0, -1), b = parts(to);
  let i = 0;
  while (i < a.length && i < b.length - 1 && a[i] === b[i]) i++;
  return [...Array(a.length - i).fill(".."), ...b.slice(i)].join("/");
}

/** The existing journal entry before (dir -1) or after (+1) DATE (YYYY-MM-DD, need not exist) among FILES (any separators) in DAILY ("a/b", any separators). */
export function journalNeighbour(files: string[], daily: string, date: string, dir: 1 | -1): string | undefined {
  const d = daily.split(/[\\/]+/).filter(Boolean);
  const days = files.map((f) => ({ f, p: f.split(/[\\/]+/) })).filter(({ p }) =>
    /^\d{4}-\d{2}-\d{2}\.org$/.test(p[p.length - 1]) && p.length > d.length && d.every((s, i) => p[p.length - 1 - d.length + i] === s))
    .map(({ f, p }) => ({ f, n: p[p.length - 1].slice(0, 10) }));
  return days.filter((x) => (dir > 0 ? x.n > date : x.n < date)).sort((a, b) => dir * a.n.localeCompare(b.n))[0]?.f;
}

/** The changed paths of two coalesced syncs; undefined (check everything) wins. */
export const mergeChanged = (a?: string[], b?: string[]) => (a && b ? [...new Set([...a, ...b])] : undefined);

/** Whether FILE is one of the changed PATHS or under one (a moved or deleted folder); no PATHS means everything changed. */
export const changed = (file: string, paths?: string[]) => !paths || paths.some((p) => file === p || (file.startsWith(p) && /[\\/]/.test(file[p.length])));
