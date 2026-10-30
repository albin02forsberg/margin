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

/** The edit that puts OUTPUT under block SRC as `#+RESULTS:` + `: ` lines (org's verbatim form, which no
 *  output can close early), replacing the `#+RESULTS:` (and its `:` lines) that follows the block, if any. */
export function resultsChange(text: string, src: Src, output: string): { from: number; to: number; insert: string } {
  const body = output.replace(/\n+$/, "");
  const res = `${src.indent}#+RESULTS:` + (body ? "\n" + body.split("\n").map((l) => (l ? `${src.indent}: ${l}` : `${src.indent}:`)).join("\n") : "");
  const ls = lines(text.slice(src.end + 1));
  let i = 0;
  while (i < ls.length && !ls[i].text.trim()) i++;
  if (src.end < text.length && i < ls.length && /^\s*#\+RESULTS(\[[^\]]*\])?:/i.test(ls[i].text)) {
    const j = resultEnd(ls, i + 1);
    const base = src.end + 1;
    return { from: base + ls[i].from, to: base + ls[j - 1].to, insert: res };
  }
  return { from: src.end, to: src.end, insert: "\n\n" + res };
}
