// CodeMirror 6 setup: vim, org highlighting, heading folding and the
// org structure-editing ops (evil-org style).
import { EditorState, Prec, type Extension } from "@codemirror/state";
import { EditorView, drawSelection, highlightActiveLine, keymap } from "@codemirror/view";
import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
import { searchKeymap, highlightSelectionMatches } from "@codemirror/search";
import {
  StreamLanguage, HighlightStyle, syntaxHighlighting, codeFolding, foldService, foldAll, unfoldAll, toggleFold, foldedRanges,
} from "@codemirror/language";
import { Tag } from "@lezer/highlight";
import { vim, Vim, getCM } from "@replit/codemirror-vim";

/** Set by the app; vim ex commands and mappings call through these. */
export const hooks = {
  save: () => {},
  close: () => {},
  follow: () => {},
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
  ...[1, 2, 3, 4, 5, 6].map((n) => ({ tag: T[`h${n}`], color: `var(--h${n})`, fontWeight: "600" })),
  { tag: T.stars, color: "var(--dim)" },
  { tag: T.todo, color: "var(--todo)", fontWeight: "700" },
  { tag: T.done, color: "var(--done)", fontWeight: "700" },
  { tag: T.tag, color: "var(--dim)", fontStyle: "italic" },
  { tag: T.date, color: "var(--date)", textDecoration: "underline" },
  { tag: T.link, color: "var(--link)", textDecoration: "underline" },
  { tag: T.prio, color: "var(--todo)" },
  { tag: T.planning, color: "var(--dim)" },
  { tag: T.prop, color: "var(--dim)" },
  { tag: T.meta, color: "var(--dim)" },
  { tag: T.title, color: "var(--h1)", fontWeight: "700", fontSize: "1.2em" },
  { tag: T.checkbox, color: "var(--date)" },
  { tag: T.comment, color: "var(--dim)", fontStyle: "italic" },
  { tag: T.code, color: "var(--code)" },
  { tag: T.bold, fontWeight: "700" },
]);

// ---------------------------------------------------------------- structure

export const level = (text: string) => /^(\*+) /.exec(text)?.[1].length ?? 0;

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

const headingFold = foldService.of((state, from) => {
  const line = state.doc.lineAt(from);
  if (!level(line.text)) return null;
  const end = state.doc.line(subtreeEnd(state, line.number));
  return end.number > line.number ? { from: line.to, to: end.to } : null;
});

const curLine = (v: EditorView) => v.state.doc.lineAt(v.state.selection.main.head);

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
  if (!level(curLine(v).text)) return false;
  return toggleFold(v);
}

export function orgShiftTab(v: EditorView): boolean {
  let any = false;
  foldedRanges(v.state).between(0, v.state.doc.length, () => { any = true; });
  return any ? unfoldAll(v) : foldAll(v);
}

/** M-h / M-l: promote or demote the heading on the cursor line. */
export function shiftHeading(v: EditorView, d: -1 | 1): boolean {
  const line = curLine(v);
  const lvl = level(line.text);
  if (!lvl || lvl + d < 1) return !!lvl;
  v.dispatch(d > 0 ? { changes: { from: line.from, insert: "*" } } : { changes: { from: line.from, to: line.from + 1 } });
  return true;
}

/** M-j / M-k: swap the subtree at the cursor with its next/previous sibling. */
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
  const head = s.selection.main.head;
  const offset = head - s.doc.line(h).from;
  const insert = text(b) + "\n" + text(a);
  const newStart = d > 0 ? from + text(b).length + 1 : from;
  v.dispatch({ changes: { from, to, insert }, selection: { anchor: newStart + offset } });
  return true;
}

/** M-Enter: new heading at the current level after the current subtree, in insert mode. */
export function newHeading(v: EditorView): boolean {
  const s = v.state;
  const h = headingAt(s, curLine(v).number);
  const lvl = h ? level(s.doc.line(h).text) : 1;
  const at = h ? s.doc.line(subtreeEnd(s, h)).to : s.doc.length;
  const insert = "\n" + "*".repeat(lvl) + " ";
  v.dispatch({ changes: { from: at, insert }, selection: { anchor: at + insert.length }, scrollIntoView: true });
  insertMode(v);
  return true;
}

export function toggleCheckbox(v: EditorView): boolean {
  const line = curLine(v);
  const m = /^(\s*(?:[-+]|\d+[.)])\s+\[)([ xX-])\]/.exec(line.text);
  if (!m) return false;
  const pos = line.from + m[1].length;
  v.dispatch({ changes: { from: pos, to: pos + 1, insert: m[2] === " " ? "X" : " " } });
  return true;
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

/** The [[link]] target under the cursor, if any. */
export function linkAtCursor(v: EditorView): string | null {
  const line = curLine(v);
  const col = v.state.selection.main.head - line.from;
  for (const m of line.text.matchAll(/\[\[([^\]]+)\](?:\[[^\]]*\])?\]/g)) {
    if (m.index! <= col && col < m.index! + m[0].length) return m[1];
  }
  return null;
}

// ---------------------------------------------------------------- state

const theme = EditorView.theme({
  "&": { height: "100%", fontSize: "15px", backgroundColor: "var(--bg)", color: "var(--fg)" },
  ".cm-scroller": { fontFamily: "var(--mono)", lineHeight: "1.55" },
  ".cm-content": { padding: "12px 0", maxWidth: "110ch", caretColor: "var(--accent)" },
  ".cm-line": { padding: "0 24px" },
  ".cm-activeLine": { backgroundColor: "var(--active)" },
  ".cm-foldPlaceholder": { background: "none", border: "none", color: "var(--dim)" },
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
    codeFolding({ placeholderText: " …" }),
    keymap.of([...defaultKeymap, ...historyKeymap, ...searchKeymap, indentWithTab]),
    theme,
    EditorState.readOnly.of(opts.readOnly),
    EditorView.updateListener.of((u) => { if (u.docChanged) opts.onChange(); }),
  ];
  if (opts.org) ext.push(orgLanguage(opts.todo, opts.done), syntaxHighlighting(orgHighlight), headingFold, Prec.high(keymap.of([{ key: "Tab", run: orgTab }])));
  return EditorState.create({ doc, extensions: ext });
}
