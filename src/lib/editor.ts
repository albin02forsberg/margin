// CodeMirror 6 setup: vim, org styling (decorations that hide markup except on
// the line being edited), folding, tables, blocks and the structure-editing ops.
import { Compartment, EditorState, Facet, Prec, StateEffect, type Extension, type Range, type TransactionSpec } from "@codemirror/state";
import {
  Decoration, EditorView, GutterMarker, ViewPlugin, WidgetType, drawSelection, gutter, highlightActiveLine, keymap, lineNumbers, showPanel, type DecorationSet, type Panel, type ViewUpdate,
} from "@codemirror/view";
import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
import { searchKeymap, highlightSelectionMatches } from "@codemirror/search";
import { acceptCompletion, autocompletion, closeCompletion, completionStatus, moveCompletionSelection, type CompletionContext, type CompletionResult } from "@codemirror/autocomplete";
import { codeFolding, foldService, foldAll, unfoldAll, toggleFold, foldedRanges, foldEffect, LanguageDescription, type Language } from "@codemirror/language";
import { languages } from "@codemirror/language-data";
import { highlightTree, classHighlighter } from "@lezer/highlight";
import { vim, Vim, getCM } from "@replit/codemirror-vim";
import { convertFileSrc } from "@tauri-apps/api/core";
import * as tbl from "./orgtable";
import { renumber, itemSwap } from "./list";
import { setArg, type NumOpts } from "./setopt";
import { changeList, changeStep } from "./changelist";
import { EMPH, toggleEmphasis } from "./emphasis";
import { FN, addFootnote, footnoteTarget } from "./footnote";
import { headingAt, level, narrowField, subtreeEnd } from "./narrow";
import { transclusion, transcludeTarget } from "./transclude";
import { completionAt } from "./links";
import { diagrams } from "./diagram";
export { directive as transcludeLine } from "./transclude";
import { block, expandSnippet, snippetAt, snippetPrompts, type Snippet } from "./snippet";
export type { Snippet } from "./snippet";

/** Set by the app; vim ex commands, mappings and clicks call through these. */
export const hooks = {
  save: () => {},
  saveAll: () => {},
  /** FORCE (`:q!`, `:bd!`) drops unsaved edits instead of saving them. */
  close: (_force?: boolean) => {},
  /** `:qa[!]`: close every tab (FORCE drops unsaved edits). */
  closeAll: (_force?: boolean) => {},
  /** `:b NAME|N`: switch to the open tab NAME or number N picks. */
  buffer: (_name: string) => {},
  /** `:bd[!] NAME`: close the open tab NAME picks (no NAME: the current one, via `close`). */
  closeTab: (_name: string, _force: boolean) => {},
  /** `:ls`: pick an open tab from a list. */
  listTabs: () => {},
  /** `:b #` / `<C-^>`: switch to the previously shown tab. */
  alternate: () => {},
  /** `:e[!] [NAME]`: open the note NAME, or reload the current one from disk. */
  edit: (_name: string, _force: boolean) => {},
  /** `:sp` / `:vs [NAME]`: open the note NAME (none: a picker) in the right pane. */
  split: (_name: string) => {},
  /** `:new` / `:vnew [TITLE]`: create a note and open it in the right pane. */
  newSplit: (_title: string) => {},
  /** `:only`: close the split, keeping the focused pane. */
  only: () => {},
  /** `:close`: close the focused pane. */
  closePane: () => {},
  /** `:sav NAME`: save the open note and rename it to NAME. */
  saveAs: (_name: string) => {},
  /** `:help`: open the tutorial. */
  help: () => {},
  follow: (_target?: string) => {},
  tab: (_d: number) => {},
  /** The user's `[[snippets]]` (config.toml), expanded by `<key` + Tab. */
  snippets: (): Snippet[] => [],
  /** Ask for a snippet's `%^{Name}` (null = cancelled). */
  ask: async (_name: string): Promise<string | null> => null,
  /** Inline completion candidates (#899): notes and headings with the `[[…]]` LINK to insert, people (no `@`) and tags. */
  completions: async (): Promise<{ links: { label: string; detail: string; link: string }[]; people: string[]; tags: string[] }> => ({ links: [], people: [], tags: [] }),
};

/** Ask every prompt of snippet TEXT in turn; null when one is cancelled. */
async function askAll(text: string): Promise<Record<string, string> | null> {
  const answers: Record<string, string> = {};
  for (const n of snippetPrompts(text)) { const a = await hooks.ask(n); if (a == null) return null; answers[n] = a; }
  return answers;
}

type ExP = { args?: string[]; argString?: string };
/** An ex command's `!` and the rest of its argument (`:e! name` → [true, "name"]). */
const bang = (p: ExP): [boolean, string] => { const s = (p.argString ?? "").trim(); return [s.startsWith("!"), s.replace(/^!\s*/, "")]; };
Vim.defineEx("write", "w", () => hooks.save());
Vim.defineEx("quit", "q", (_cm: unknown, p: ExP) => hooks.close(bang(p)[0]));
Vim.defineEx("wq", "wq", () => { hooks.save(); hooks.close(); });
Vim.defineEx("xit", "x", () => { hooks.save(); hooks.close(); });
Vim.defineEx("wall", "wa", () => hooks.saveAll());
Vim.defineEx("bnext", "bn", () => hooks.tab(1));
Vim.defineEx("bprevious", "bp", () => hooks.tab(-1));
Vim.defineEx("bdelete", "bd", (_cm: unknown, p: ExP) => { const [f, n] = bang(p); n ? hooks.closeTab(n, f) : hooks.close(f); });
for (const [name, alias] of [["ls", "ls"], ["buffers", "buffers"], ["files", "files"]]) Vim.defineEx(name, alias, () => hooks.listTabs());
Vim.defineEx("qall", "qa", (_cm: unknown, p: ExP) => hooks.closeAll(bang(p)[0]));
Vim.defineEx("wqall", "wqa", () => { hooks.saveAll(); hooks.closeAll(); });
Vim.defineEx("xall", "xa", () => { hooks.saveAll(); hooks.closeAll(); });
Vim.defineEx("buffer", "b", (_cm: unknown, p: ExP) => hooks.buffer(p.argString ?? ""));
Vim.defineEx("edit", "e", (_cm: unknown, p: ExP) => { const [f, n] = bang(p); hooks.edit(n, f); });
// Split view (#874) has one side-by-side layout, so `:sp` and `:vs` are the same.
for (const [name, alias] of [["split", "sp"], ["vsplit", "vs"]]) Vim.defineEx(name, alias, (_cm: unknown, p: ExP) => hooks.split((p.argString ?? "").trim()));
for (const name of ["new", "vnew"]) Vim.defineEx(name, name, (_cm: unknown, p: ExP) => hooks.newSplit((p.argString ?? "").trim()));
Vim.defineEx("only", "on", () => hooks.only());
Vim.defineEx("close", "clo", () => hooks.closePane());
Vim.defineEx("saveas", "sav", (_cm: unknown, p: ExP) => hooks.saveAs((p.argString ?? "").trim()));
Vim.defineEx("help", "h", () => hooks.help());

// `:set number` / `relativenumber` / `wrap`: session-wide vim options, a compartment per view.
// Global only: the callback ignores per-editor (cm) calls, so codemirror-vim keeps no stale local copies.
const vimOpts = { number: false, relativenumber: false, wrap: true };
for (const [name, alias] of [["number", "nu"], ["relativenumber", "rnu"], ["wrap", "wrap"]] as const)
  Vim.defineOption(name, undefined, "boolean", [alias], (v?: boolean, cm?: unknown) => {
    if (cm) return undefined;
    if (v === undefined) return vimOpts[name];
    vimOpts[name] = v;
  });
const numConf = new Compartment();
const wrapConf = new Compartment();
const wrapExt = () => (vimOpts.wrap ? EditorView.lineWrapping : []);
const views = new Set<EditorView>();
// `spellcheck` setting: org notes only; one session-wide flag, a compartment per view.
let spellOn = false;
const spellConf = new Compartment();
const spellExt = () => (spellOn ? EditorView.contentAttributes.of({ spellcheck: "true" }) : []);
export function setSpellcheck(on: boolean) {
  if (on === spellOn) return;
  spellOn = on;
  for (const v of views) v.dispatch({ effects: spellConf.reconfigure(spellExt()) });
}
class Num extends GutterMarker {
  constructor(readonly n: string) { super(); }
  eq(o: Num) { return o.n === this.n; }
  toDOM() { return document.createTextNode(this.n); }
}
function numExt(o: NumOpts): Extension {
  if (!o.relativenumber) return o.number ? lineNumbers() : [];
  return gutter({
    class: "cm-lineNumbers",
    lineMarker(v, line) {
      const cur = v.state.doc.lineAt(v.state.selection.main.head).number, n = v.state.doc.lineAt(line.from).number;
      return new Num(String(n === cur ? (o.number ? n : 0) : Math.abs(n - cur)));
    },
    lineMarkerChange: (u) => u.selectionSet || u.docChanged,
    initialSpacer: () => new Num("9999"),
  });
}
const trackViews = ViewPlugin.fromClass(class {
  constructor(readonly v: EditorView) { views.add(v); }
  destroy() { views.delete(this.v); }
});
// Replaces codemirror-vim's `:set` (one arg only) so `:set nu rnu` works; its own options (`tw`, …) still go through setOption.
Vim.defineEx("set", "se", (cm: any, p: ExP) => {
  if (!p.args) return say(cm, "Argument required");
  for (const a of p.args) {
    const r = setArg(a, (n) => Vim.getOption(n, cm));
    const err = "set" in r ? Vim.setOption(r.set[0], r.set[1], cm) : undefined;
    if ("msg" in r || err) { say(cm, "msg" in r ? r.msg : err!.message); break; }
  }
  for (const v of views) v.dispatch({ effects: [numConf.reconfigure(numExt(vimOpts)), wrapConf.reconfigure(wrapExt())] });
});
/** Show MSG in the vim status bar, like vim's own messages. */
function say(cm: { openNotification?: (n: Node, o: object) => unknown }, msg: string) {
  const el = Object.assign(document.createElement("div"), { className: "cm-vim-message", textContent: msg });
  el.style.whiteSpace = "pre";
  cm.openNotification?.(el, { bottom: true, duration: 5000 });
}
Vim.defineAction("orgFollow", () => hooks.follow());
Vim.mapCommand("gf", "action", "orgFollow", {}, { context: "normal" });
Vim.defineAction("nextTab", () => hooks.tab(1));
Vim.defineAction("prevTab", () => hooks.tab(-1));
Vim.mapCommand("gt", "action", "nextTab", {}, { context: "normal" });
Vim.mapCommand("gT", "action", "prevTab", {}, { context: "normal" });
Vim.defineAction("altTab", () => hooks.alternate());
for (const k of ["<C-^>", "<C-6>"]) Vim.mapCommand(k, "action", "altTab", {}, { context: "normal" });
// `g;` / `g,`: walk the change list (see changelist.ts); the count steps that many entries.
for (const [keys, d] of [["g;", -1], ["g,", 1]] as const) {
  Vim.defineAction(`change${d}`, (cm: any, a: { repeat?: number }) => {
    const v: EditorView = cm.cm6, r = changeStep(v.state, d * (a.repeat ?? 1));
    if ("msg" in r) return say(cm, r.msg);
    v.dispatch({ selection: { anchor: r.pos }, effects: r.effect, scrollIntoView: true });
  });
  Vim.mapCommand(keys, "action", `change${d}`, {}, { context: "normal" });
}

type OrgConf = { todo: string[]; done: string[]; path?: string };
const orgConf = Facet.define<OrgConf, OrgConf>({ combine: (v) => v[0] ?? { todo: [], done: [] } });

// ---------------------------------------------------------------- document structure

export { level, narrowToggle, narrowedTo, revealEffects } from "./narrow";
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
  for (const m of text.matchAll(FN)) {
    const s = from + m.index!, e = s + m[0].length;
    if (free(s, e)) { taken.push([s, e]); out.push(mark("cm-org-fn").range(s, e)); }
  }
  const EM: Record<string, string> = { "*": "b", "/": "i", _: "u", "=": "v", "~": "c", "+": "s" };
  for (const m of text.matchAll(EMPH)) {
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
  const t = transcludeTarget(ln.text); // anywhere on a #+TRANSCLUDE: line, so gf jumps to the source
  if (t) return t;
  const col = pos - ln.from;
  for (const m of ln.text.matchAll(LINK)) {
    if (m.index! <= col && col <= m.index! + m[0].length) return m[1];
  }
  return null;
}

export const linkAtCursor = (v: EditorView) => linkAt(v.state, v.state.selection.main.head);

/** Follow the footnote at POS (reference ⇄ definition) as a vim jump, so Ctrl+O
 *  returns; null when POS isn't on one, else its label, whether it's the
 *  definition and whether it moved. */
export function followFootnote(v: EditorView, pos = v.state.selection.main.head) {
  const t = footnoteTarget(v.state.doc.toString(), pos);
  if (!t) return null;
  if (t.to == null) return { label: t.label, def: t.def, moved: false };
  recordJump(v);
  v.dispatch({ selection: { anchor: t.to }, effects: EditorView.scrollIntoView(t.to, { y: "center" }) });
  return { label: t.label, def: t.def, moved: true };
}

/** Put the cursor on vim's jump list (Ctrl+O comes back to it). The list is global
 *  and keeps marks from other tabs' documents, which can be out of range here. */
function recordJump(v: EditorView) {
  const cm = getCM(v) as any;
  try { if (cm) Vim.getVimGlobalState_().jumpList.add(cm, cm.getCursor(), cm.getCursor()); } catch { /* stale mark */ }
}

/** Forget vim's jump list after a tab switch (`setState` swaps the document under its marks, and `G`
 *  then throws on a stale one). Registers, histories and the rest of vim's global state stay. */
export function vimClearJumps() {
  const keep = { ...Vim.getVimGlobalState_() };
  Vim.resetVimGlobalState_();
  const g = Vim.getVimGlobalState_();
  Object.assign(g, keep, { jumpList: g.jumpList });
}

/** Insert the next numbered footnote at the cursor and type its definition under `* Footnotes`. */
export function insertFootnote(v: EditorView) {
  const r = addFootnote(v.state.doc.toString(), v.state.selection.main.head);
  const cm = getCM(v) as any;
  if (cm?.state.vim?.visualMode) Vim.handleKey(cm, "<Esc>", "mapping");
  recordJump(v);
  v.dispatch({ changes: r.changes, selection: { anchor: r.cursor }, effects: EditorView.scrollIntoView(r.cursor, { y: "center" }) });
  insertMode(v);
  return true;
}

/** Ctrl/Cmd-click follows links; clicking a checkbox toggles it. */
const clicks = EditorView.domEventHandlers({
  mousedown(e, v) {
    const pos = v.posAtCoords({ x: e.clientX, y: e.clientY });
    if (pos == null) return false;
    if (e.ctrlKey || e.metaKey) {
      const target = linkAt(v.state, pos);
      const fn = !target && footnoteTarget(v.state.doc.toString(), pos);
      if (!target && !fn) return false;
      e.preventDefault();
      if (fn) v.dispatch({ selection: { anchor: pos } }); // the app follows the footnote at the cursor
      hooks.follow(target ?? undefined);
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

import { diffChange } from "./merge";
export { diffChange, mergeEdits, perKey } from "./merge";

/** Replace the doc with TEXT as one minimal change, keeping cursor and undo sane. */
export function applyText(view: EditorView, text: string) {
  const old = view.state.doc.toString();
  if (old !== text) view.dispatch({ changes: diffChange(old, text) });
}

/** Org's "<s" + Tab (org-tempo): expand a block template or a user snippet (hooks.snippets). */
function expandTemplate(v: EditorView): boolean {
  const ln = curLine(v);
  const x = v.state.selection.main.head === ln.to && snippetAt(ln.text, hooks.snippets());
  if (!x) return false;
  if ("ask" in x) { // prompts: answer first, then expand if the line still reads `<key`
    const { from, to, text } = ln, { text: src, indent } = x;
    void askAll(src).then((a) => {
      if (!a || v.state.doc.sliceString(from, to) !== text) return v.focus();
      const y = expandSnippet(src, indent, new Date(), a);
      v.dispatch({ changes: { from, to, insert: y.text }, selection: { anchor: from + y.cursor } });
      v.focus(); insertMode(v);
    });
    return true;
  }
  v.dispatch({ changes: { from: ln.from, to: ln.to, insert: x.text }, selection: { anchor: ln.from + x.cursor } });
  insertMode(v);
  return true;
}

/** Insert user snippet S at the cursor, later lines indented like the cursor line (palette "Insert snippet…"). */
export async function insertSnippet(v: EditorView, s: Snippet) {
  const answers = await askAll(s.text);
  if (!answers) return;
  const { from, to } = v.state.selection.main;
  const indent = /^\s*/.exec(curLine(v).text)![0];
  const x = expandSnippet(s.text, indent, new Date(), answers); // its first line's indent is already there
  v.dispatch({ changes: { from, to, insert: x.text.slice(indent.length) }, selection: { anchor: from + x.cursor - indent.length } });
  insertMode(v);
}

export function insertBlock(v: EditorView, kind: string, lang = "") {
  const ln = curLine(v);
  const lead = ln.text.trim() ? "\n" : "";
  const from = lead ? ln.to : ln.from;
  const x = block(lang ? `${kind} ${lang}` : kind, /^\s*/.exec(ln.text)![0]);
  v.dispatch({ changes: { from, insert: lead + x.text }, selection: { anchor: from + lead.length + x.cursor } });
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

/** Swap the list item at the cursor (with its nested lines) with its next/previous
 *  sibling, renumbering ordered lists. false off an item line (a continuation line
 *  included) so the caller can fall back to moveSubtree; true (no-op) at either end. */
export function moveItem(v: EditorView, d: -1 | 1): boolean {
  const s = v.state, ln = curLine(v);
  const r = itemSwap((i) => (i >= 1 && i <= s.doc.lines ? s.doc.line(i).text : undefined), ln.number, d);
  if (r === null) return false;
  if (r === "end") return true;
  const [a, b] = r;
  const slice = (x: [number, number]) => s.sliceDoc(s.doc.line(x[0]).from, s.doc.line(x[1]).to);
  const from = s.doc.line(a[0]).from;
  const gap = s.sliceDoc(s.doc.line(a[1]).to, s.doc.line(b[0]).from);
  const insert = slice(b) + gap + slice(a);
  const head = (d > 0 ? from + slice(b).length + gap.length : from) + s.selection.main.head - ln.from;
  withRenumber(v, { changes: { from, to: s.doc.line(b[1]).to, insert }, selection: { anchor: head } }, a[0]);
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

/** Dispatch SPEC plus, in the same transaction (one undo), the renumbering of
 *  the ordered list at line N of the resulting doc. */
function withRenumber(v: EditorView, spec: TransactionSpec, n: number) {
  const doc = v.state.update(spec).state.doc;
  const changes = renumber((i) => (i >= 1 && i <= doc.lines ? doc.line(i).text : undefined), n)
    .map((c) => ({ from: doc.line(c.line).from + c.from, to: doc.line(c.line).from + c.to, insert: c.insert }));
  v.dispatch(spec, { changes, sequential: true });
}

/** Renumber the ordered list at the cursor (after vim moves / deletes items). */
export function renumberList(v: EditorView): boolean {
  withRenumber(v, {}, curLine(v).number);
  return true;
}

/** Continue a list (new item after this line); otherwise a new heading. */
export function newItem(v: EditorView): boolean {
  const ln = curLine(v);
  const m = ITEM.exec(ln.text);
  if (!m || level(ln.text)) return newHeading(v);
  const num = /\d/.test(m[2]);
  const bullet = num ? parseInt(m[2]) + 1 + m[2].slice(-1) : m[2];
  const insert = `\n${m[1]}${bullet}${m[3]}${m[4] ? "[ ] " : ""}`;
  const spec = { changes: { from: ln.to, insert }, selection: { anchor: ln.to + insert.length }, scrollIntoView: true };
  if (num) withRenumber(v, spec, ln.number + 1);
  else v.dispatch(spec);
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
    // Ending a list in its middle: the items below move up a number.
    withRenumber(v, { changes: { from: ln.from, to: ln.to, insert: "" } }, ln.number + 1);
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

type TableOp = "next" | "prev" | "enter" | "align" | "recalc" | "rowBelow" | "rowAbove" | "delRow" | "colRight" | "colBefore" | "delCol"
  | "rowDown" | "rowUp" | "colLeft" | "colRightMove" | "sep" | "sort" | "sortDesc";
/** Ops that recalculate #+TBLFM formulas, like moving through cells in a spreadsheet. */
const RECALC = new Set<TableOp>(["next", "prev", "enter", "align", "recalc"]);

/** Run a table edit at the cursor (also "recalc" from the #+TBLFM line under a table).
 *  Returns false when the cursor is not in a table. */
/** The table at the cursor (or above its #+TBLFM line with FORMULA): first/last line,
 *  the #+TBLFM line (0 when none) and its formulas, the cursor's row line. */
function findTable(s: EditorState, formula: boolean) {
  let ln = s.doc.lineAt(s.selection.main.head);
  const onFormula = formula && TBLFM.test(ln.text) && ln.number > 1 && tbl.isTableLine(s.doc.line(ln.number - 1).text);
  if (onFormula) ln = s.doc.line(ln.number - 1);
  else if (!tbl.isTableLine(ln.text)) return null;
  let a = ln.number, b = ln.number;
  while (a > 1 && tbl.isTableLine(s.doc.line(a - 1).text)) a--;
  while (b < s.doc.lines && tbl.isTableLine(s.doc.line(b + 1).text)) b++;
  const fmLine = b < s.doc.lines && TBLFM.test(s.doc.line(b + 1).text) ? b + 1 : 0;
  const fm = fmLine ? s.doc.line(fmLine).text.replace(TBLFM, "").trim() : "";
  const lines = Array.from({ length: b - a + 1 }, (_, i) => s.doc.line(a + i).text);
  return { a, b, fmLine, fm, lines, ln, onFormula };
}

export function tableOp(v: EditorView, op: TableOp): boolean {
  const s = v.state;
  const found = findTable(s, op === "recalc");
  if (!found) return false;
  const { a, b, fmLine, lines, ln, onFormula } = found;
  let fm = found.fm;
  const col = onFormula ? 0 : tbl.cellIndex(ln.text, s.selection.main.head - ln.from);
  const t: tbl.Table = { lines, row: ln.number - a, col };
  const ops: Record<TableOp, () => tbl.Table | null> = {
    next: () => tbl.nextCell(t, 1), prev: () => tbl.nextCell(t, -1), enter: () => tbl.nextRow(t),
    align: () => t, recalc: () => t, rowBelow: () => tbl.insertRow(t, true), rowAbove: () => tbl.insertRow(t, false),
    delRow: () => tbl.deleteRow(t), colRight: () => tbl.insertCol(t), colBefore: () => tbl.insertCol(t, true), delCol: () => tbl.deleteCol(t),
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

/** For the formula dialog: the table at the cursor (cell `=expr` formulas moved into
 *  its TBLFM) and the cursor's 1-based data row and column, or null outside a table. */
export function tableFormulas(v: EditorView) {
  const f = findTable(v.state, true);
  if (!f) return null;
  const ex = tbl.extractCellFormulas(f.lines);
  const row = f.onFormula ? 1 : f.lines.slice(0, f.ln.number - f.a + 1).filter((l) => !tbl.isSep(l)).length;
  const col = f.onFormula ? 1 : tbl.cellIndex(f.ln.text, v.state.selection.main.head - f.ln.from) + 1;
  return { lines: ex.lines, fm: tbl.mergeTblfm(f.fm, ex.formulas), row: Math.max(1, row), col };
}

/** Replace the cursor's table formulas with FM, then recalculate and align. */
export function setTableFormulas(v: EditorView, fm: string): boolean {
  const f = findTable(v.state, true);
  if (!f) return false;
  const lines = tbl.extractCellFormulas(f.lines).lines;
  const text = tbl.align(fm ? tbl.applyFormulas(lines, fm) : lines).join("\n") + (fm ? `\n#+TBLFM: ${fm}` : "");
  const from = v.state.doc.line(f.a).from, to = v.state.doc.line(f.fmLine || f.b).to;
  const head = Math.min(v.state.selection.main.head, from + text.length);
  v.dispatch({ changes: { from, to, insert: text }, selection: { anchor: head } });
  return true;
}

/** Set by the page: opens the formula dialog (the toolbar's "Formulas…" button). */
export const tableHooks: { formulas?: () => void } = {};

/** Table toolbar buttons: [label, tooltip, op]; "|" draws a divider. */
const TABLE_TOOLS: ([string, string, TableOp] | "|")[] = [
  ["+ Row ↑", "Insert row above (Space T k)", "rowAbove"], ["+ Row ↓", "Insert row below (Space T j, Alt+Shift+J)", "rowBelow"],
  ["− Row", "Delete row (Space T d, Alt+Shift+K)", "delRow"], ["Row ↑", "Move row up (Alt+K)", "rowUp"], ["Row ↓", "Move row down (Alt+J)", "rowDown"], "|",
  ["+ Col ←", "Insert column left (Space T H)", "colBefore"], ["+ Col →", "Insert column right (Space T l)", "colRight"],
  ["− Col", "Delete column (Space T h)", "delCol"], ["Col ←", "Move column left (Alt+H)", "colLeft"], ["Col →", "Move column right (Alt+L)", "colRightMove"], "|",
  ["Sort ↑", "Sort rows by this column (Space T s)", "sort"], ["Sort ↓", "Sort rows by this column, descending (Space T S)", "sortDesc"],
  ["Separator", "Add a separator line below (Space T -)", "sep"], ["Align", "Align the table (Space T a)", "align"], ["Recalc", "Recalculate #+TBLFM formulas (Space T r)", "recalc"],
];

function tablePanel(v: EditorView): Panel {
  const dom = document.createElement("div");
  dom.className = "cm-table-tools";
  dom.setAttribute("role", "toolbar");
  dom.setAttribute("aria-label", "Table");
  dom.append(Object.assign(document.createElement("span"), { textContent: "Table" }));
  for (const t of TABLE_TOOLS) {
    if (t === "|") { dom.append(document.createElement("i")); continue; }
    const b = Object.assign(document.createElement("button"), { type: "button", textContent: t[0], title: t[1] });
    b.dataset.op = t[2];
    b.onmousedown = (e) => e.preventDefault(); // keep the editor's focus, selection and vim mode
    b.onclick = () => { tableOp(v, t[2]); v.focus(); };
    dom.append(b);
  }
  const f = Object.assign(document.createElement("button"), { type: "button", textContent: "Formulas…", title: "List, add and edit #+TBLFM formulas (Space T f)" });
  f.dataset.op = "formulas";
  f.onmousedown = (e) => e.preventDefault();
  f.onclick = () => tableHooks.formulas?.();
  dom.append(f);
  return { dom };
}

/** The table toolbar, under the editor while the cursor is in a table. */
const tableTools = showPanel.compute(["doc", "selection"], (s) =>
  !s.readOnly && tbl.isTableLine(s.doc.lineAt(s.selection.main.head).text) ? tablePanel : null);

export function insertTable(v: EditorView, cols: number, rows: number, header = true) {
  const ln = curLine(v);
  const lines = tbl.create(cols, rows, header);
  const lead = ln.text.trim() ? "\n" : "";
  const from = ln.text.trim() ? ln.to : ln.from;
  v.dispatch({ changes: { from, to: ln.text.trim() ? from : ln.to, insert: lead + lines.join("\n") }, selection: { anchor: from + lead.length + 2 } });
  insertMode(v);
}

/** Replace [FROM, TO) with table LINES on lines of their own, as one undoable change. */
function putTable(v: EditorView, from: number, to: number, lines: string[]) {
  const d = v.state.doc;
  const lead = d.sliceString(d.lineAt(from).from, from).trim() ? "\n" : "";
  const trail = d.sliceString(to, d.lineAt(to).to).trim() ? "\n" : "";
  replaceRange(v, from, to, lead + lines.join("\n") + trail);
}

/** Paste TEXT (spreadsheet TSV) as a table over the selection; false when the cursor is in a table or it isn't TSV. */
export function pasteTable(v: EditorView, text: string): boolean {
  const lines = !inTable(v) && tbl.isTsv(text) && tbl.fromDelimited(text, "\t");
  if (!lines) return false;
  const { from, to } = v.state.selection.main;
  putTable(v, from, to, lines);
  return true;
}

/** Turn the CSV / TSV on the selected lines into a table; false when it doesn't parse. */
export function convertToTable(v: EditorView): boolean {
  const { from, to } = v.state.selection.main;
  const a = v.state.doc.lineAt(from), b = v.state.doc.lineAt(to > from && v.state.doc.lineAt(to).from === to ? to - 1 : to); // vim's V ends at the next line's start
  const lines = tbl.fromDelimited(v.state.sliceDoc(a.from, b.to));
  if (!lines) return false;
  putTable(v, a.from, b.to, lines);
  return true;
}

/** The table at the cursor as CSV, or null outside a table. */
export const tableCsv = (v: EditorView) => { const f = findTable(v.state, true); return f && tbl.toCsv(f.lines); };

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

/** The non-empty, single-line selection ({from, to, text}), else null. */
export function lineSelection(v: EditorView) {
  const { from, to } = v.state.selection.main;
  const text = v.state.sliceDoc(from, to);
  return text.trim() && !text.includes("\n") ? { from, to, text } : null;
}

/** Replace [FROM, TO) with TEXT, cursor after it, leaving vim visual mode. */
export function replaceRange(v: EditorView, from: number, to: number, text: string) {
  const cm = getCM(v) as any;
  if (cm?.state.vim?.visualMode) Vim.handleKey(cm, "<Esc>", "mapping");
  v.dispatch({ changes: { from, to, insert: text }, selection: { anchor: from + text.length } });
}

/** Bold / italic / … (MARK) the selection or the word at the cursor, or remove
 *  the mark around it; an empty pair puts the cursor between in insert mode. */
export function emphasis(v: EditorView, mark: string): boolean {
  const s = v.state;
  const { from, to } = s.selection.main;
  const at = s.doc.lineAt(from).from;
  const r = toggleEmphasis(s.sliceDoc(at, s.doc.lineAt(to).to), from - at, to - at, mark);
  const cm = getCM(v) as any;
  const visual = cm?.state.vim?.visualMode;
  if (visual) Vim.handleKey(cm, "<Esc>", "mapping");
  const changes = r.changes.map((c) => ({ from: c.from + at, to: c.to + at, insert: c.insert }));
  v.dispatch({ changes, selection: { anchor: r.from + at, head: (visual ? r.from : r.to) + at } });
  if (r.empty) insertMode(v);
  return true;
}

const vimInsert = (v: EditorView) => !!(getCM(v) as any)?.state.vim?.insertMode;

export function insertMode(v: EditorView) {
  const cm = getCM(v) as any;
  if (cm && !cm.state.vim?.insertMode) Vim.handleKey(cm, "i", "mapping");
}

/** True when vim is idle in normal mode (no pending operator/count/keys). */
export function vimIdle(v: EditorView): boolean {
  const vs = (getCM(v) as any)?.state?.vim;
  return !!vs && !vs.insertMode && !vs.visualMode && !vs.inputState.operator && vs.inputState.keyBuffer.length === 0;
}

const watched = new WeakSet<object>();
/** Call F on vim mode changes of V; registers once per vim instance (setState makes a new one) however often it is called. */
export function onModeChange(v: EditorView, f: (mode: string) => void) {
  const cm = getCM(v) as any;
  if (!cm || watched.has(cm)) return;
  watched.add(cm);
  cm.on("vim-mode-change", (e: { mode: string; subMode?: string }) => f(e.subMode ? `${e.mode} ${e.subMode}` : e.mode));
}

/** The vim mode string of V as the status bar shows it ("normal", "insert", "visual", "visual linewise"). */
export function vimMode(v: EditorView): string {
  const vs = (getCM(v) as any)?.state?.vim;
  return !vs ? "normal" : vs.insertMode ? "insert" : vs.visualMode ? (vs.visualLine ? "visual linewise" : vs.visualBlock ? "visual blockwise" : "visual") : "normal";
}

// ---------------------------------------------------------------- completion

/** `[[` notes and headings, `@` people, `:` tags on a heading line, as you type (#899). A link replaces the `[[` and a `]]` right after it. */
async function complete(cx: CompletionContext): Promise<CompletionResult | null> {
  const ln = cx.state.doc.lineAt(cx.pos), at = completionAt(cx.state.sliceDoc(ln.from, cx.pos), !!level(ln.text));
  if (!at) return null;
  const from = cx.pos - at.query.length, c = await hooks.completions();
  if (cx.aborted) return null;
  if (at.kind === "person") return { from, validFor: /^[\w.-]*$/, options: c.people.map((label) => ({ label })) };
  if (at.kind === "tag") return { from, validFor: /^[\w@#%]*$/, options: c.tags.map((label) => ({ label, apply: `${label}:` })) };
  return {
    from, validFor: /^[^[\]\n]*$/, // no newline: an unclosed `[[` must not keep its result alive on the next lines
    options: c.links.map(({ label, detail, link }) => ({
      label, detail,
      apply: (v: EditorView, _: unknown, f: number, to: number) => {
        const end = v.state.sliceDoc(to, to + 2) === "]]" ? to + 2 : to;
        v.dispatch({ changes: { from: f - 2, to: end, insert: link }, selection: { anchor: f - 2 + link.length }, userEvent: "input.complete" });
      },
    })),
  };
}

const completeKeys: Record<string, (v: EditorView) => boolean> = {
  Enter: acceptCompletion, Tab: acceptCompletion, Escape: closeCompletion,
  ArrowDown: moveCompletionSelection(true), ArrowUp: moveCompletionSelection(false), PageDown: moveCompletionSelection(true, "page"), PageUp: moveCompletionSelection(false, "page"),
};
/** The popup's keys while it is open. A plugin handles keydown before vim's plugin does, so Esc closes the popup and stays in insert mode. */
const completion = [
  autocompletion({ override: [complete], defaultKeymap: false, icons: false }),
  Prec.highest(ViewPlugin.define(() => ({}), {
    eventHandlers: {
      keydown: (e, v) => {
        const f = completionStatus(v.state) === "active" && !e.ctrlKey && !e.metaKey && !e.altKey && !e.shiftKey && completeKeys[e.key];
        return !!f && f(v);
      },
    },
  })),
];

// ---------------------------------------------------------------- state

const theme = EditorView.theme({
  "&": { height: "100%", fontSize: "var(--fs-lg)", backgroundColor: "var(--bg)", color: "var(--fg)" },
  ".cm-scroller": { fontFamily: "var(--editor-font)", lineHeight: "1.65" },
  ".cm-content": { padding: "24px 0 40vh", maxWidth: "72ch", margin: "0 auto", caretColor: "var(--accent)" },
  ".cm-line": { padding: "0 36px" },
  ".cm-activeLine": { backgroundColor: "var(--active)" },
  ".cm-tooltip.cm-tooltip-autocomplete": { background: "var(--panel)", color: "var(--fg)", border: "1px solid var(--border)", borderRadius: "var(--radius)", boxShadow: "var(--soft)", fontFamily: "var(--sans)", fontSize: "var(--fs-md)" },
  ".cm-tooltip-autocomplete ul li[aria-selected]": { background: "var(--sel) !important", color: "var(--fg) !important" },
  ".cm-completionDetail": { color: "var(--dim)", fontStyle: "normal", marginLeft: "1em" },
  ".cm-completionMatchedText": { textDecoration: "none", color: "var(--accent)" },
  ".cm-foldPlaceholder": { background: "var(--active)", border: "none", color: "var(--dim)", padding: "0 6px", borderRadius: "var(--radius-sm)", margin: "0 4px" },
  "&.cm-focused .cm-selectionBackground, .cm-selectionBackground": { backgroundColor: "var(--sel) !important" },
  ".cm-fat-cursor": { background: "var(--accent) !important", color: "var(--bg) !important" },
  "&:not(.cm-focused) .cm-fat-cursor": { outline: "1px solid var(--accent)", background: "none !important" },
  // the Vim status line: part of the card (CodeMirror's light default draws a #ddd edge and its own fill)
  ".cm-panels": { backgroundColor: "transparent", color: "var(--fg)", fontFamily: "var(--mono)" },
  ".cm-panels.cm-panels-bottom": { borderTop: "1px solid var(--border)" },
  ".cm-vim-panel": { padding: "2px 12px" },
  ".cm-vim-panel input": { color: "var(--fg)", fontFamily: "var(--mono)" },
  ".cm-table-tools": { display: "flex", flexWrap: "wrap", gap: "4px", alignItems: "center", padding: "4px 12px", fontFamily: "var(--sans)" },
  ".cm-table-tools > span": { color: "var(--dim)", fontSize: "var(--fs-xs)", textTransform: "uppercase", letterSpacing: "0.05em", marginRight: "4px" },
  ".cm-table-tools > i": { width: "1px", alignSelf: "stretch", background: "var(--border)", margin: "0 4px" },
  ".cm-table-tools button": { background: "var(--bg)", border: "1px solid var(--border)", color: "var(--fg)", borderRadius: "var(--radius-sm)", padding: "1px 7px", font: "var(--fs-sm) var(--sans)", cursor: "pointer" },
  ".cm-table-tools button:hover": { borderColor: "var(--accent)", color: "var(--accent)" },

  // headings
  ".cm-org-h": { fontWeight: "650", paddingTop: "0.9em", paddingBottom: "0.1em" },
  ".cm-org-h:first-child": { paddingTop: "0.35em" },
  ".cm-org-h1": { fontSize: "var(--fs-h1)", color: "var(--h1)" },
  ".cm-org-h2": { fontSize: "var(--fs-h2)", color: "var(--h2)" },
  ".cm-org-h3": { fontSize: "var(--fs-h3)", color: "var(--h3)" },
  ".cm-org-h4": { color: "var(--h4)" },
  ".cm-org-h5": { color: "var(--h5)" },
  ".cm-org-h6": { color: "var(--h6)" },
  ".cm-org-h-done": { color: "var(--dim)" },
  ".cm-org-done-title": { textDecoration: "line-through", textDecorationColor: "color-mix(in srgb, var(--dim) 60%, transparent)" },
  ".cm-org-bullet": { display: "inline-block", width: "1.1em", color: "var(--dim)", fontSize: "0.8em", verticalAlign: "0.1em", textDecoration: "none" },
  ".cm-org-kw": { fontSize: "0.68em", fontWeight: "700", letterSpacing: "0.04em", padding: "0.15em 0.5em", borderRadius: "var(--radius-sm)", verticalAlign: "0.18em", fontFamily: "var(--sans)", textDecoration: "none", display: "inline-block" },
  ".cm-org-kw-todo": { background: "color-mix(in srgb, var(--todo) 18%, transparent)", color: "var(--todo)" },
  ".cm-org-kw-NEXT": { background: "color-mix(in srgb, var(--link) 18%, transparent)", color: "var(--link)" },
  ".cm-org-kw-WAIT": { background: "color-mix(in srgb, var(--h5) 18%, transparent)", color: "var(--h5)" },
  ".cm-org-kw-done": { background: "color-mix(in srgb, var(--done) 18%, transparent)", color: "var(--done)" },
  ".cm-org-prio": { fontSize: "0.68em", fontWeight: "700", color: "var(--dim)", border: "1px solid var(--border)", borderRadius: "var(--radius-sm)", padding: "0 0.35em", verticalAlign: "0.18em", fontFamily: "var(--sans)", display: "inline-block", textDecoration: "none" },
  ".cm-org-prio-A": { color: "var(--todo)", borderColor: "color-mix(in srgb, var(--todo) 45%, transparent)" },
  ".cm-org-prio-B": { color: "var(--h5)", borderColor: "color-mix(in srgb, var(--h5) 45%, transparent)" },
  ".cm-org-tag": { fontSize: "0.68em", fontWeight: "500", color: "var(--dim)", background: "var(--active)", borderRadius: "999px", padding: "0.1em 0.6em", marginLeft: "0.3em", verticalAlign: "0.18em", fontFamily: "var(--sans)", display: "inline-block" },
  ".cm-org-cookie": { fontSize: "0.75em", color: "var(--dim)", fontFamily: "var(--mono)" },
  ".cm-org-cookie-done": { color: "var(--done)" },

  // inline
  ".cm-org-link": { color: "var(--link)", textDecoration: "underline", textUnderlineOffset: "3px", textDecorationColor: "color-mix(in srgb, var(--link) 45%, transparent)", cursor: "pointer" },
  ".cm-org-link-raw": { color: "var(--link)" },
  ".cm-org-ts": { fontFamily: "var(--mono)", fontSize: "0.82em", color: "var(--date)", background: "color-mix(in srgb, var(--date) 12%, transparent)", borderRadius: "var(--radius-sm)", padding: "0.1em 0.35em" },
  ".cm-org-ts-inactive": { color: "var(--dim)", background: "var(--active)" },
  ".cm-org-fn": { color: "var(--link)", fontSize: "0.8em", verticalAlign: "0.3em" },
  ".cm-org-em-b": { fontWeight: "700" },
  ".cm-org-em-i": { fontStyle: "italic" },
  ".cm-org-em-u": { textDecoration: "underline" },
  ".cm-org-em-s": { textDecoration: "line-through", color: "var(--dim)" },
  ".cm-org-em-v, .cm-org-em-c": { fontFamily: "var(--mono)", fontSize: "0.88em", background: "var(--active)", color: "var(--code)", borderRadius: "var(--radius-sm)", padding: "0.1em 0.3em" },
  ".cm-org-img img": { display: "block", maxWidth: "100%", maxHeight: "420px", borderRadius: "8px", margin: "8px 0" },

  // lists
  ".cm-org-li": { color: "var(--accent)", fontWeight: "700" },
  ".cm-org-li-n": { color: "var(--accent)" },
  ".cm-org-check": { display: "inline-grid", placeItems: "center", width: "0.95em", height: "0.95em", border: "1.5px solid var(--dim)", borderRadius: "var(--radius-sm)", verticalAlign: "-0.12em", fontSize: "0.8em", lineHeight: "1", cursor: "pointer", boxSizing: "border-box", marginRight: "0.1em" },
  ".cm-org-check-on": { background: "var(--done)", borderColor: "var(--done)", color: "var(--bg)", fontWeight: "700" },
  ".cm-org-check-raw": { color: "var(--date)", fontFamily: "var(--mono)" },
  ".cm-org-checked": { color: "var(--dim)" },

  // lines
  ".cm-org-title": { fontSize: "var(--fs-title)", fontWeight: "750", color: "var(--fg)", paddingTop: "0.2em", paddingBottom: "0.3em", lineHeight: "1.25" },
  ".cm-org-meta, .cm-org-comment, .cm-org-drawer, .cm-org-planning": { color: "var(--dim)", fontSize: "0.85em" },
  ".cm-org-drawer, .cm-org-planning": { fontFamily: "var(--mono)" },
  ".cm-org-comment": { fontStyle: "italic" },
  ".cm-org-hr": { backgroundImage: "linear-gradient(var(--border), var(--border))", backgroundSize: "calc(100% - 72px) 1px", backgroundRepeat: "no-repeat", backgroundPosition: "center" },

  // blocks
  ".cm-org-block": { background: "var(--panel)", borderLeft: "3px solid var(--border)" },
  ".cm-org-block-code": { fontFamily: "var(--mono)", fontSize: "0.88em" },
  ".cm-org-block-quote, .cm-org-block-verse": { fontStyle: "italic", borderLeftColor: "var(--accent)", color: "color-mix(in srgb, var(--fg) 85%, var(--dim))" },
  ".cm-org-block-begin": { borderTopRightRadius: "var(--radius)", paddingTop: "0.2em", fontSize: "0.85em", color: "var(--dim)", fontFamily: "var(--mono)" },
  ".cm-org-block-end": { borderBottomRightRadius: "var(--radius)", fontSize: "0.85em", color: "var(--dim)", fontFamily: "var(--mono)" },
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

/** A read-only, org-styled state for the link peek panel (#849): no vim, so ex commands there can't act on the open tab. */
export const peekState = (doc: string, opts: { todo: string[]; done: string[]; path?: string }) =>
  EditorState.create({ doc, extensions: [theme, orgConf.of(opts), orgStyle, EditorView.lineWrapping, EditorState.readOnly.of(true)] });

export function createState(doc: string, opts: { org: boolean; readOnly: boolean; todo: string[]; done: string[]; path?: string; onChange: () => void; onUpdate?: (s: EditorState, docChanged: boolean) => void }) {
  const ext: Extension[] = [
    vim({ status: true }),
    numConf.of(numExt(vimOpts)),
    trackViews,
    history(),
    changeList,
    drawSelection(),
    highlightActiveLine(),
    highlightSelectionMatches(),
    wrapConf.of(wrapExt()),
    codeFolding({ placeholderText: "⋯" }),
    keymap.of([...defaultKeymap, ...historyKeymap, ...searchKeymap, indentWithTab]),
    theme,
    EditorState.readOnly.of(opts.readOnly),
    EditorView.updateListener.of((u) => {
      if (u.docChanged) opts.onChange();
      if (u.docChanged || u.selectionSet) opts.onUpdate?.(u.state, u.docChanged);
    }),
  ];
  if (opts.org) {
    ext.push(
      spellConf.of(spellExt()), orgConf.of({ todo: opts.todo, done: opts.done, path: opts.path }), orgStyle, orgFold, clicks, tableTools, narrowField, transclusion(opts.path, (t) => hooks.follow(t)), diagrams, completion,
      Prec.high(keymap.of([{ key: "Tab", run: orgTab }, { key: "Shift-Tab", run: orgShiftTab }, { key: "Enter", run: smartEnter },
        // Insert mode only: in normal / visual mode Ctrl+B stays vim's page up.
        { key: "Mod-b", run: (v) => vimInsert(v) && emphasis(v, "*") }, { key: "Mod-i", run: (v) => vimInsert(v) && emphasis(v, "/") }])),
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
