// Org footnotes: `[fn:LABEL]` references and their `[fn:LABEL] text` definitions
// at a line start. Inline (`[fn:name:text]`) and anonymous ones aren't followed.

/** A footnote reference or definition; the editor's highlighter uses it too. */
export const FN = /\[fn:([^\]\s]+)\]/g;

type Fn = { label: string; from: number; to: number; def: boolean };

/** Every followable footnote in TEXT, skipping #+begin_…/#+end_ blocks. */
function all(text: string): Fn[] {
  const out: Fn[] = [];
  let pos = 0, block = false;
  for (const line of text.split("\n")) {
    const t = line.trimStart().toLowerCase();
    if (block) block = !t.startsWith("#+end_");
    else if (t.startsWith("#+begin_")) block = true;
    else for (const m of line.matchAll(FN)) {
      if (!m[1].includes(":")) out.push({ label: m[1], from: pos + m.index!, to: pos + m.index! + m[0].length, def: m.index === 0 });
    }
    pos += line.length + 1;
  }
  return out;
}

/** The footnote at POS and where following it goes: a reference to its definition's
 *  text, a definition to its first reference (`to` null when there is none); null
 *  when POS isn't on a footnote. */
export function footnoteTarget(text: string, pos: number): { label: string; def: boolean; to: number | null } | null {
  const ls = text.lastIndexOf("\n", pos - 1) + 1, le = text.indexOf("\n", pos);
  if (!text.slice(ls, le < 0 ? undefined : le).includes("[fn:")) return null; // cheap: Enter calls this
  const fns = all(text);
  const at = fns.find((f) => f.from <= pos && pos <= f.to);
  if (!at) return null;
  const t = fns.find((f) => f.label === at.label && f.def !== at.def);
  return { label: at.label, def: at.def, to: !t ? null : t.def ? t.to + (text[t.to] === " " ? 1 : 0) : t.from };
}

/** One more than the highest numeric footnote label in TEXT. */
export const nextLabel = (text: string) => Math.max(0, ...all(text).map((f) => (/^\d+$/.test(f.label) ? +f.label : 0))) + 1;

/** Insert a new numbered reference at POS and its empty definition at the end of
 *  the `* Footnotes` section (appended to TEXT when missing); the cursor goes to
 *  the definition. Changes are in TEXT's coordinates (one CodeMirror change set). */
export function addFootnote(text: string, pos: number) {
  const ref = `[fn:${nextLabel(text)}]`;
  const h = /^\* Footnotes[ \t]*$/m.exec(text);
  let at: number, def: string;
  if (h) {
    const next = text.slice(h.index + h[0].length).search(/^\* /m);
    at = (next < 0 ? text : text.slice(0, h.index + h[0].length + next)).trimEnd().length;
    def = `\n${ref} `;
  } else {
    at = text.length;
    const t = pos === at ? text + ref : text; // the reference may end the note
    def = `${t && !t.endsWith("\n") ? "\n" : ""}${t.trim() ? "\n" : ""}* Footnotes\n${ref} `;
  }
  return { changes: [{ from: pos, insert: ref }, { from: at, insert: def }], cursor: at + def.length + (pos <= at ? ref.length : 0) };
}
