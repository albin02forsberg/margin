// Code blocks run on demand (Ctrl+Enter / Space i r, like org-babel): find the block, write #+RESULTS: under it.

/** A `#+begin_src LANG … #+end_src` block: its language, code (org's `,*` / `,#+` escapes undone),
 *  indentation and the offsets of the start of its `#+begin_src` line and the end of its `#+end_src` line. */
export type Src = { lang: string; code: string; indent: string; from: number; end: number };

type Line = { text: string; from: number; to: number };
function lines(text: string): Line[] {
  const out: Line[] = [];
  let from = 0;
  for (const t of text.split("\n")) { out.push({ text: t, from, to: from + t.length }); from += t.length + 1; }
  return out;
}

const BEGIN = /^(\s*)#\+begin_src(?:[ \t]+(\S+))?/i, END = /^\s*#\+end_src\b/i;

/** Every closed src block in TEXT, in order. */
export function srcBlocks(text: string): Src[] {
  const ls = lines(text), out: Src[] = [];
  for (let i = 0; i < ls.length; i++) {
    const b = BEGIN.exec(ls[i].text);
    if (!b) continue;
    let j = i + 1;
    while (j < ls.length && !END.test(ls[j].text)) j++;
    if (j === ls.length) break; // unclosed: nothing after it is a block either
    const code = ls.slice(i + 1, j).map((l) => l.text.replace(/^(\s*),(?=\*|#\+)/, "$1")).join("\n");
    out.push({ lang: b[2] ?? "", code, indent: b[1], from: ls[i].from, end: ls[j].to });
    i = j;
  }
  return out;
}

/** A diagram's code as the key of its exported SVG (#942), the same as `export::diagram_key` in Rust: each line
 *  trimmed and its `,*` / `,#+` escape undone, blank lines dropped, so indentation and trailing newlines don't matter. */
export const diagramKey = (code: string) => code.split("\n").map((l) => l.trim().replace(/^,(?=\*|#\+)/, "")).filter(Boolean).join("\n");

/** The src block around offset POS (its begin and end lines count), else null. */
export const srcAt = (text: string, pos: number): Src | null => srcBlocks(text).find((s) => pos >= s.from && pos <= s.end) ?? null;

/** Index after the existing result starting at line J: `: ` lines, a begin/end block, a `:RESULTS:` drawer,
 *  `|` table lines or one file link. An unclosed block or drawer consumes nothing. */
function resultEnd(ls: Line[], j: number): number {
  const t = ls[j]?.text ?? "";
  if (/^\s*:( |$)/.test(t)) { while (j < ls.length && /^\s*:( |$)/.test(ls[j].text)) j++; return j; }
  const blk = /^\s*#\+begin_(example|src|export)\b/i.exec(t);
  const close = blk ? new RegExp(`^\\s*#\\+end_${blk[1]}\\b`, "i") : /^\s*:RESULTS:\s*$/i.test(t) ? /^\s*:END:\s*$/i : null;
  if (close) { for (let k = j + 1; k < ls.length; k++) if (close.test(ls[k].text)) return k + 1; return j; }
  if (/^\s*\|/.test(t)) { while (j < ls.length && /^\s*\|/.test(ls[j].text)) j++; return j; }
  return /^\s*\[\[file:[^\]]*\]\]\s*$/i.test(t) ? j + 1 : j;
}

const RESULTS = /^\s*#\+RESULTS(\[[^\]]*\])?:/i;

/** LS with their common leading whitespace removed (blank lines don't count). */
const dedent = (ls: string[]) => {
  const ind = (l: string) => /^\s*/.exec(l)![0].length, cut = Math.min(...ls.filter((l) => l.trim()).map(ind));
  return ls.map((l) => l.slice(Math.min(cut, ind(l))));
};

/** The text of the result in lines I..J-1: `: ` prefixes, a block's or drawer's begin/end lines and indentation removed. */
function resultBody(ls: Line[], i: number, j: number): string {
  let b = ls.slice(i, j).map((l) => l.text);
  if (/^\s*:( |$)/.test(b[0] ?? "")) return b.map((t) => t.replace(/^\s*:( |$)/, "")).join("\n");
  if (/^\s*(#\+begin_|:RESULTS:\s*$)/i.test(b[0] ?? "")) b = b.slice(1, -1);
  return dedent(b).join("\n");
}

/** The `#+RESULTS:` line (and its result) right after the line ending at END, blank lines skipped, else null. */
function resultsAt(text: string, end: number): { from: number; to: number; body: string } | null {
  if (end >= text.length) return null;
  const base = end + 1, ls = lines(text.slice(base));
  let i = 0;
  while (i < ls.length && !ls[i].text.trim()) i++;
  if (i === ls.length || !RESULTS.test(ls[i].text)) return null;
  const j = resultEnd(ls, i + 1);
  return { from: base + ls[i].from, to: base + ls[j - 1].to, body: resultBody(ls, i + 1, j) };
}

/** The edit that puts OUTPUT under block SRC (or a `#+CALL:` line) as `#+RESULTS:` + `: ` lines (org's verbatim
 *  form, which no output can close early), replacing the `#+RESULTS:` (and its result) that follows, if any.
 *  NAME gives org's named form `#+RESULTS: name`, which `:var` reads. */
export function resultsChange(text: string, src: Pick<Src, "indent" | "end">, output: string, name = ""): { from: number; to: number; insert: string } {
  const body = output.replace(/\n+$/, "");
  const res = `${src.indent}#+RESULTS:${name ? " " + name : ""}` + (body ? "\n" + body.split("\n").map((l) => (l ? `${src.indent}: ${l}` : `${src.indent}:`)).join("\n") : "");
  const r = resultsAt(text, src.end);
  return r ? { from: r.from, to: r.to, insert: res } : { from: src.end, to: src.end, insert: "\n\n" + res };
}

/** Header arguments of a `#+begin_src LANG :key value …` line: lowercased keys (without `:`), values joined by
 *  single spaces, double quotes around a value removed (`:shebang "#!/bin/sh -e"`; `x="a b"` stays one value);
 *  a key with no value maps to ""; repeated `:var`s are joined. */
export function headerArgs(line: string): Record<string, string> {
  const out: Record<string, string> = {};
  let key = "";
  for (const t of line.replace(BEGIN, "").match(/(?:[^\s"]*"[^"]*")+[^\s"]*|\S+/g) ?? []) {
    if (t.startsWith(":")) out[(key = t.slice(1).toLowerCase())] = key === "var" ? (out.var ?? "") : "";
    else if (key) out[key] += (out[key] ? " " : "") + t.replace(/^"(.*)"$/, "$1");
  }
  return out;
}

/** One file `tangleFiles` writes: PATH relative to the note's folder, its CONTENT, whether it gets `0755`
 *  (a block set `:shebang`), whether missing folders are created (`:mkdirp yes`) and how many blocks went in. */
export type Tangled = { path: string; content: string; exec: boolean; mkdirp: boolean; blocks: number };

const EXT: Record<string, string> = { python: "py", shell: "sh", "emacs-lisp": "el", elisp: "el", javascript: "js", typescript: "ts", ruby: "rb", rust: "rs" };

/** Org-babel-tangle for one note (NOTE: its file name): the blocks with `:tangle path` grouped by target in order,
 *  common indentation removed and joined by a blank line; `:tangle no` or none skips a block, `:tangle yes` names the
 *  file after the note (`notes.org` + sh → `notes.sh`). A file's first `:shebang` heads it. */
export function tangleFiles(text: string, note: string): Tangled[] {
  const out = new Map<string, Tangled & { parts: string[]; shebang: string }>();
  for (const s of srcBlocks(text)) {
    const a = headerArgs(text.slice(s.from).split("\n", 1)[0]);
    let p = a.tangle?.trim();
    if (!p || p === "no") continue;
    if (p === "yes" && s.lang.toLowerCase() === "org") continue; // would overwrite the note itself
    if (p === "yes") p = note.replace(/\.org$/i, "") + "." + (EXT[s.lang.toLowerCase()] ?? s.lang.toLowerCase());
    p = p.replace(/\\/g, "/").replace(/^(\.\/)+/, "");
    const f = out.get(p) ?? { path: p, content: "", exec: false, mkdirp: false, blocks: 0, parts: [], shebang: "" };
    f.parts.push(dedent(s.code.split("\n")).join("\n").replace(/\n+$/, ""));
    f.shebang ||= a.shebang ?? "";
    f.exec ||= !!a.shebang;
    f.mkdirp ||= a.mkdirp === "yes" || a.mkdirp === "t";
    f.blocks++;
    out.set(p, f);
  }
  return [...out.values()].map(({ parts, shebang, ...f }) => ({ ...f, content: (shebang ? shebang + "\n" : "") + parts.join("\n\n") + "\n" }));
}

/** `name=value` pairs of a `:var` header argument or `#+CALL:` argument list (`x=totals, y="a b"`), values as written. */
export const parseVars = (spec: string): [string, string][] =>
  [...spec.matchAll(/([^\s=,()]+)\s*=\s*("[^"]*"|[^\s,()]+)/g)].map((m) => [m[1], m[2]]);

const AFFIL = /^\s*#\+(name|header|caption|attr_[\w-]+|plot)\s*:/i, NAME = /^\s*#\+name:\s*(\S+)/i;

/** The `#+NAME:` among the keyword lines right above the line starting at FROM, else "". */
function nameAbove(text: string, from: number): string {
  const ls = text.slice(0, from).split("\n");
  ls.pop();
  for (let k = ls.length - 1; k >= 0 && AFFIL.test(ls[k]); k--) { const m = NAME.exec(ls[k]); if (m) return m[1]; }
  return "";
}

/** What `:var x=NAME` refers to: the src block under `#+NAME: NAME` with its results' text (null when it has none
 *  yet), else a named result (`#+RESULTS: NAME`, or `#+NAME: NAME` above results or a table) with no block; else null. */
export function named(text: string, name: string): { src: Src | null; result: string | null } | null {
  const src = srcBlocks(text).find((s) => nameAbove(text, s.from) === name);
  if (src) return { src, result: resultsAt(text, src.end)?.body ?? null };
  const ls = lines(text);
  for (let i = 0; i < ls.length; i++) {
    const t = ls[i].text;
    if (/^\s*#\+RESULTS(?:\[[^\]]*\])?:\s*(\S+)/i.exec(t)?.[1] === name) return { src: null, result: resultBody(ls, i + 1, resultEnd(ls, i + 1)) };
    if (NAME.exec(t)?.[1] !== name) continue;
    const r = i + 1 + +RESULTS.test(ls[i + 1]?.text ?? ""), j = resultEnd(ls, r);
    if (j > r || r > i + 1) return { src: null, result: resultBody(ls, r, j) };
  }
  return null;
}

/** A `#+CALL: name(x=1, y=totals)` line around POS: the block's name, its arguments and the line's indent and end. */
export type Call = { name: string; args: [string, string][]; indent: string; end: number };
export function callAt(text: string, pos: number): Call | null {
  const from = text.lastIndexOf("\n", pos - 1) + 1, nl = text.indexOf("\n", pos), end = nl < 0 ? text.length : nl;
  const m = /^(\s*)#\+call:\s*([^\s(]+)\s*(?:\((.*)\))?/i.exec(text.slice(from, end));
  return m ? { name: m[2], args: parseVars(m[3] ?? ""), indent: m[1], end } : null;
}

/** A run Ctrl+Enter does: block SRC with shell variables VARS, its results written under AT (the block, or the
 *  `#+CALL:` line) as `#+RESULTS: NAME` (unnamed when NAME is ""). */
export type Job = { src: Src; vars: [string, string][]; at: Pick<Src, "indent" | "end">; name: string };

/** The `:var` pairs on SRC's `#+begin_src` line. */
const blockVars = (text: string, src: Src) => parseVars(headerArgs(text.slice(src.from).split("\n", 1)[0]).var ?? "");

/** Values for `:var` PAIRS: `"quoted"` and numbers as written, any other value names a block's results (or a named
 *  result). A named block without results yet comes back as `need` (run it first), only one level deep; STACK
 *  holds the names being resolved, so a reference back to one is a cycle. */
export function resolveVars(text: string, pairs: [string, string][], stack: string[] = [], deep = false): { vars: [string, string][] } | { need: Job } | { error: string } {
  const vars: [string, string][] = [];
  for (const [k, v] of pairs) {
    if (!/^[A-Za-z_]\w*$/.test(k)) return { error: `:var ${k}: not a shell variable name.` };
    const lit = /^"([^"]*)"$/.exec(v)?.[1] ?? (/^-?\d+(\.\d+)?$/.test(v) ? v : null);
    if (lit != null) { vars.push([k, lit]); continue; }
    if (stack.includes(v)) return { error: `:var ${k}=${v} is a cycle (${[...stack, v].join(" → ")}); nothing ran.` };
    const n = named(text, v);
    if (!n) return { error: `:var ${k}=${v}: no #+NAME: ${v} in this note; nothing ran.` };
    if (n.result != null) { vars.push([k, n.result]); continue; }
    if (deep) return { error: `“${v}” has no results yet: run it first (dependencies run one level deep).` };
    const d = resolveVars(text, blockVars(text, n.src!), [...stack, v], true);
    return "vars" in d ? { need: { src: n.src!, vars: d.vars, at: n.src!, name: v } } : d;
  }
  return { vars };
}

/** What Ctrl+Enter at POS runs: the src block there, or the block a `#+CALL:` line names (its arguments override the
 *  block's `:var`s); a dependency without results comes first (`need`). Null when POS is in neither. */
export function babelJob(text: string, pos: number): Job | { need: Job } | { error: string } | null {
  const here = srcAt(text, pos), call = here ? null : callAt(text, pos);
  if (!here && !call) return null;
  const name = here ? nameAbove(text, here.from) : call!.name, src = here ?? named(text, name)?.src;
  if (!src) return { error: `No code block named “${name}” in this note.` };
  const pairs = new Map(blockVars(text, src));
  for (const [k, v] of call?.args ?? []) pairs.set(k, v);
  const r = resolveVars(text, [...pairs], name ? [name] : []);
  return "vars" in r ? { src, vars: r.vars, at: call ?? src, name: here ? name : "" } : r;
}

/** VARS as `sh` assignments to put before a block's code, each value single-quoted (`'` as `'\''`), so `$()`,
 *  backquotes and quotes in a value stay text. */
export const shellVars = (vars: [string, string][]) => vars.map(([k, v]) => `${k}='${v.replace(/'/g, "'\\''")}'\n`).join("");
