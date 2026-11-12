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

export type Entry = { line: number; level: number; title: string };
const MAX = 500;
/** Headings of DOC (0-based `line`) down to level DEPTH (0 = all); skips `* ` lines in blocks and noexport/COMMENT subtrees.
 *  With AT (0-based line of a `local` TOC): only the subtree of the heading above it, DEPTH counted from it (none above: all). */
export function tocEntries(doc: string, todo: string[], done: string[], depth = 0, at = -1): Entry[] {
  if (at >= 0) {
    const all = tocEntries(doc, todo, done), i = all.findLastIndex((e) => e.line < at), base = i < 0 ? 0 : all[i].level;
    const sub = all.slice(i + 1), n = sub.findIndex((e) => e.level <= base);
    return (n < 0 || i < 0 ? sub : sub.slice(0, n)).filter((e) => !depth || e.level <= base + depth);
  }
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
  eq(o: Toc) { return JSON.stringify(o.entries) === JSON.stringify(this.entries); }
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
  const build = (s: EditorState): DecorationSet => {
    const ranges: Range<Decoration>[] = [];
    let block = false, doc: string | null = null;
    for (let i = 1; i <= s.doc.lines; i++) {
      const l = s.doc.line(i);
      if (/^\s*#\+begin_/i.test(l.text)) block = true;
      else if (/^\s*#\+end_/i.test(l.text)) block = false;
      const dir = !block && l.text.includes("#+") ? tocDirective(l.text) : null;
      if (!dir) continue;
      const entries = tocEntries((doc ??= s.doc.toString()), todo(), done(), dir.depth, dir.local ? i - 1 : -1);
      if (entries.length) ranges.push(Decoration.widget({ widget: new Toc(entries), block: true, side: 1 }).range(l.to));
    }
    return Decoration.set(ranges);
  };
  return [
    StateField.define<DecorationSet>({
      create: build,
      update: (d, tr) => (tr.docChanged ? build(tr.state) : d),
      provide: (f) => EditorView.decorations.from(f),
    }),
    EditorView.baseTheme({
      ".cm-toc": { color: "var(--dim)", background: "var(--panel)", borderLeft: "3px solid var(--border)", padding: "0.2em 0.8em", margin: "0.1em 0 0.3em", fontSize: "0.92em", userSelect: "none", borderRadius: "0 var(--radius-sm) var(--radius-sm) 0" },
      ".cm-toc-item": { cursor: "pointer" },
      ".cm-toc-item:hover": { color: "var(--accent)" },
    }),
  ];
}
