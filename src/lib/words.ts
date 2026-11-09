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

/** `#+WORDGOAL: N` from the first 30 lines of TEXT; 0 when absent or not a positive integer. */
export function parseWordGoal(text: string) {
  for (const l of text.split("\n", 30)) { const m = /^\s*#\+WORDGOAL:\s*(\d+)\s*$/i.exec(l); if (m) return +m[1]; }
  return 0;
}

/** Status-bar label: "412 words", or "38 of 412 words" while text is selected; SCOPE ("heading") adds " in heading".
 *  GOAL > 0 makes it "412 / 1500 words". */
export function wordsLabel(doc: number, sel?: number, scope = "", goal = 0) {
  const n = (k: number) => `${k}${goal > 0 && k === doc ? ` / ${goal}` : ""} word${k === 1 && !goal ? "" : "s"}${scope && ` in ${scope}`}`;
  return sel == null ? n(doc) : `${sel} of ${n(doc)}`;
}
