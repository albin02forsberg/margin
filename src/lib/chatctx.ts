/** What of the open note the AI chat sends (#211): the Rust side cuts the note to its first 4000 bytes, so pick the useful part here. */
const LIMIT = 4000;
const enc = new TextEncoder();
const bytes = (s: string) => enc.encode(s).length;
const level = (l: string) => /^(\*+) /.exec(l)?.[1].length ?? 0;

/** A streamed or cut-off chat answer as ai.rs `clean(raw, false)` would leave it (keep the two in step), plus an unclosed trailing `<think>`. */
export const cleanChat = (s: string) =>
  s.replace(/<think>[\s\S]*?<\/think>/g, "").replace(/<think>[\s\S]*$/, "").trim().replace(/^\s*\*+ /gm, "- ");

/** What ai_chat's ON_TOOL channel sends (#963): a tool call the model made, or that the model can't use tools. */
export type ToolEvent = { name: string; args: Record<string, unknown> } | { no_tools: true };

/** EVENT as one line under the question: "searched notes: foo", or the hint that the model can't use tools. */
export function toolLine(e: ToolEvent): string {
  if ("no_tools" in e) return "This model can't use tools, so it answered from the matching notes only.";
  const arg = (k: string) => String(e.args?.[k] ?? "").trim();
  const what = ({ search_notes: ["searched notes:", "query"], open_note: ["opened", "title"], neighbors: ["looked up links of", "title"] } as Record<string, string[]>)[e.name];
  return what ? `${what[0]} ${arg(what[1])}`.trim() : `called ${e.name}`;
}

/**
 * The part of DOC to send, for a selection FROM..TO and cursor HEAD (character offsets), with a LABEL for the UI:
 * a non-empty selection wins; else the whole note if it fits; else the largest section around the cursor that
 * fits; else about LIMIT bytes of lines around the cursor. TITLE labels the whole note.
 */
export function chatContext(doc: string, from: number, to: number, head: number, title: string, limit = LIMIT): { text: string; label: string } {
  if (from !== to) {
    const sel = doc.slice(Math.min(from, to), Math.max(from, to));
    if (bytes(sel) <= limit) return { text: sel, label: "selection" };
    let [text, size] = ["", 0]; // too long: its first LIMIT bytes, cut on a code point so Rust's cut is a no-op
    for (const ch of sel) {
      if ((size += bytes(ch)) > limit) break;
      text += ch;
    }
    return { text, label: `selection (first ${+(limit / 1000).toFixed(1)} KB)` };
  }
  if (bytes(doc) <= limit) return { text: doc, label: title };
  const lines = doc.split("\n");
  const cur = doc.slice(0, head).split("\n").length - 1;
  // headings enclosing the cursor line, outermost first (a heading line encloses itself)
  const anc: number[] = [];
  for (let i = cur, lvl = Infinity; i >= 0; i--) {
    const l = level(lines[i]);
    if (l && l < lvl) { anc.unshift(i); lvl = l; }
  }
  for (const h of anc) {
    let end = h + 1;
    while (end < lines.length && !(level(lines[end]) && level(lines[end]) <= level(lines[h]))) end++;
    const text = lines.slice(h, end).join("\n");
    if (bytes(text) <= limit) return { text, label: `section ${lines[h].replace(/^\*+ /, "")}` };
  }
  let [a, b, size] = [cur, cur + 1, bytes(lines[cur]) + 1];
  for (;;) { // grow alternately, a line at a time, while it fits
    const up = a > 0 && size + bytes(lines[a - 1]) + 1 <= limit;
    const down = b < lines.length && size + bytes(lines[b]) + 1 <= limit;
    if (!up && !down) break;
    if (up && (cur - a <= b - cur - 1 || !down)) size += bytes(lines[--a]) + 1;
    else size += bytes(lines[b++]) + 1;
  }
  return { text: lines.slice(a, b).join("\n"), label: "text around the cursor" };
}
