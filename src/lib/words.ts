/** Words in TEXT: runs of non-whitespace. */
export const wordCount = (text: string) => text.split(/\s+/).filter(Boolean).length;

/** Status-bar label: "412 words", or "38 of 412 words" while text is selected. */
export function wordsLabel(doc: number, sel?: number) {
  const n = (k: number) => `${k} word${k === 1 ? "" : "s"}`;
  return sel == null ? n(doc) : `${sel} of ${n(doc)}`;
}
