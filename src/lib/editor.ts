// CodeMirror 6 setup: vim, org styling (decorations that hide markup except on
// the line being edited), folding, tables, blocks and the structure-editing ops.
import { EditorState, Facet, Prec, StateEffect, type Extension, type Range } from "@codemirror/state";
import {
  Decoration, EditorView, ViewPlugin, WidgetType, drawSelection, highlightActiveLine, keymap, type DecorationSet, type ViewUpdate,
} from "@codemirror/view";
import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
import { searchKeymap, highlightSelectionMatches } from "@codemirror/search";
import { codeFolding, foldService, foldAll, unfoldAll, toggleFold, foldedRanges, foldEffect, LanguageDescription, type Language } from "@codemirror/language";
import { languages } from "@codemirror/language-data";
import { highlightTree, classHighlighter } from "@lezer/highlight";
import { vim, Vim, getCM } from "@replit/codemirror-vim";
import { convertFileSrc } from "@tauri-apps/api/core";
import * as tbl from "./orgtable";

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

type OrgConf = { todo: string[]; done: string[]; path?: string };
const orgConf = Facet.define<OrgConf, OrgConf>({ combine: (v) => v[0] ?? { todo: [], done: [] } });

// ---------------------------------------------------------------- document structure

export const level = (text: string) => /^(\*+) /.exec(text)?.[1].length ?? 0;
const DRAWER = /^\s*:(PROPERTIES|LOGBOOK):\s*$/i;
const CHECKBOX = /^(\s*(?:[-+]|\d+[.)])\s+\[)([ xX-])\]/;
const ITEM = /^(\s*)([-+]|\d+[.)]|\*(?=\s))(\s+)(\[[ xX-]\]\s+)?/;
const LINK = /\[\[([^\]]+)\](?:\[([^\]]+)\])?\]/g;
const TS = /([<[])(\d{4})-(\d{2})-(\d{2})(?: ([^\s\]>\d]+))?(?: (\d{1,2}):(\d{2}))?([^\]>]*)([>\]])/dg;
const IMAGE = /\.(png|jpe?g|gif|svg|webp|bmp)$/i;

type Block = { start: number; end: number; kind: string; lang: string };
type Tbl = { start: number; end: number; sep: number };

/** Blocks (#+begin_x … #+end_x) and tables, by 1-based line number. */
function scan(state: EditorState) {
  const blocks: Block[] = [];
  const tables: Tbl[] = [];
  const doc = state.doc;
  for (let i = 1; i <= doc.lines; i++) {
    const t = doc.line(i).text;
    const b = /^\s*#\+begin_(\w+)\s*(\S*)/i.exec(t);
    if (b) {
      const endRe = new RegExp(`^\\s*#\\+end_${b[1]}\\b`, "i");
      let j = i + 1;
      while (j <= doc.lines && !endRe.test(doc.line(j).text)) j++;
      if (j <= doc.lines) {
        blocks.push({ start: i, end: j, kind: b[1].toLowerCase(), lang: b[2] });
        i = j;
      }
      continue;
    }
    if (tbl.isTableLine(t)) {
      let j = i;
      while (j < doc.lines && tbl.isTableLine(doc.line(j + 1).text)) j++;
      let sep = 0;
      for (let k = i; k <= j && !sep; k++) if (tbl.isSep(doc.line(k).text)) sep = k;
      tables.push({ start: i, end: j, sep });
      i = j;
    }
  }
  return { blocks, tables };
}

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
  if (!level(line.text)) {
    const b = /^\s*#\+begin_(\w+)/i.exec(line.text);
    if (b) {
      const endRe = new RegExp(`^\\s*#\\+end_${b[1]}\\b`, "i");
      for (let j = line.number + 1; j <= state.doc.lines; j++) {
        if (endRe.test(state.doc.line(j).text)) return { from: line.to, to: state.doc.line(j).to };
      }
    }
    return drawerRange(state, line.number);
  }
  const end = state.doc.line(subtreeEnd(state, line.number));
  return end.number > line.number ? { from: line.to, to: end.to } : null;
});

// ---------------------------------------------------------------- styling

class Glyph extends WidgetType {
  constructor(readonly text: string, readonly cls: string) { super(); }
  eq(o: Glyph) { return o.text === this.text && o.cls === this.cls; }
  toDOM() {
    const s = document.createElement("span");
    s.className = this.cls;
    s.textContent = this.text;
    return s;
  }
  ignoreEvent() { return false; }
}

class Img extends WidgetType {
  constructor(readonly src: string) { super(); }
  eq(o: Img) { return o.src === this.src; }
  toDOM() {
    const s = document.createElement("span");
    s.className = "cm-org-img";
    const img = document.createElement("img");
    img.src = this.src;
    img.alt = "";
    s.appendChild(img);
    return s;
  }
}

const BULLETS = ["◉", "○", "◈", "◇", "▸", "•"];
const hide = Decoration.replace({});
const mark = (cls: string) => Decoration.mark({ class: cls });
const line = (cls: string) => Decoration.line({ class: cls });

/** Resolve an image link target relative to the note, for display through the asset protocol. */
function imageSrc(target: string, notePath?: string): string | null {
  if (/^https?:\/\//.test(target)) return IMAGE.test(target) ? target : null;
  const p = target.replace(/^file:/, "");
  if (!IMAGE.test(p) || p.includes("://") || !notePath || !("__TAURI_INTERNALS__" in window)) return null;
  const sep = notePath.includes("\\") ? "\\" : "/";
  const abs = /^([\\/]|[A-Za-z]:)/.test(p) ? p : notePath.replace(/[\\/][^\\/]*$/, "") + sep + p.replace(/^\.\//, "");
  return convertFileSrc(abs);
}

// ---------------------------------------------------------------- code block highlighting

const LANG_ALIAS: Record<string, string> = { elisp: "lisp", "emacs-lisp": "lisp", "c++": "cpp" };
const langs = new Map<string, Language | null | "loading">();
const langLoaded = StateEffect.define<null>();

/** The parser for a src block language, loading it on first use (then VIEW re-renders). */
function langFor(name: string, view: EditorView): Language | null {
  const key = name.toLowerCase();
  const known = langs.get(key);
  if (known !== undefined) return known === "loading" ? null : known;
  const desc = LanguageDescription.matchLanguageName(languages, LANG_ALIAS[key] ?? key, true);
  if (!desc) {
    langs.set(key, null);
    return null;
  }
  if (desc.support) {
    langs.set(key, desc.support.language);
    return desc.support.language;
  }
  langs.set(key, "loading");
  desc.load().then(
    (sup) => { langs.set(key, sup.language); view.dispatch({ effects: langLoaded.of(null) }); },
    () => langs.set(key, null),
  );
  return null;
}

const tokenCache = new Map<string, [number, number, string][]>();

/** Highlight token ranges for CODE, cached by content so cursor moves don't re-parse. */
function codeTokens(lang: Language, code: string): [number, number, string][] {
  const key = `${lang.name}\0${code}`;
  let toks = tokenCache.get(key);
  if (!toks) {
    toks = [];
    const out = toks;
    highlightTree(lang.parser.parse(code), classHighlighter, (from, to, cls) => out.push([from, to, cls]));
    if (tokenCache.size > 300) tokenCache.clear();
    tokenCache.set(key, toks);
  }
  return toks;
}

const markCache = new Map<string, Decoration>();
const tokMark = (cls: string) => markCache.get(cls) ?? (markCache.set(cls, Decoration.mark({ class: cls })), markCache.get(cls)!);

/** Inline markup on one line: links, dates, emphasis, footnotes, cookies. */
function inline(out: Range<Decoration>[], text: string, from: number, active: boolean, conf: OrgConf) {
  const taken: [number, number][] = [];
  const free = (a: number, b: number) => !taken.some(([x, y]) => a < y && b > x);
  for (const m of text.matchAll(LINK)) {
    const s = from + m.index!, e = s + m[0].length;
    taken.push([s, e]);
    const img = !m[2] && imageSrc(m[1], conf.path);
    if (img) out.push(Decoration.widget({ widget: new Img(img), side: 1 }).range(e));
    if (active) { out.push(mark("cm-org-link-raw").range(s, e)); continue; }
    const textFrom = m[2] ? e - 2 - m[2].length : s + 2;
    out.push(hide.range(s, textFrom), mark("cm-org-link").range(textFrom, e - 2), hide.range(e - 2, e));
  }
  for (const m of text.matchAll(TS)) {
    const s = from + m.index!, e = s + m[0].length;
    if (!free(s, e)) continue;
    taken.push([s, e]);
    const cls = m[1] === "<" ? "cm-org-ts" : "cm-org-ts cm-org-ts-inactive";
    if (active) out.push(mark(cls).range(s, e));
    else out.push(hide.range(s, s + 1), mark(cls).range(s + 1, e - 1), hide.range(e - 1, e));
  }
  for (const m of text.matchAll(/\[(\d*%|\d*\/\d*)\]/g)) {
    const s = from + m.index!, e = s + m[0].length;
    if (!free(s, e)) continue;
    taken.push([s, e]);
    const [a, b] = m[1].split("/");
    const complete = m[1].endsWith("%") ? m[1] === "100%" : a !== "" && a === b;
    out.push(mark(`cm-org-cookie${complete ? " cm-org-cookie-done" : ""}`).range(s, e));
  }
  for (const m of text.matchAll(/\[fn:[^\]\s]+\]/g)) {
    const s = from + m.index!, e = s + m[0].length;
    if (free(s, e)) { taken.push([s, e]); out.push(mark("cm-org-fn").range(s, e)); }
  }
  const EM: Record<string, string> = { "*": "b", "/": "i", _: "u", "=": "v", "~": "c", "+": "s" };
  for (const m of text.matchAll(/(^|[\s\-({'"])([*/_=~+])(\S|\S.*?\S)\2(?=[\s\-.,:!?;'")}\]]|$)/g)) {
    const s = from + m.index! + m[1].length, e = s + m[3].length + 2;
    if (!free(s, e)) continue;
    taken.push([s, e]);
    out.push(mark(`cm-org-em-${EM[m[2]]}`).range(s + 1, e - 1));
    if (!active) out.push(hide.range(s, s + 1), hide.range(e - 1, e));
  }
}

function decorate(view: EditorView, blocks: Block[], tables: Tbl[]): DecorationSet {
  const out: Range<Decoration>[] = [];
  const doc = view.state.doc;
  const conf = view.state.facet(orgConf);
  const activeLines = new Set(view.state.selection.ranges.map((r) => doc.lineAt(r.head).number));
  const isKw = (w: string) => conf.todo.includes(w) || conf.done.includes(w);
  const highlighted = new Set<Block>();
  let last = 0;
  for (const vr of view.visibleRanges) {
    for (let pos = vr.from; pos <= vr.to; ) {
      const ln = doc.lineAt(pos);
      pos = ln.to + 1;
      if (ln.number <= last) continue;
      last = ln.number;
      const n = ln.number, t = ln.text, from = ln.from;
      const active = activeLines.has(n);

      const block = blocks.find((b) => n >= b.start && n <= b.end);
      if (block) {
        const code = block.kind === "src" || block.kind === "example";
        const cls = `cm-org-block cm-org-block-${code ? "code" : block.kind}`;
        if (code && block.lang && block.end > block.start + 1 && !highlighted.has(block)) {
          highlighted.add(block);
          const lang = langFor(block.lang, view);
          if (lang) {
            const start = doc.line(block.start + 1).from;
            for (const [a, b, c] of codeTokens(lang, doc.sliceString(start, doc.line(block.end - 1).to))) out.push(tokMark(c).range(start + a, start + b));
          }
        }
        if (n === block.start) {
          out.push(line(`${cls} cm-org-block-begin`).range(from));
          if (!active && t.length) out.push(Decoration.replace({ widget: new Glyph(block.lang || block.kind, "cm-org-block-label") }).range(from, ln.to));
        } else if (n === block.end) {
          out.push(line(`${cls} cm-org-block-end`).range(from));
          if (!active && t.length) out.push(hide.range(from, ln.to));
        } else {
          out.push(line(cls).range(from));
          if (!code) inline(out, t, from, active, conf);
        }
        continue;
      }

      const table = tables.find((x) => n >= x.start && n <= x.end);
      if (table) {
        const head = table.sep && n < table.sep ? " cm-org-table-head" : "";
        out.push(line(`cm-org-table${tbl.isSep(t) ? " cm-org-table-sep" : head}`).range(from));
        for (const m of t.matchAll(/[|+]/g)) out.push(mark("cm-org-pipe").range(from + m.index!, from + m.index! + 1));
        continue;
      }

      const lvl = level(t);
      if (lvl) {
        const words = t.slice(lvl + 1).split(" ");
        const kw = isKw(words[0]) ? words[0] : "";
        const done = conf.done.includes(kw);
        out.push(line(`cm-org-h cm-org-h${Math.min(lvl, 6)}${done ? " cm-org-h-done" : ""}`).range(from));
        if (!active) out.push(Decoration.replace({ widget: new Glyph(BULLETS[(lvl - 1) % BULLETS.length], `cm-org-bullet cm-org-bullet${Math.min(lvl, 6)}`) }).range(from, from + lvl));
        let at = from + lvl + 1;
        if (kw) {
          out.push(mark(`cm-org-kw ${done ? "cm-org-kw-done" : "cm-org-kw-todo"} cm-org-kw-${kw}`).range(at, at + kw.length));
          at += kw.length + 1;
        }
        const pm = /^\[#([A-Z])\]/.exec(doc.sliceString(at, ln.to));
        if (pm) {
          out.push(mark(`cm-org-prio cm-org-prio-${pm[1]}`).range(at, at + 4));
          at += 5;
        }
        let end = ln.to;
        const tm = /\s(:[\w@#%:]+:)\s*$/.exec(t);
        if (tm) {
          const ts = from + tm.index! + 1;
          end = ts;
          let p = ts;
          for (const part of tm[1].split(/(:)/)) {
            if (part === ":") { if (!active) out.push(hide.range(p, p + 1)); }
            else if (part) out.push(mark("cm-org-tag").range(p, p + part.length));
            p += part.length;
          }
        }
        if (done && at < end) out.push(mark("cm-org-done-title").range(at, end));
        if (at < end) inline(out, doc.sliceString(at, end), at, active, conf);
        continue;
      }

      if (/^\s*#\+title:/i.test(t)) {
        out.push(line("cm-org-title").range(from));
        const k = t.indexOf(":") + 1;
        if (!active) out.push(hide.range(from, from + k + (t[k] === " " ? 1 : 0)));
        continue;
      }
      if (/^\s*#\+\w+/.test(t)) { out.push(line("cm-org-meta").range(from)); continue; }
      if (/^\s*#(\s|$)/.test(t)) { out.push(line("cm-org-comment").range(from)); continue; }
      if (/^\s*(SCHEDULED|DEADLINE|CLOSED):/.test(t)) { out.push(line("cm-org-planning").range(from)); inline(out, t, from, active, conf); continue; }
      if (/^\s*:[\w-]+:/.test(t)) { out.push(line("cm-org-drawer").range(from)); continue; }
      if (/^\s*-{5,}\s*$/.test(t)) {
        out.push(line("cm-org-hr").range(from));
        if (!active) out.push(hide.range(from, ln.to));
        continue;
      }

      const item = ITEM.exec(t);
      if (item) {
        const b = from + item[1].length;
        const bullet = item[2];
        if (!active && (bullet === "-" || bullet === "+" || bullet === "*")) {
          out.push(Decoration.replace({ widget: new Glyph("•", "cm-org-li") }).range(b, b + 1));
        } else out.push(mark("cm-org-li-n").range(b, b + bullet.length));
        if (item[4]) {
          const cb = b + bullet.length + item[3].length;
          const st = item[4][1];
          if (st === "X" || st === "x") out.push(line("cm-org-checked").range(from));
          const box = new Glyph(st === " " ? "" : st === "-" ? "–" : "✓", `cm-org-check${st === " " ? "" : " cm-org-check-on"}`);
          if (!active) out.push(Decoration.replace({ widget: box }).range(cb, cb + 3));
          else out.push(mark("cm-org-check-raw").range(cb, cb + 3));
        }
      }
      inline(out, t, from, active, conf);
    }
  }
  return Decoration.set(out, true);
}

const orgStyle = ViewPlugin.fromClass(
  class {
    decorations: DecorationSet;
    blocks: Block[];
    tables: Tbl[];
    constructor(v: EditorView) {
      ({ blocks: this.blocks, tables: this.tables } = scan(v.state));
      this.decorations = decorate(v, this.blocks, this.tables);
    }
    update(u: ViewUpdate) {
      if (u.docChanged) ({ blocks: this.blocks, tables: this.tables } = scan(u.state));
      const loaded = u.transactions.some((tr) => tr.effects.some((e) => e.is(langLoaded)));
      if (u.docChanged || u.viewportChanged || u.selectionSet || loaded) this.decorations = decorate(u.view, this.blocks, this.tables);
    }
  },
  { decorations: (v) => v.decorations },
);

export function linkAt(state: EditorState, pos: number): string | null {
  const ln = state.doc.lineAt(pos);
  const col = pos - ln.from;
  for (const m of ln.text.matchAll(LINK)) {
    if (m.index! <= col && col <= m.index! + m[0].length) return m[1];
  }
  return null;
}

export const linkAtCursor = (v: EditorView) => linkAt(v.state, v.state.selection.main.head);

/** Ctrl/Cmd-click follows links; clicking a checkbox toggles it. */
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
    const ln = v.state.doc.lineAt(pos);
    const m = CHECKBOX.exec(ln.text);
    const box = m ? ln.from + m[1].length - 1 : -1;
    const onWidget = (e.target as HTMLElement).classList?.contains("cm-org-check");
    if (!m || v.state.readOnly || (!onWidget && (pos < box || pos > box + 3))) return false;
    e.preventDefault();
    v.dispatch({ changes: { from: box + 1, to: box + 2, insert: m[2] === " " ? "X" : " " } });
    updateCookies(v, ln.number);
    return true;
  },
});

// ---------------------------------------------------------------- editing ops

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

/** Org's "<s" + Tab: expand a block template. */
function expandTemplate(v: EditorView): boolean {
  const ln = curLine(v);
  const m = /^(\s*)<([sqevc])$/.exec(ln.text);
  if (!m || v.state.selection.main.head !== ln.to) return false;
  const kind = { s: "src", q: "quote", e: "example", v: "verse", c: "center" }[m[2]]!;
  insertBlock(v, kind, "", { from: ln.from, to: ln.to, indent: m[1] });
  return true;
}

export function insertBlock(v: EditorView, kind: string, lang = "", at?: { from: number; to: number; indent: string }) {
  const ln = curLine(v);
  const indent = at?.indent ?? /^\s*/.exec(ln.text)![0];
  const from = at?.from ?? (ln.text.trim() ? ln.to : ln.from);
  const lead = !at && ln.text.trim() ? "\n" : "";
  const begin = `${indent}#+begin_${kind}${kind === "src" ? " " + lang : ""}`;
  const insert = `${lead}${begin}\n${indent}\n${indent}#+end_${kind}`;
  const cursor = kind === "src" && !lang ? from + lead.length + begin.length : from + lead.length + begin.length + 1 + indent.length;
  v.dispatch({ changes: { from, to: at?.to ?? from, insert }, selection: { anchor: cursor } });
  insertMode(v);
}

export function orgTab(v: EditorView): boolean {
  if (tableOp(v, "next") || tableOp(v, "recalc")) return true;
  if (expandTemplate(v)) return true;
  const t = curLine(v).text;
  if (!level(t) && !DRAWER.test(t) && !/^\s*#\+begin_/i.test(t)) return false;
  return toggleFold(v);
}

export function orgShiftTab(v: EditorView): boolean {
  if (tableOp(v, "prev")) return true;
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
    const ln = s.doc.line(i);
    if (level(ln.text)) changes.push(d > 0 ? { from: ln.from, insert: "*" } : { from: ln.from, to: ln.from + 1 });
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

/** Continue a list (new item after this line); otherwise a new heading. */
export function newItem(v: EditorView): boolean {
  const ln = curLine(v);
  const m = ITEM.exec(ln.text);
  if (!m || level(ln.text)) return newHeading(v);
  const bullet = /\d/.test(m[2]) ? parseInt(m[2]) + 1 + m[2].slice(-1) : m[2];
  const insert = `\n${m[1]}${bullet}${m[3]}${m[4] ? "[ ] " : ""}`;
  v.dispatch({ changes: { from: ln.to, insert }, selection: { anchor: ln.to + insert.length }, scrollIntoView: true });
  insertMode(v);
  return true;
}

/** Enter in insert mode: next table row, or continue / end a list. */
function smartEnter(v: EditorView): boolean {
  if (tableOp(v, "enter")) return true;
  const ln = curLine(v);
  const m = ITEM.exec(ln.text);
  if (!m || level(ln.text) || v.state.selection.main.head !== ln.to) return false;
  if (m[0].length === ln.text.length) {
    v.dispatch({ changes: { from: ln.from, to: ln.to, insert: "" } });
    return true;
  }
  return newItem(v);
}

export function toggleHeading(v: EditorView): boolean {
  const ln = curLine(v);
  const lvl = level(ln.text);
  v.dispatch(lvl ? { changes: { from: ln.from, to: ln.from + lvl + 1 } } : { changes: { from: ln.from, insert: "* " } });
  return true;
}

export function toggleCheckbox(v: EditorView): boolean {
  const ln = curLine(v);
  const m = CHECKBOX.exec(ln.text);
  if (!m) {
    // Turn a list item into a checkbox item.
    const it = ITEM.exec(ln.text);
    if (!it || level(ln.text)) return false;
    v.dispatch({ changes: { from: ln.from + it[0].length, insert: "[ ] " } });
    return true;
  }
  const pos = ln.from + m[1].length;
  v.dispatch({ changes: { from: pos, to: pos + 1, insert: m[2] === " " ? "X" : " " } });
  updateCookies(v, ln.number);
  return true;
}

/** Refresh the [n/m] / [n%] cookie of heading H: counts the checkboxes in
 *  its body, or else its child tasks. */
function updateCookie(v: EditorView, h: number) {
  const s = v.state;
  const conf = s.facet(orgConf);
  const hl = s.doc.line(h);
  const lvl = level(hl.text);
  const cm = /\[(\d*%|\d*\/\d*)\]/.exec(hl.text);
  if (!cm) return;
  let done = 0, total = 0;
  const end = subtreeEnd(s, h);
  for (let i = h + 1; i <= end; i++) {
    const t = s.doc.line(i).text;
    if (level(t)) break;
    const c = CHECKBOX.exec(t);
    if (c) { total++; if (/[xX]/.test(c[2])) done++; }
  }
  if (!total) {
    for (let i = h + 1; i <= end; i++) {
      const t = s.doc.line(i).text;
      if (level(t) !== lvl + 1) continue;
      const kw = t.slice(lvl + 1).split(" ")[0];
      if (conf.todo.includes(kw)) total++;
      else if (conf.done.includes(kw)) { total++; done++; }
    }
  }
  const val = cm[1].includes("%") ? `${total ? Math.round((done * 100) / total) : 0}%` : `${done}/${total}`;
  const from = hl.from + cm.index + 1;
  if (val !== cm[1]) v.dispatch({ changes: { from, to: from + cm[1].length, insert: val } });
}

/** Update the progress cookies of the heading owning line N and of its parent. */
export function updateCookies(v: EditorView, n = curLine(v).number) {
  let h = headingAt(v.state, n);
  for (let round = 0; h && round < 2; round++) {
    updateCookie(v, h);
    const lvl = level(v.state.doc.line(h).text);
    let p = headingAt(v.state, h - 1);
    while (p && level(v.state.doc.line(p).text) >= lvl) p = headingAt(v.state, p - 1);
    h = p;
  }
}

const pad = (n: number) => String(n).padStart(2, "0");

/** Nudge the timestamp under the cursor. With FIELD, the part under the
 *  cursor (year/month/day/hour/5 minutes) changes; otherwise the day. */
export function shiftTimestamp(v: EditorView, dir: number, field = true): boolean {
  const ln = curLine(v);
  const head = v.state.selection.main.head;
  const col = head - ln.from;
  for (const m of ln.text.matchAll(TS)) {
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
    const from = ln.from + m.index!;
    v.dispatch({ changes: { from, to: from + m[0].length, insert: s }, selection: { anchor: head } });
    return true;
  }
  return false;
}

// ---------------------------------------------------------------- tables

export const inTable = (v: EditorView) => tbl.isTableLine(curLine(v).text);
const TBLFM = /^\s*#\+TBLFM:\s*/i;

type TableOp = "next" | "prev" | "enter" | "align" | "recalc" | "rowBelow" | "rowAbove" | "delRow" | "colRight" | "delCol"
  | "rowDown" | "rowUp" | "colLeft" | "colRightMove" | "sep" | "sort" | "sortDesc";
/** Ops that recalculate #+TBLFM formulas, like moving through cells in a spreadsheet. */
const RECALC = new Set<TableOp>(["next", "prev", "enter", "align", "recalc"]);

/** Run a table edit at the cursor (also "recalc" from the #+TBLFM line under a table).
 *  Returns false when the cursor is not in a table. */
export function tableOp(v: EditorView, op: TableOp): boolean {
  const s = v.state;
  let ln = curLine(v);
  const onFormula = op === "recalc" && TBLFM.test(ln.text) && ln.number > 1 && tbl.isTableLine(s.doc.line(ln.number - 1).text);
  if (onFormula) ln = s.doc.line(ln.number - 1);
  else if (!tbl.isTableLine(ln.text)) return false;
  let a = ln.number, b = ln.number;
  while (a > 1 && tbl.isTableLine(s.doc.line(a - 1).text)) a--;
  while (b < s.doc.lines && tbl.isTableLine(s.doc.line(b + 1).text)) b++;
  const fmLine = b < s.doc.lines && TBLFM.test(s.doc.line(b + 1).text) ? b + 1 : 0;
  let fm = fmLine ? s.doc.line(fmLine).text.replace(TBLFM, "").trim() : "";
  const lines = Array.from({ length: b - a + 1 }, (_, i) => s.doc.line(a + i).text);
  const col = onFormula ? 0 : tbl.cellIndex(ln.text, s.selection.main.head - ln.from);
  const t: tbl.Table = { lines, row: ln.number - a, col };
  const ops: Record<TableOp, () => tbl.Table | null> = {
    next: () => tbl.nextCell(t, 1), prev: () => tbl.nextCell(t, -1), enter: () => tbl.nextRow(t),
    align: () => t, recalc: () => t, rowBelow: () => tbl.insertRow(t, true), rowAbove: () => tbl.insertRow(t, false),
    delRow: () => tbl.deleteRow(t), colRight: () => tbl.insertCol(t), delCol: () => tbl.deleteCol(t),
    rowDown: () => tbl.moveRow(t, 1), rowUp: () => tbl.moveRow(t, -1), colLeft: () => tbl.moveCol(t, -1), colRightMove: () => tbl.moveCol(t, 1),
    sep: () => tbl.insertSep(t), sort: () => tbl.sortRows(t), sortDesc: () => tbl.sortRows(t, true),
  };
  const r = ops[op]();
  if (!r) return true;
  // Cells typed as =expr / :=expr become formulas; then recalculate and align.
  const ex = tbl.extractCellFormulas(r.lines);
  if (ex.formulas.length) fm = tbl.mergeTblfm(fm, ex.formulas);
  const out = tbl.align(fm && (RECALC.has(op) || ex.formulas.length) ? tbl.applyFormulas(ex.lines, fm) : ex.lines);
  const from = s.doc.line(a).from;
  const text = out.join("\n") + (fm ? `\n#+TBLFM: ${fm}` : "");
  let off = 0;
  for (let i = 0; i < r.row; i++) off += out[i].length + 1;
  const cell = tbl.isSep(out[r.row]) ? 1 : tbl.cellOffset(out[r.row], r.col);
  const anchor = onFormula ? s.selection.main.head : from + off + cell;
  const to = s.doc.line(fmLine || b).to;
  const change = { from, to, insert: text };
  // Keep the cursor on the formula line when recalculating from there.
  const mapped = onFormula ? from + text.length - (to - anchor) : anchor;
  v.dispatch({ changes: change, selection: { anchor: Math.min(mapped, from + text.length) }, scrollIntoView: true });
  return true;
}

export function insertTable(v: EditorView, cols: number, rows: number) {
  const ln = curLine(v);
  const lines = tbl.create(cols, rows);
  const lead = ln.text.trim() ? "\n" : "";
  const from = ln.text.trim() ? ln.to : ln.from;
  v.dispatch({ changes: { from, to: ln.text.trim() ? from : ln.to, insert: lead + lines.join("\n") }, selection: { anchor: from + lead.length + 2 } });
  insertMode(v);
}

/** All headings in the doc, for a "go to heading" picker. */
export function headings(state: EditorState) {
  const out: { line: number; level: number; text: string }[] = [];
  for (let i = 1; i <= state.doc.lines; i++) {
    const t = state.doc.line(i).text;
    const l = level(t);
    if (l) out.push({ line: i - 1, level: l, text: t.slice(l + 1).replace(/\s+:[\w@#%:]+:\s*$/, "").replace(/\[\[(?:[^\]]+)\]\[([^\]]+)\]\]/g, "$1") });
  }
  return out;
}

export function insertText(v: EditorView, text: string, at = v.state.selection.main.head) {
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
  ".cm-scroller": { fontFamily: "var(--editor-font)", lineHeight: "1.65" },
  ".cm-content": { padding: "24px 0 40vh", maxWidth: "88ch", margin: "0 auto", caretColor: "var(--accent)" },
  ".cm-line": { padding: "0 36px" },
  ".cm-activeLine": { backgroundColor: "var(--active)" },
  ".cm-foldPlaceholder": { background: "var(--active)", border: "none", color: "var(--dim)", padding: "0 6px", borderRadius: "4px", margin: "0 4px" },
  "&.cm-focused .cm-selectionBackground, .cm-selectionBackground": { backgroundColor: "var(--sel) !important" },
  ".cm-fat-cursor": { background: "var(--accent) !important", color: "var(--bg) !important" },
  "&:not(.cm-focused) .cm-fat-cursor": { outline: "1px solid var(--accent)", background: "none !important" },
  ".cm-panels": { backgroundColor: "var(--panel)", color: "var(--fg)", fontFamily: "var(--mono)" },
  ".cm-vim-panel input": { color: "var(--fg)", fontFamily: "var(--mono)" },

  // headings
  ".cm-org-h": { fontWeight: "650", paddingTop: "0.35em" },
  ".cm-org-h1": { fontSize: "1.5em", color: "var(--h1)" },
  ".cm-org-h2": { fontSize: "1.28em", color: "var(--h2)" },
  ".cm-org-h3": { fontSize: "1.12em", color: "var(--h3)" },
  ".cm-org-h4": { color: "var(--h4)" },
  ".cm-org-h5": { color: "var(--h5)" },
  ".cm-org-h6": { color: "var(--h6)" },
  ".cm-org-h-done": { color: "var(--dim)" },
  ".cm-org-done-title": { textDecoration: "line-through", textDecorationColor: "color-mix(in srgb, var(--dim) 60%, transparent)" },
  ".cm-org-bullet": { display: "inline-block", width: "1.1em", color: "var(--dim)", fontSize: "0.8em", verticalAlign: "0.1em", textDecoration: "none" },
  ".cm-org-kw": { fontSize: "0.68em", fontWeight: "700", letterSpacing: "0.04em", padding: "0.15em 0.5em", borderRadius: "4px", verticalAlign: "0.18em", fontFamily: "var(--sans)", textDecoration: "none", display: "inline-block" },
  ".cm-org-kw-todo": { background: "color-mix(in srgb, var(--todo) 18%, transparent)", color: "var(--todo)" },
  ".cm-org-kw-NEXT": { background: "color-mix(in srgb, var(--link) 18%, transparent)", color: "var(--link)" },
  ".cm-org-kw-WAIT": { background: "color-mix(in srgb, var(--h5) 18%, transparent)", color: "var(--h5)" },
  ".cm-org-kw-done": { background: "color-mix(in srgb, var(--done) 18%, transparent)", color: "var(--done)" },
  ".cm-org-prio": { fontSize: "0.68em", fontWeight: "700", color: "var(--dim)", border: "1px solid var(--border)", borderRadius: "4px", padding: "0 0.35em", verticalAlign: "0.18em", fontFamily: "var(--sans)", display: "inline-block", textDecoration: "none" },
  ".cm-org-prio-A": { color: "var(--todo)", borderColor: "color-mix(in srgb, var(--todo) 45%, transparent)" },
  ".cm-org-prio-B": { color: "var(--h5)", borderColor: "color-mix(in srgb, var(--h5) 45%, transparent)" },
  ".cm-org-tag": { fontSize: "0.68em", fontWeight: "500", color: "var(--dim)", background: "var(--active)", borderRadius: "999px", padding: "0.1em 0.6em", marginLeft: "0.3em", verticalAlign: "0.18em", fontFamily: "var(--sans)", display: "inline-block" },
  ".cm-org-cookie": { fontSize: "0.75em", color: "var(--dim)", fontFamily: "var(--mono)" },
  ".cm-org-cookie-done": { color: "var(--done)" },

  // inline
  ".cm-org-link": { color: "var(--link)", textDecoration: "underline", textUnderlineOffset: "3px", textDecorationColor: "color-mix(in srgb, var(--link) 45%, transparent)", cursor: "pointer" },
  ".cm-org-link-raw": { color: "var(--link)" },
  ".cm-org-ts": { fontFamily: "var(--mono)", fontSize: "0.82em", color: "var(--date)", background: "color-mix(in srgb, var(--date) 12%, transparent)", borderRadius: "4px", padding: "0.1em 0.35em" },
  ".cm-org-ts-inactive": { color: "var(--dim)", background: "var(--active)" },
  ".cm-org-fn": { color: "var(--link)", fontSize: "0.8em", verticalAlign: "0.3em" },
  ".cm-org-em-b": { fontWeight: "700" },
  ".cm-org-em-i": { fontStyle: "italic" },
  ".cm-org-em-u": { textDecoration: "underline" },
  ".cm-org-em-s": { textDecoration: "line-through", color: "var(--dim)" },
  ".cm-org-em-v, .cm-org-em-c": { fontFamily: "var(--mono)", fontSize: "0.88em", background: "var(--active)", color: "var(--code)", borderRadius: "4px", padding: "0.1em 0.3em" },
  ".cm-org-img img": { display: "block", maxWidth: "100%", maxHeight: "420px", borderRadius: "8px", margin: "8px 0" },

  // lists
  ".cm-org-li": { color: "var(--accent)", fontWeight: "700" },
  ".cm-org-li-n": { color: "var(--accent)" },
  ".cm-org-check": { display: "inline-grid", placeItems: "center", width: "0.95em", height: "0.95em", border: "1.5px solid var(--dim)", borderRadius: "4px", verticalAlign: "-0.12em", fontSize: "0.8em", lineHeight: "1", cursor: "pointer", boxSizing: "border-box", marginRight: "0.1em" },
  ".cm-org-check-on": { background: "var(--done)", borderColor: "var(--done)", color: "var(--bg)", fontWeight: "700" },
  ".cm-org-check-raw": { color: "var(--date)", fontFamily: "var(--mono)" },
  ".cm-org-checked": { color: "var(--dim)" },

  // lines
  ".cm-org-title": { fontSize: "1.9em", fontWeight: "750", color: "var(--fg)", paddingTop: "0.2em", paddingBottom: "0.3em", lineHeight: "1.25" },
  ".cm-org-meta, .cm-org-comment, .cm-org-drawer, .cm-org-planning": { color: "var(--dim)", fontSize: "0.85em" },
  ".cm-org-drawer, .cm-org-planning": { fontFamily: "var(--mono)" },
  ".cm-org-comment": { fontStyle: "italic" },
  ".cm-org-hr": { backgroundImage: "linear-gradient(var(--border), var(--border))", backgroundSize: "calc(100% - 72px) 1px", backgroundRepeat: "no-repeat", backgroundPosition: "center" },

  // blocks
  ".cm-org-block": { background: "var(--panel)", borderLeft: "3px solid var(--border)" },
  ".cm-org-block-code": { fontFamily: "var(--mono)", fontSize: "0.88em" },
  ".cm-org-block-quote, .cm-org-block-verse": { fontStyle: "italic", borderLeftColor: "var(--accent)", color: "color-mix(in srgb, var(--fg) 85%, var(--dim))" },
  ".cm-org-block-begin": { borderTopRightRadius: "6px", paddingTop: "0.2em", fontSize: "0.85em", color: "var(--dim)", fontFamily: "var(--mono)" },
  ".cm-org-block-end": { borderBottomRightRadius: "6px", fontSize: "0.85em", color: "var(--dim)", fontFamily: "var(--mono)" },
  ".cm-org-block-label": { fontSize: "0.85em", color: "var(--dim)", textTransform: "lowercase" },
  ".cm-org-block-code .tok-keyword, .cm-org-block-code .tok-operatorKeyword, .cm-org-block-code .tok-controlKeyword": { color: "var(--kw)" },
  ".cm-org-block-code .tok-string, .cm-org-block-code .tok-string2": { color: "var(--done)" },
  ".cm-org-block-code .tok-comment": { color: "var(--dim)", fontStyle: "italic" },
  ".cm-org-block-code .tok-number, .cm-org-block-code .tok-bool, .cm-org-block-code .tok-atom": { color: "var(--h6)" },
  ".cm-org-block-code .tok-typeName, .cm-org-block-code .tok-className, .cm-org-block-code .tok-namespace": { color: "var(--h5)" },
  ".cm-org-block-code .tok-definition, .cm-org-block-code .tok-macroName": { color: "var(--link)" },
  ".cm-org-block-code .tok-propertyName, .cm-org-block-code .tok-labelName": { color: "var(--h3)" },
  ".cm-org-block-code .tok-variableName2, .cm-org-block-code .tok-meta": { color: "var(--todo)" },
  ".cm-org-block-code .tok-operator, .cm-org-block-code .tok-punctuation": { color: "color-mix(in srgb, var(--fg) 75%, var(--dim))" },
  ".cm-org-block-code .tok-invalid": { color: "var(--todo)", textDecoration: "underline wavy" },

  // tables
  ".cm-org-table": { fontFamily: "var(--mono)", fontSize: "0.9em" },
  ".cm-org-table-head": { fontWeight: "700" },
  ".cm-org-table-sep": { color: "var(--border)" },
  ".cm-org-pipe": { color: "var(--border)" },
});

export function createState(doc: string, opts: { org: boolean; readOnly: boolean; todo: string[]; done: string[]; path?: string; onChange: () => void }) {
  const ext: Extension[] = [
    vim({ status: true }),
    history(),
    drawSelection(),
    highlightActiveLine(),
    highlightSelectionMatches(),
    EditorView.lineWrapping,
    codeFolding({ placeholderText: "⋯" }),
    keymap.of([...defaultKeymap, ...historyKeymap, ...searchKeymap, indentWithTab]),
    theme,
    EditorState.readOnly.of(opts.readOnly),
    EditorView.updateListener.of((u) => { if (u.docChanged) opts.onChange(); }),
  ];
  if (opts.org) {
    ext.push(
      orgConf.of({ todo: opts.todo, done: opts.done, path: opts.path }), orgStyle, orgFold, clicks,
      Prec.high(keymap.of([{ key: "Tab", run: orgTab }, { key: "Shift-Tab", run: orgShiftTab }, { key: "Enter", run: smartEnter }])),
    );
  } else ext.push(EditorView.contentAttributes.of({ class: "cm-plain" }));
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
