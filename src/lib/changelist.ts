// Vim's change list (`g;` / `g,`): where recent edits happened, mapped through later edits.
import { StateEffect, StateField, type EditorState } from "@codemirror/state";

type Changes = { list: number[]; i: number };
const MAX = 100;
const stepEffect = StateEffect.define<number>();

/** Edit positions, oldest first; I is the `g;` cursor (list.length = past the newest). */
export const changeList = StateField.define<Changes>({
  create: () => ({ list: [], i: 0 }),
  update(v, tr) {
    for (const e of tr.effects) if (e.is(stepEffect)) v = { ...v, i: e.value };
    if (!tr.docChanged) return v;
    let pos = -1;
    tr.changes.iterChangedRanges((_fa, _ta, fb) => (pos = fb));
    const list = v.list.map((p) => tr.changes.mapPos(p, -1));
    // Like vim: an edit on the newest entry's line replaces it instead of adding one.
    const line = (p: number) => tr.state.doc.lineAt(Math.min(p, tr.state.doc.length)).number;
    if (list.length && line(list[list.length - 1]) === line(pos)) list.pop();
    list.push(pos);
    if (list.length > MAX) list.shift();
    return { list, i: list.length };
  },
});

/** `g;` (D < 0) / `g,` (D > 0) by |D| entries: the position to jump to and the effect that moves the cursor, or a vim-style error. */
export function changeStep(s: EditorState, d: number): { pos: number; effect: StateEffect<number> } | { msg: string } {
  const { list, i } = s.field(changeList);
  if (!list.length) return { msg: "E664: Changelist is empty" };
  if (d < 0 ? i <= 0 : i >= list.length - 1) return { msg: d < 0 ? "E662: At start of changelist" : "E663: At end of changelist" };
  const j = Math.max(0, Math.min(list.length - 1, i + d)); // from past the newest, the first g; lands on the newest entry
  const p = Math.min(list[j], s.doc.length), ln = s.doc.lineAt(p);
  return { pos: p === ln.to && p > ln.from ? p - 1 : p, effect: stepEffect.of(j) }; // normal mode: not past the line's last char
}
