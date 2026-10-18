const WORD = /[\p{L}\p{N}]/u;

/** Words in TEXT: whitespace-separated tokens with a letter or digit. For ORG, skip planning lines, drawers and
 *  `#+KEY:` names, and trailing `:tags:` on headings (a TODO keyword still counts). */
export function wordCount(text: string, org = false) {
  let n = 0, drawer = false;
  for (let line of text.split("\n")) {
    if (org) {
      const t = line.trim();
      if (drawer) { drawer = !/^:END:$/i.test(t); continue; }
      if (/^:[\w-]+:$/.test(t) && !/^:END:$/i.test(t)) { drawer = true; continue; }
      if (/^(SCHEDULED|DEADLINE|CLOSED):/.test(t)) continue;
      line = line.replace(/^\s*#\+\w+:/, "");
      if (/^\*+\s/.test(line)) line = line.replace(/\s:[\w@#%:-]+:\s*$/, "");
    }
    for (const w of line.split(/\s+/)) if (WORD.test(w)) n++;
  }
  return n;
}

/** Status-bar label: "412 words", or "38 of 412 words" while text is selected. */
export function wordsLabel(doc: number, sel?: number) {
  const n = (k: number) => `${k} word${k === 1 ? "" : "s"}`;
  return sel == null ? n(doc) : `${sel} of ${n(doc)}`;
}
