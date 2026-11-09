// The `vim_init` setting (#1209): ex commands run like a vimrc, one per line, when the first editor opens.

/** The non-empty, non-comment (`"`) lines of TEXT, trimmed and without a leading `:`, with their 1-based numbers. */
export function parseVimInit(text: string): { line: number; cmd: string }[] {
  return text.split("\n").flatMap((l, i) => {
    const cmd = l.trim().replace(/^:+\s*/, "");
    return cmd && !cmd.startsWith('"') ? [{ line: i + 1, cmd }] : [];
  });
}

export type Ctx = "normal" | "insert" | "visual" | "operatorPending" | undefined;
/** codemirror-vim's map commands: name, shortest abbreviation, mode (none = normal, visual and operator-pending). */
const MAPS: [string, string, Ctx][] = [
  ["map", "map", undefined], ["nmap", "nm", "normal"], ["imap", "im", "insert"], ["vmap", "vm", "visual"], ["omap", "om", "operatorPending"],
  ["noremap", "no", undefined], ["nnoremap", "nn", "normal"], ["inoremap", "ino", "insert"], ["vnoremap", "vn", "visual"], ["onoremap", "ono", "operatorPending"],
];

/** The lhs and mode CMD maps, if it is a map command with both an lhs and an rhs (`nn ; :` → `;` in normal mode). */
export function mapTarget(cmd: string): { lhs: string; ctx: Ctx } | null {
  const [name, lhs, rhs] = cmd.split(/\s+/);
  const m = MAPS.find(([n, short]) => n.startsWith(name) && name.length >= short.length);
  return m && lhs && rhs ? { lhs, ctx: m[2] } : null;
}

/** Run TEXT's lines through EXEC (returns an error message, or nothing on success); the rest still run after a failure.
 *  Returns the failures as `line N: msg` and the mappings made, so a re-run can undo them first. */
export function runVimInit(text: string, exec: (cmd: string) => string | void) {
  const errors: string[] = [], mapped: { lhs: string; ctx: Ctx }[] = [];
  for (const { line, cmd } of parseVimInit(text)) {
    const m = mapTarget(cmd);
    // The Space leader is a capture-phase window listener, so a normal/visual `<Space>…` mapping would never fire.
    const err = m && m.ctx !== "insert" && m.ctx !== "operatorPending" && /^<space>/i.test(m.lhs) ? "<Space> is the leader key" : exec(cmd);
    if (err) errors.push(`line ${line}: ${err}`);
    else if (m) mapped.push(m);
  }
  return { errors, mapped };
}
