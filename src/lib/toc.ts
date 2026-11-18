// `#+TOC: headlines [N]` (#1295): the note's outline shows read-only under the line, as a block widget (like transclusions),
// built from the live document. Entries jump to their heading. `:noexport:` / `COMMENT` subtrees stay out, as in the export.
import { StateField, type EditorState, type Extension, type Range } from "@codemirror/state";
import { Decoration, EditorView, WidgetType, type DecorationSet } from "@codemirror/view";
import { bareTitle, headTitle } from "./links.ts";

const DIRECTIVE = /^\s*#\+toc:\s*headlines(?:\s+(\d+))?(\s+local)?\s*$/i;
/** `#+TOC: headlines [N] [local]` line: depth N (0 = all levels) and `local`; null when LINE isn't one. */
export const tocDirective = (line: string) => { const m = DIRECTIVE.exec(line); return m ? { depth: Number(m[1] ?? 0), local: !!m[2] } : null; };
/** Depth of a `#+TOC: headlines [N]` line (0 = all levels), or null when LINE isn't one. */
export const tocDepth = (line: string) => tocDirective(line)?.depth ?? null;

/** Whether replacing the whole lines OLD with NOW can change a note's TOC widgets; HAS: the note has a `#+TOC` line.
 *  Without one only a new directive counts. With one: headings and `#+` lines (blocks, directives), and any line-count
 *  change, since entries hold line numbers. Otherwise mapping the widgets is exact (#1432). */
export const tocEditMatters = (old: string, now: string, has: boolean) =>
  has ? /#\+|^\*/m.test(`${old}\n${now}`) || old.split("\n").length !== now.split("\n").length : /#\+toc/i.test(now);

export type Entry ={ line: number; level: number; title: string };
const MAX = 500;
/** The `local` TOC at line AT from ALL (the depth-0 entries): subtree of the heading above it, DEPTH counted from it. */
export function localEntries(all: Entry[], at: number, depth = 0): Entry[] {
  const i = all.findLastIndex((e) => e.line < at), base = i < 0 ? 0 : all[i].level;
  const sub = all.slice(i + 1), n = sub.findIndex((e) => e.level <= base);
  return (n < 0 || i < 0 ? sub : sub.slice(0, n)).filter((e) => !depth || e.level <= base + depth);
}

/** Headings of DOC (0-based `line`) down to level DEPTH (0 = all); skips `* ` lines in blocks and noexport/COMMENT subtrees.
 *  With AT (0-based line of a `local` TOC): only the subtree of the heading above it, DEPTH counted from it (none above: all). */
export function tocEntries(doc: string, todo: string[], done: string[], depth = 0, at = -1): Entry[] {
  if (at >= 0) return localEntries(tocEntries(doc, todo, done), at, depth);
  const out: Entry[] = [], kw = [...todo, ...done];
  let block = false, skip = 0; // skip: level of the excluded heading whose subtree we're in
  doc.split("\n").forEach((text, line) => {
    if (/^\s*#\+begin_/i.test(text)) block = true;
    else if (/^\s*#\+end_/i.test(text)) block = false;
    if (block) return;
    const lv = /^(\*+)\s/.exec(text)?.[1].length;
    if (!lv || out.length >= MAX) return;
    if (skip && lv > skip) return;
    skip = 0;
    const raw = headTitle(text, kw);
    if (raw == null) return;
    const tags = /\s:([\w@#%:]+):\s*$/.exec(text)?.[1].split(":") ?? [];
    if (tags.some((t) => t.toLowerCase() === "noexport") || raw === "COMMENT" || raw.startsWith("COMMENT ")) return void (skip = lv);
    if (depth && lv > depth) return;
    out.push({ line, level: lv, title: bareTitle(raw) });
  });
  return out;
}

class Toc extends WidgetType {
  entries: Entry[];
  constructor(entries: Entry[]) { super(); this.entries = entries; }
  eq(o: Toc) { return o.entries.length === this.entries.length && o.entries.every((e, i) => { const f = this.entries[i]; return e.line === f.line && e.level === f.level && e.title === f.title; }); }
  toDOM(view: EditorView) {
    const d = document.createElement("div"), top = Math.min(...this.entries.map((e) => e.level));
    d.className = "cm-toc";
    for (const e of this.entries) {
      const a = document.createElement("div");
      a.className = "cm-toc-item";
      a.style.paddingLeft = `${(e.level - top) * 1.2}em`;
      a.textContent = e.title || "(untitled)";
      a.onmousedown = (ev) => {
        ev.preventDefault();
        if (e.line >= view.state.doc.lines) return;
        view.dispatch({ selection: { anchor: view.state.doc.line(e.line + 1).from }, scrollIntoView: true });
        view.focus();
      };
      d.append(a);
    }
    return d;
  }
  ignoreEvent() { return true; }
}

/** `#+TOC: headlines` tables of contents; TODO and DONE give the note's keywords. */
export function toc(todo: () => string[], done: () => string[]): Extension {
  type Val = { decos: DecorationSet; has: boolean };
  const build = (s: EditorState): Val => {
    const ranges: Range<Decoration>[] = [];
    let block = false, has = false, doc: string | null = null, kw: [string[], string[]] | null = null;
    const scans = new Map<number, Entry[]>(); // one scan per distinct depth, shared by the note's directives
    const scan = (depth: number) => { let r = scans.get(depth); if (!r) scans.set(depth, r = tocEntries((doc ??= s.doc.toString()), ...(kw ??= [todo(), done()]), depth)); return r; };
    for (let i = 1; i <= s.doc.lines; i++) {
      const l = s.doc.line(i);
      if (/^\s*#\+begin_/i.test(l.text)) block = true;
      else if (/^\s*#\+end_/i.test(l.text)) block = false;
      const dir = l.text.includes("#+") ? tocDirective(l.text) : null;
      has ||= !!dir;
      if (!dir || block) continue;
      const entries = dir.local ? localEntries(scan(0), i - 1, dir.depth) : scan(dir.depth);
      if (entries.length) ranges.push(Decoration.widget({ widget: new Toc(entries), block: true, side: 1 }).range(l.to));
    }
    return { decos: Decoration.set(ranges), has };
  };
  const lines = (doc: EditorState["doc"], a: number, b: number) => doc.sliceString(doc.lineAt(a).from, doc.lineAt(b).to);
  return [
    StateField.define<Val>({
      create: build,
      update: (v, tr) => {
        if (!tr.docChanged) return v;
        let hit = false;
        tr.changes.iterChangedRanges((fa, ta, fb, tb) => { hit ||= tocEditMatters(lines(tr.startState.doc, fa, ta), lines(tr.state.doc, fb, tb), v.has); });
        return hit ? build(tr.state) : v.has ? { decos: v.decos.map(tr.changes), has: true } : v;
      },
      provide: (f) => EditorView.decorations.from(f, (v) => v.decos),
    }),
    EditorView.baseTheme({
      ".cm-toc": { color: "var(--dim)", background: "var(--panel)", borderLeft: "3px solid var(--border)", padding: "0.2em 0.8em", margin: "0.1em 0 0.3em", fontSize: "0.92em", userSelect: "none", borderRadius: "0 var(--radius-sm) var(--radius-sm) 0" },
      ".cm-toc-item": { cursor: "pointer" },
      ".cm-toc-item:hover": { color: "var(--accent)" },
    }),
  ];
}
