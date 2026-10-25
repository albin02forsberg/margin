// Code blocks run on demand (Ctrl+Enter / Space i r, like org-babel): find the block, write #+RESULTS: under it.

/** A `#+begin_src LANG … #+end_src` block: its language, code (org's `,*` / `,#+` escapes undone),
 *  indentation and the offset of the end of its `#+end_src` line. */
export type Src = { lang: string; code: string; indent: string; end: number };

type Line = { text: string; from: number; to: number };
function lines(text: string): Line[] {
  const out: Line[] = [];
  let from = 0;
  for (const t of text.split("\n")) { out.push({ text: t, from, to: from + t.length }); from += t.length + 1; }
  return out;
}

const BEGIN = /^(\s*)#\+begin_src(?:[ \t]+(\S+))?/i, END = /^\s*#\+end_src\b/i;

/** The src block around offset POS (its begin and end lines count), else null. */
export function srcAt(text: string, pos: number): Src | null {
  const ls = lines(text);
  for (let i = 0; i < ls.length; i++) {
    const b = BEGIN.exec(ls[i].text);
    if (!b) continue;
    let j = i + 1;
    while (j < ls.length && !END.test(ls[j].text)) j++;
    if (j === ls.length) return null; // unclosed: nothing after it is a block either
    if (pos >= ls[i].from && pos <= ls[j].to) {
      const code = ls.slice(i + 1, j).map((l) => l.text.replace(/^(\s*),(?=\*|#\+)/, "$1")).join("\n");
      return { lang: b[2] ?? "", code, indent: b[1], end: ls[j].to };
    }
    i = j;
  }
  return null;
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
    let j = i + 1;
    while (j < ls.length && /^\s*:( |$)/.test(ls[j].text)) j++;
    const base = src.end + 1;
    return { from: base + ls[i].from, to: base + ls[j - 1].to, insert: res };
  }
  return { from: src.end, to: src.end, insert: "\n\n" + res };
}
