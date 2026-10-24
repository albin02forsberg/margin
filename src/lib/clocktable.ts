// Dynamic blocks Margin fills in a note, `#+BEGIN: clocktable :block thismonth` (#693, from the time log) and
// `#+BEGIN: tasks :query "…"` (#756, a task query) … `#+END:`, whose body is regenerated.

export type ClockBlock = { name: "clocktable" | "tasks"; args: string; from: number; to: number };

/** The clocktable and tasks blocks of TEXT: block NAME, header options ARGS and the body span FROM..TO (from the line
 *  after the header to the start of the `#+END:` line). Blocks without an `#+END:` are skipped, so nothing past them is touched. */
export function clockBlocks(text: string): ClockBlock[] {
  const out: ClockBlock[] = [], re = /^[ \t]*#\+begin:[ \t]+(clocktable|tasks)\b(.*)$/gim;
  for (let m; (m = re.exec(text)); ) {
    const nl = text.indexOf("\n", m.index + m[0].length);
    if (nl < 0) break;
    const end = /^[ \t]*#\+end:/gim;
    end.lastIndex = nl + 1;
    const e = end.exec(text);
    const next = /^[ \t]*#\+begin:/gim;
    next.lastIndex = nl + 1;
    const n = next.exec(text);
    if (!e || (n && n.index < e.index)) continue; // unterminated: the next block's #+END: isn't ours
    out.push({ name: m[1].toLowerCase() as ClockBlock["name"], args: m[2].trim(), from: nl + 1, to: e.index });
    re.lastIndex = e.index;
  }
  return out;
}
