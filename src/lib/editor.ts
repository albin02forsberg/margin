// CodeMirror 6 setup: vim, org highlighting, folding (headings + drawers),
// readable links, clickable checkboxes and the structure-editing ops.
import { EditorState, Prec, RangeSetBuilder, type Extension } from "@codemirror/state";
import { Decoration, EditorView, ViewPlugin, drawSelection, highlightActiveLine, keymap, type DecorationSet, type ViewUpdate } from "@codemirror/view";
import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
import { searchKeymap, highlightSelectionMatches } from "@codemirror/search";
import {
  StreamLanguage, HighlightStyle, syntaxHighlighting, codeFolding, foldService, foldAll, unfoldAll, toggleFold, foldedRanges, foldEffect,
} from "@codemirror/language";
import { Tag } from "@lezer/highlight";
import { vim, Vim, getCM } from "@replit/codemirror-vim";

/** Set by the app; vim ex commands, mappings and clicks call through these. */
export const hooks = {
  save: () => {},
  close: () => {},
  follow: (_target?: string) => {},
  tab: (_d: number) => {},
};

Vim.defineEx("write", "w", () => hooks.save());
Vim.defineEx("quit", "q", () => hooks.close());
Vim.defineEx("wq", "wq", () => { hooks.save(); hooks.close(); });
Vim.defineAction("orgFollow", () => hooks.follow());
Vim.mapCommand("gf", "action", "orgFollow", {}, { context: "normal" });
Vim.defineAction("nextTab", () => hooks.tab(1));
Vim.defineAction("prevTab", () => hooks.tab(-1));
Vim.mapCommand("gt", "action", "nextTab", {}, { context: "normal" });
Vim.mapCommand("gT", "action", "prevTab", {}, { context: "normal" });

// ---------------------------------------------------------------- highlighting

const T = Object.fromEntries(
  ["h1", "h2", "h3", "h4", "h5", "h6", "stars", "todo", "done", "tag", "date", "link", "prio", "planning", "prop", "meta", "title", "checkbox", "comment", "code", "bold"]
    .map((n) => [n, Tag.define()]),
);

const esc = (s: string) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

function orgLanguage(todo: string[], done: string[]) {
  const todoRe = new RegExp(`^(${todo.map(esc).join("|") || "\\b\\B"})(?= |$)`);
  const doneRe = new RegExp(`^(${done.map(esc).join("|") || "\\b\\B"})(?= |$)`);
  type S = { h: number; kw: boolean };
  return StreamLanguage.define<S>({
    name: "org",
    startState: () => ({ h: 0, kw: false }),
    tokenTable: T,
    token(stream, st) {
      if (stream.sol()) {
        st.h = 0;
        const m = stream.match(/^\*+(?= )/) as RegExpMatchArray | null;
        if (m) {
          st.h = Math.min(m[0].length, 6);
          st.kw = true;
          return "stars";
        }
        if (stream.match(/^\s*#\+title:.*$/i)) return "title";
        if (stream.match(/^\s*#\+\w+:?/)) return "meta";
        if (stream.match(/^\s*#( .*)?$/)) return "comment";
        if (stream.match(/^\s*(SCHEDULED|DEADLINE|CLOSED):/)) return "planning";
        if (stream.match(/^\s*:[\w-]+:/)) return "prop";
        if (stream.match(/^\s*([-+]|\d+[.)])\s+\[[ xX-]\]/)) return "checkbox";
      }
      if (st.h && st.kw) {
        st.kw = false;
        if (stream.eatSpace()) return `h${st.h}`;
      }
      if (st.h && stream.match(todoRe)) return "todo";
      if (st.h && stream.match(doneRe)) return "done";
      if (stream.match(/^\[\[[^\]]+\](\[[^\]]*\])?\]/)) return "link";
      if (stream.match(/^<\d{4}-\d{2}-\d{2}[^>]*>/) || stream.match(/^\[\d{4}-\d{2}-\d{2}[^\]]*\]/)) return "date";
      if (st.h && stream.match(/^\[#[A-Z]\]/)) return "prio";
      if (st.h && stream.match(/^:[\w@#%:]+:\s*$/)) return "tag";
      if (stream.match(/^[=~][^=~\s](?:[^=~]*[^=~\s])?[=~](?=\W|$)/)) return "code";
      if (stream.match(/^\*[^*\s](?:[^*]*[^*\s])?\*(?=\W|$)/)) return "bold";
      stream.next();
      stream.eatWhile(/[^[<:*=~\s]/);
      return st.h ? `h${st.h}` : null;
    },
  });
}

const orgHighlight = HighlightStyle.define([
  ...[1, 2, 3, 4, 5, 6].map((n) => ({ tag: T[`h${n}`], color: `var(--h${n})`, fontWeight: "600", fontSize: n === 1 ? "1.15em" : n === 2 ? "1.07em" : undefined })),
  { tag: T.stars, color: "var(--dim)" },
  { tag: T.todo, color: "var(--todo)", fontWeight: "700" },
  { tag: T.done, color: "var(--done)", fontWeight: "700" },
  { tag: T.tag, color: "var(--dim)", fontStyle: "italic" },
  { tag: T.date, color: "var(--date)" },
  { tag: T.link, color: "var(--link)" },
  { tag: T.prio, color: "var(--todo)" },
  { tag: T.planning, color: "var(--dim)" },
  { tag: T.prop, color: "var(--dim)" },
  { tag: T.meta, color: "var(--dim)" },
  { tag: T.title, color: "var(--h1)", fontWeight: "700", fontSize: "1.35em" },
  { tag: T.checkbox, color: "var(--date)" },
  { tag: T.comment, color: "var(--dim)", fontStyle: "italic" },
  { tag: T.code, color: "var(--code)" },
  { tag: T.bold, fontWeight: "700" },
]);

// ---------------------------------------------------------------- readable links

const LINK = /\[\[([^\]]+)\](?:\[([^\]]+)\])?\]/g;
const hidden = Decoration.replace({});
const linkMark = Decoration.mark({ class: "cm-org-link" });

/** Show `[[target][Title]]` as just "Title" except on lines with a cursor. */
function linkDecorations(view: EditorView): DecorationSet {
  const b = new RangeSetBuilder<Decoration>();
  const doc = view.state.doc;
  const active = new Set(view.state.selection.ranges.map((r) => doc.lineAt(r.head).number));
  let last = 0;
  for (const { from, to } of view.visibleRanges) {
    for (let pos = from; pos <= to; ) {
      const line = doc.lineAt(pos);
      pos = line.to + 1;
      if (line.number <= last || active.has(line.number)) continue;
      last = line.number;
      for (const m of line.text.matchAll(LINK)) {
        const s = line.from + m.index!;
        const e = s + m[0].length;
        const textFrom = m[2] ? e - 2 - m[2].length : s + 2;
        b.add(s, textFrom, hidden);
        b.add(textFrom, e - 2, linkMark);
        b.add(e - 2, e, hidden);
      }
    }
  }
  return b.finish();
}

const readableLinks = ViewPlugin.fromClass(
  class {
    decorations: DecorationSet;
    constructor(v: EditorView) { this.decorations = linkDecorations(v); }
    update(u: ViewUpdate) {
      if (u.docChanged || u.viewportChanged || u.selectionSet) this.decorations = linkDecorations(u.view);
    }
  },
  { decorations: (v) => v.decorations },
);

export function linkAt(state: EditorState, pos: number): string | null {
  const line = state.doc.lineAt(pos);
  const col = pos - line.from;
  for (const m of line.text.matchAll(LINK)) {
    if (m.index! <= col && col < m.index! + m[0].length) return m[1];
  }
  return null;
}

export const linkAtCursor = (v: EditorView) => linkAt(v.state, v.state.selection.main.head);

const CHECKBOX = /^(\s*(?:[-+]|\d+[.)])\s+\[)([ xX-])\]/;

/** Ctrl/Cmd-click follows links; clicking a [ ] checkbox toggles it. */
const clicks = EditorView.domEventHandlers({
  mousedown(e, v) {
    const pos = v.posAtCoords({ x: e.clientX, y: e.clientY });
    if (pos == null) return false;
    if (e.ctrlKey || e.metaKey) {
      const target = linkAt(v.state, pos);
      if (!target) return false;
      e.preventDefault();
      hooks.follow(target);
      return true;
    }
    const line = v.state.doc.lineAt(pos);
    const m = CHECKBOX.exec(line.text);
    const box = m ? line.from + m[1].length - 1 : -1;
    if (!m || pos < box || pos > box + 3 || v.state.readOnly) return false;
    e.preventDefault();
    v.dispatch({ changes: { from: box + 1, to: box + 2, insert: m[2] === " " ? "X" : " " } });
    return true;
  },
});

// ---------------------------------------------------------------- structure

export const level = (text: string) => /^(\*+) /.exec(text)?.[1].length ?? 0;
const DRAWER = /^\s*:(PROPERTIES|LOGBOOK):\s*$/i;

/** Last line number (1-based) of the subtree whose heading is on line N. */
function subtreeEnd(state: EditorState, n: number): number {
  const lvl = level(state.doc.line(n).text);
  let i = n + 1;
  while (i <= state.doc.lines) {
    const l = level(state.doc.line(i).text);
    if (l && l <= lvl) break;
    i++;
  }
  return i - 1;
}

/** Line number of the heading at or above line N, or 0. */
function headingAt(state: EditorState, n: number): number {
  for (let i = n; i >= 1; i--) if (level(state.doc.line(i).text)) return i;
  return 0;
}

function drawerRange(state: EditorState, n: number) {
  const line = state.doc.line(n);
  if (!DRAWER.test(line.text)) return null;
  for (let i = n + 1; i <= state.doc.lines && !level(state.doc.line(i).text); i++) {
    if (/^\s*:END:\s*$/i.test(state.doc.line(i).text)) return { from: line.to, to: state.doc.line(i).to };
  }
  return null;
}

const orgFold = foldService.of((state, from) => {
  const line = state.doc.lineAt(from);
  if (!level(line.text)) return drawerRange(state, line.number);
  const end = state.doc.line(subtreeEnd(state, line.number));
  return end.number > line.number ? { from: line.to, to: end.to } : null;
});

const curLine = (v: EditorView) => v.state.doc.lineAt(v.state.selection.main.head);
export const onHeading = (v: EditorView) => !!level(curLine(v).text);

/** The single change turning OLD into TEXT (common prefix/suffix kept). */
export function diffChange(old: string, text: string) {
  let a = 0;
  while (a < old.length && a < text.length && old[a] === text[a]) a++;
  let b = 0;
  while (b < old.length - a && b < text.length - a && old[old.length - 1 - b] === text[text.length - 1 - b]) b++;
  return { from: a, to: old.length - b, insert: text.slice(a, text.length - b) };
}

/** Replace the doc with TEXT as one minimal change, keeping cursor and undo sane. */
export function applyText(view: EditorView, text: string) {
  const old = view.state.doc.toString();
  if (old !== text) view.dispatch({ changes: diffChange(old, text) });
}

export function orgTab(v: EditorView): boolean {
  if (!level(curLine(v).text) && !DRAWER.test(curLine(v).text)) return false;
  return toggleFold(v);
}

export function orgShiftTab(v: EditorView): boolean {
  let any = false;
  foldedRanges(v.state).between(0, v.state.doc.length, () => { any = true; });
  return any ? unfoldAll(v) : foldAll(v);
}

/** Promote/demote the heading on the cursor line, or with SUBTREE all its children too. */
export function shiftHeading(v: EditorView, d: -1 | 1, subtree = false): boolean {
  const s = v.state;
  const n = curLine(v).number;
  const lvl = level(s.doc.line(n).text);
  if (!lvl || lvl + d < 1) return !!lvl;
  const last = subtree ? subtreeEnd(s, n) : n;
  const changes = [];
  for (let i = n; i <= last; i++) {
    const line = s.doc.line(i);
    if (level(line.text)) changes.push(d > 0 ? { from: line.from, insert: "*" } : { from: line.from, to: line.from + 1 });
  }
  v.dispatch({ changes });
  return true;
}

/** Swap the subtree at the cursor with its next/previous sibling. */
export function moveSubtree(v: EditorView, d: -1 | 1): boolean {
  const s = v.state;
  const h = headingAt(s, curLine(v).number);
  if (!h) return false;
  const lvl = level(s.doc.line(h).text);
  const end = subtreeEnd(s, h);
  let a: [number, number], b: [number, number];
  if (d > 0) {
    if (end >= s.doc.lines || level(s.doc.line(end + 1).text) !== lvl) return true;
    a = [h, end];
    b = [end + 1, subtreeEnd(s, end + 1)];
  } else {
    let p = headingAt(s, h - 1);
    while (p && level(s.doc.line(p).text) > lvl) p = headingAt(s, p - 1);
    if (!p || level(s.doc.line(p).text) !== lvl) return true;
    a = [p, h - 1];
    b = [h, end];
  }
  const text = (r: [number, number]) => s.sliceDoc(s.doc.line(r[0]).from, s.doc.line(r[1]).to);
  const from = s.doc.line(a[0]).from;
  const to = s.doc.line(b[1]).to;
  const offset = s.selection.main.head - s.doc.line(h).from;
  const newStart = d > 0 ? from + text(b).length + 1 : from;
  v.dispatch({ changes: { from, to, insert: text(b) + "\n" + text(a) }, selection: { anchor: newStart + offset } });
  return true;
}

/** New heading (optionally a task) at the current level after the current subtree, in insert mode. */
export function newHeading(v: EditorView, todo = ""): boolean {
  const s = v.state;
  const h = headingAt(s, curLine(v).number);
  const lvl = h ? level(s.doc.line(h).text) : 1;
  const at = h ? s.doc.line(subtreeEnd(s, h)).to : s.doc.length;
  const insert = (at > 0 ? "\n" : "") + "*".repeat(lvl) + " " + (todo ? todo + " " : "");
  v.dispatch({ changes: { from: at, insert }, selection: { anchor: at + insert.length }, scrollIntoView: true });
  insertMode(v);
  return true;
}

const ITEM = /^(\s*)([-+]|\d+[.)])(\s+)(\[[ xX-]\]\s+)?/;

/** Continue a list (new item after this line); otherwise a new heading. */
export function newItem(v: EditorView): boolean {
  const line = curLine(v);
  const m = ITEM.exec(line.text);
  if (!m) return newHeading(v);
  const bullet = /\d/.test(m[2]) ? parseInt(m[2]) + 1 + m[2].slice(-1) : m[2];
  const insert = `\n${m[1]}${bullet}${m[3]}${m[4] ? "[ ] " : ""}`;
  v.dispatch({ changes: { from: line.to, insert }, selection: { anchor: line.to + insert.length }, scrollIntoView: true });
  insertMode(v);
  return true;
}

/** Enter in insert mode on a list item: continue the list, or end it on an empty item. */
function listEnter(v: EditorView): boolean {
  const line = curLine(v);
  const m = ITEM.exec(line.text);
  if (!m || v.state.selection.main.head !== line.to) return false;
  if (m[0].length === line.text.length) {
    v.dispatch({ changes: { from: line.from, to: line.to, insert: "" } });
    return true;
  }
  return newItem(v);
}

export function toggleHeading(v: EditorView): boolean {
  const line = curLine(v);
  const lvl = level(line.text);
  v.dispatch(lvl ? { changes: { from: line.from, to: line.from + lvl + 1 } } : { changes: { from: line.from, insert: "* " } });
  return true;
}

export function toggleCheckbox(v: EditorView): boolean {
  const line = curLine(v);
  const m = CHECKBOX.exec(line.text);
  if (!m) return false;
  const pos = line.from + m[1].length;
  v.dispatch({ changes: { from: pos, to: pos + 1, insert: m[2] === " " ? "X" : " " } });
  return true;
}

const TS = /([<[])(\d{4})-(\d{2})-(\d{2})(?: ([^\s\]>\d]+))?(?: (\d{1,2}):(\d{2}))?([^\]>]*)([>\]])/dg;
const pad = (n: number) => String(n).padStart(2, "0");

/** Nudge the timestamp under the cursor. With FIELD, the part under the
 *  cursor (year/month/day/hour/5 minutes) changes; otherwise the day. */
export function shiftTimestamp(v: EditorView, dir: number, field = true): boolean {
  const line = curLine(v);
  const head = v.state.selection.main.head;
  const col = head - line.from;
  for (const m of line.text.matchAll(TS)) {
    if (col < m.index! || col >= m.index! + m[0].length) continue;
    const ix = (m as any).indices as ([number, number] | undefined)[];
    const on = (g: number) => field && !!ix[g] && ix[g]![0] <= col && col <= ix[g]![1];
    const d = new Date(+m[2], +m[3] - 1, +m[4], m[6] ? +m[6] : 0, m[7] ? +m[7] : 0);
    if (on(2)) d.setFullYear(d.getFullYear() + dir);
    else if (on(3)) d.setMonth(d.getMonth() + dir);
    else if (m[6] && on(6)) d.setHours(d.getHours() + dir);
    else if (m[7] && on(7)) d.setMinutes(d.getMinutes() + 5 * dir);
    else d.setDate(d.getDate() + dir);
    const day = d.toLocaleDateString("en-US", { weekday: "short" });
    let s = `${m[1]}${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${day}`;
    if (m[6]) s += ` ${pad(d.getHours())}:${pad(d.getMinutes())}`;
    s += m[8] + m[9];
    const from = line.from + m.index!;
    v.dispatch({ changes: { from, to: from + m[0].length, insert: s }, selection: { anchor: head } });
    return true;
  }
  return false;
}

export function insertText(v: EditorView, text: string) {
  const at = v.state.selection.main.head;
  v.dispatch({ changes: { from: at, insert: text }, selection: { anchor: at + text.length } });
}

export function insertMode(v: EditorView) {
  const cm = getCM(v) as any;
  if (cm && !cm.state.vim?.insertMode) Vim.handleKey(cm, "i", "mapping");
}

/** True when vim is idle in normal mode (no pending operator/count/keys). */
export function vimIdle(v: EditorView): boolean {
  const vs = (getCM(v) as any)?.state?.vim;
  return !!vs && !vs.insertMode && !vs.visualMode && !vs.inputState.operator && vs.inputState.keyBuffer.length === 0;
}

export function onModeChange(v: EditorView, f: (mode: string) => void) {
  (getCM(v) as any)?.on("vim-mode-change", (e: { mode: string; subMode?: string }) => f(e.subMode ? `${e.mode} ${e.subMode}` : e.mode));
}

// ---------------------------------------------------------------- state

const theme = EditorView.theme({
  "&": { height: "100%", fontSize: "15px", backgroundColor: "var(--bg)", color: "var(--fg)" },
  ".cm-scroller": { fontFamily: "var(--mono)", lineHeight: "1.6" },
  ".cm-content": { padding: "20px 0 40vh", maxWidth: "100ch", margin: "0 auto", caretColor: "var(--accent)" },
  ".cm-line": { padding: "0 32px" },
  ".cm-activeLine": { backgroundColor: "var(--active)" },
  ".cm-foldPlaceholder": { background: "none", border: "none", color: "var(--dim)", padding: "0 4px" },
  ".cm-org-link": { color: "var(--link)", textDecoration: "underline", textUnderlineOffset: "3px", cursor: "pointer" },
  "&.cm-focused .cm-selectionBackground, .cm-selectionBackground": { backgroundColor: "var(--sel) !important" },
  ".cm-fat-cursor": { background: "var(--accent) !important", color: "var(--bg) !important" },
  "&:not(.cm-focused) .cm-fat-cursor": { outline: "1px solid var(--accent)", background: "none !important" },
  ".cm-panels": { backgroundColor: "var(--panel)", color: "var(--fg)", fontFamily: "var(--mono)" },
  ".cm-vim-panel input": { color: "var(--fg)", fontFamily: "var(--mono)" },
});

export function createState(doc: string, opts: { org: boolean; readOnly: boolean; todo: string[]; done: string[]; onChange: () => void }) {
  const ext: Extension[] = [
    vim({ status: true }),
    history(),
    drawSelection(),
    highlightActiveLine(),
    highlightSelectionMatches(),
    EditorView.lineWrapping,
    codeFolding({ placeholderText: "…" }),
    keymap.of([...defaultKeymap, ...historyKeymap, ...searchKeymap, indentWithTab]),
    theme,
    EditorState.readOnly.of(opts.readOnly),
    EditorView.updateListener.of((u) => { if (u.docChanged) opts.onChange(); }),
  ];
  if (opts.org) {
    ext.push(
      orgLanguage(opts.todo, opts.done), syntaxHighlighting(orgHighlight), orgFold, readableLinks, clicks,
      Prec.high(keymap.of([{ key: "Tab", run: orgTab }, { key: "Enter", run: listEnter }])),
    );
  }
  let state = EditorState.create({ doc, extensions: ext });
  if (opts.org) {
    // Start with property/logbook drawers folded, like org.
    const effects = [];
    for (let i = 1; i <= state.doc.lines; i++) {
      const r = drawerRange(state, i);
      if (r) effects.push(foldEffect.of(r));
    }
    if (effects.length) state = state.update({ effects }).state;
  }
  return state;
}
