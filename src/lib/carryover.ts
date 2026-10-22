// Journal carry-over (#648): move yesterday's open checkboxes and TODO subtrees into today's entry.

const head = (l: string) => /^(\*+)\s/.exec(l)?.[1].length ?? 0;
const indent = (l: string) => /^\s*/.exec(l)![0].length;
const open = (l: string) => /^\s*[-+]\s+\[ \]/.test(l);

/** Split TEXT into what stays (`kept`) and what moves (`items`, outermost first): headings with a TODO
 *  keyword (not tagged :ARCHIVE:) as whole subtrees, re-levelled to level 2; unchecked `- [ ]`/`+ [ ]`
 *  items with their deeper-indented lines, dedented. Done/other-keyword/archived subtrees stay whole.
 *  Checkbox items come first (body of the new heading), then subtrees, each in source order. */
export function extractOpen(text: string, todo: string[], done: string[]): { kept: string; items: string[] } {
  const eol = text.includes("\r\n") ? "\r\n" : "\n", lines = text.split(/\r?\n/);
  const kept: string[] = [], boxes: string[] = [], trees: string[] = [];
  for (let i = 0; i < lines.length; ) {
    const l = lines[i], n = head(l);
    if (n) {
      let e = i + 1;
      while (e < lines.length && !(head(lines[e]) && head(lines[e]) <= n)) e++;
      while (e > i + 1 && !lines[e - 1].trim()) e--; // trailing blank lines stay
      const kw = l.slice(n).trim().split(/\s/)[0], archived = !!/\s(:\S+:)\s*$/.exec(l)?.[1].split(":").includes("ARCHIVE");
      if (todo.includes(kw) && !archived) trees.push(lines.slice(i, e).map((x) => (head(x) ? "*".repeat(head(x) - n + 2) + x.slice(head(x)) : x)).join("\n"));
      else if (archived || done.includes(kw)) kept.push(...lines.slice(i, e));
      else { kept.push(lines[i++]); continue; }
      i = e;
    } else if (open(l)) {
      const d = indent(l);
      let e = i + 1;
      for (let j = i + 1; j < lines.length && !head(lines[j]); j++) {
        if (!lines[j].trim()) continue;
        if (indent(lines[j]) <= d) break;
        e = j + 1;
      }
      boxes.push(lines.slice(i, e).map((x) => x.slice(Math.min(d, indent(x)))).join("\n"));
      i = e;
    } else kept.push(lines[i++]);
  }
  return { kept: kept.join(eol), items: [...boxes, ...trees] };
}

/** Today's entry for ITEMS carried over from DATE (`capture_insert` files it at top level). */
export const carryEntry = (date: string, items: string[]) => [`* Carried over from ${date}`, ...items].join("\n");
