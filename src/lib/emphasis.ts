// Org inline emphasis (*bold*, /italic/, ~code~, …): the span regex the editor
// styles, and toggling a mark around a selection or word as a pure function.

/** Pre-char, mark, body; the closing mark is followed by space, punctuation or the end. */
export const EMPH = /(^|[\s\-({'"])([*/_=~+])(\S|\S.*?\S)\2(?=[\s\-.,:!?;'")}\]]|$)/g;
const WORD = /[\p{L}\p{N}_'-]/u;

/** Toggle MARK on [FROM, TO) of TEXT: unwrap the MARK span it lies in, else wrap
 *  the selection (trimmed) or the word at the cursor, else insert an empty pair.
 *  Returns the changes (positions in TEXT), the new selection and whether the
 *  pair was empty (the caller then wants insert mode). */
export function toggleEmphasis(text: string, from: number, to: number, mark: string) {
  for (const m of text.matchAll(EMPH)) {
    const s = m.index! + m[1].length, e = s + m[3].length + 2;
    if (m[2] !== mark || from < s || from >= e || to > e) continue;
    const map = (p: number) => p - (p > s ? 1 : 0) - (p >= e ? 1 : 0);
    return { changes: [{ from: s, to: s + 1, insert: "" }, { from: e - 1, to: e, insert: "" }], from: map(from), to: map(to), empty: false };
  }
  let a = from, b = to;
  while (a < b && /\s/.test(text[a])) a++;
  while (b > a && /\s/.test(text[b - 1])) b--;
  if (a === b) {
    for (a = b = from; a > 0 && WORD.test(text[a - 1]); ) a--;
    while (b < text.length && WORD.test(text[b])) b++;
  }
  if (a === b) return { changes: [{ from, to, insert: mark + mark }], from: from + 1, to: from + 1, empty: true };
  const sel = from < to ? [a + 1, b + 1] : [from + 1, from + 1];
  return { changes: [{ from: a, to: a, insert: mark }, { from: b, to: b, insert: mark }], from: sel[0], to: sel[1], empty: false };
}
