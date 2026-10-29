// Outline side panel (#915): the headings of a note with their TODO keyword, cookie and tags.
import { level } from "./narrow.ts";

export type OutlineItem = { line: number; level: number; title: string; keyword: string | null; done: boolean; cookie: string | null; tags: string[] };

/** Headings of TEXT (0-based LINE); KEYWORD only when it is one of the configured TODO / DONE keywords. */
export function buildOutline(text: string, todo: string[], done: string[]): OutlineItem[] {
  const out: OutlineItem[] = [], kws = new Set([...todo, ...done]);
  text.split("\n").forEach((t, line) => {
    const lvl = level(t);
    if (!lvl) return;
    let s = t.slice(lvl + 1).trim(), keyword: string | null = null, cookie: string | null = null, tags: string[] = [];
    const tm = /(?:^|\s+):([\w@#%:]+):$/.exec(s);
    if (tm) { tags = tm[1].split(":").filter(Boolean); s = s.slice(0, tm.index); }
    const w = /^(\S+)(?:\s+|$)/.exec(s);
    if (w && kws.has(w[1])) { keyword = w[1]; s = s.slice(w[0].length); }
    s = s.replace(/^\[#[A-Z0-9]+\]\s*/, "");
    const cm = /\[\d*(?:\/\d*|%)\]/.exec(s);
    if (cm) { cookie = cm[0]; s = s.slice(0, cm.index) + s.slice(cm.index + cm[0].length); }
    const title = s.replace(/\[\[(?:[^\]]+)\]\[([^\]]+)\]\]/g, "$1").replace(/\[\[([^\]]+)\]\]/g, "$1").replace(/\s+/g, " ").trim();
    out.push({ line, level: lvl, title, keyword, done: !!keyword && done.includes(keyword), cookie, tags });
  });
  return out;
}

/** Index of the heading containing LINE (the last one at or above it), -1 before the first heading. */
export function headingAt(items: OutlineItem[], line: number) {
  let i = -1;
  for (let j = 0; j < items.length && items[j].line <= line; j++) i = j;
  return i;
}

/** ITEMS matching FILTER (title or tag, case-insensitive); HIDEDONE drops done headings with their subtrees. */
export function visibleOutline(items: OutlineItem[], filter: string, hideDone: boolean) {
  const q = filter.trim().toLowerCase(), out: OutlineItem[] = [];
  let skip = 0; // level of the done heading whose subtree is being skipped
  for (const it of items) {
    if (skip && it.level > skip) continue;
    skip = hideDone && it.done ? it.level : 0;
    if (skip) continue;
    if (!q || it.title.toLowerCase().includes(q) || it.tags.some((t) => t.toLowerCase().includes(q))) out.push(it);
  }
  return out;
}
