// Org outline helpers and narrowing (org-narrow-to-subtree): the view shows only the
// subtree at the cursor. View-only: the hidden text stays in the doc and is saved.
import { EditorSelection, EditorState, StateEffect, StateField, type Range } from "@codemirror/state";
import { Decoration, EditorView, type DecorationSet } from "@codemirror/view";

export const level = (text: string) => /^(\*+) /.exec(text)?.[1].length ?? 0;

/** Last line number (1-based) of the subtree whose heading is on line N. */
export function subtreeEnd(state: EditorState, n: number): number {
  const lvl = level(state.doc.line(n).text);
  let i = n + 1;
  while (i <= state.doc.lines) {
    const l = level(state.doc.line(i).text);
    if (l && l <= lvl) break;
    i++;
  }
  // The empty line after a final newline belongs to no subtree.
  if (i > state.doc.lines && i - 1 > n && state.doc.line(i - 1).text === "") i--;
  return i - 1;
}

/** Line number of the heading at or above line N, or 0. */
export function headingAt(state: EditorState, n: number): number {
  for (let i = n; i >= 1; i--) if (level(state.doc.line(i).text)) return i;
  return 0;
}

type Narrow = { from: number; to: number };

/** The subtree around POS (its heading's line start to its last line's end), or null before the first heading. */
export function narrowRange(state: EditorState, pos: number): Narrow | null {
  const h = headingAt(state, state.doc.lineAt(pos).number);
  return h ? { from: state.doc.line(h).from, to: state.doc.line(subtreeEnd(state, h)).to } : null;
}

export const narrowEffect = StateEffect.define<Narrow | null>();

const hideBlock = Decoration.replace({ block: true });

/** Whole lines above and below R, as block replacements (empty ones skipped). */
function hidden(state: EditorState, r: Narrow | null): DecorationSet {
  if (!r) return Decoration.none;
  const out: Range<Decoration>[] = [];
  const top = state.doc.lineAt(r.from).from - 1, bottom = state.doc.lineAt(r.to).to + 1;
  if (top > 0) out.push(hideBlock.range(0, top));
  if (bottom < state.doc.length) out.push(hideBlock.range(bottom, state.doc.length));
  return Decoration.set(out);
}

/** The narrowed range of this editor's doc, or null when it shows everything. */
export const narrowField = StateField.define<Narrow | null>({
  create: () => null,
  update(r, tr) {
    // assoc -1 / 1: text typed at either edge stays inside.
    if (r && tr.docChanged) r = { from: tr.changes.mapPos(r.from, -1), to: tr.changes.mapPos(r.to, 1) };
    for (const e of tr.effects) if (e.is(narrowEffect)) r = e.value;
    return r && r.from < r.to ? r : null;
  },
  provide: (f) => {
    const deco = EditorView.decorations.compute([f, "doc"], (s) => hidden(s, s.field(f)));
    return [
      deco,
      EditorView.atomicRanges.of((v) => hidden(v.state, v.state.field(f))),
      // Keep the cursor inside, so gg / G / search land on the visible text.
      EditorState.transactionFilter.of((tr) => {
        const r = tr.state.field(f);
        if (!r || !(tr.selection || tr.effects.some((e) => e.is(narrowEffect)))) return tr;
        const sel = tr.state.selection, c = (p: number) => Math.min(Math.max(p, r.from), r.to);
        if (sel.ranges.every((x) => c(x.anchor) === x.anchor && c(x.head) === x.head)) return tr;
        return [tr, { selection: EditorSelection.create(sel.ranges.map((x) => EditorSelection.range(c(x.anchor), c(x.head))), sel.mainIndex), sequential: true }];
      }),
    ];
  },
});

/** The narrowed-to heading's title, or "" when not narrowed. */
export function narrowedTo(state: EditorState): string {
  const r = state.field(narrowField, false);
  if (!r) return "";
  const t = state.doc.lineAt(r.from).text;
  return t.slice(level(t) + 1).replace(/\s+:[\w@#%:]+:\s*$/, "").trim() || t;
}

/** Narrow to the subtree at the cursor, or widen when narrowed. null outside org notes or before the first heading. */
export function narrowToggle(v: EditorView): "narrow" | "widen" | null {
  const cur = v.state.field(narrowField, false);
  if (cur === undefined) return null;
  if (cur) {
    v.dispatch({ effects: [narrowEffect.of(null), EditorView.scrollIntoView(v.state.selection.main.head, { y: "center" })] });
    return "widen";
  }
  const r = narrowRange(v.state, v.state.selection.main.head);
  if (!r) return null;
  v.dispatch({ effects: narrowEffect.of(r), selection: v.state.selection, scrollIntoView: true });
  return "narrow";
}

/** Effects that widen the narrow when POS lies outside it, so a jump there is not clamped back. */
export function revealEffects(state: EditorState, pos: number): StateEffect<unknown>[] {
  const r = state.field(narrowField, false);
  return r && (pos < r.from || pos > r.to) ? [narrowEffect.of(null)] : [];
}
