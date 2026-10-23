/** Named positions in notes (#672): a file plus the heading above the cursor, so edits above don't break them. */
import { findTarget, headTitle, norm } from "./links.ts";

/** HEADING is the title of the nearest heading at or above LINE (0-based), OFFSET the line's distance below it. */
export type Bookmark = { name: string; file: string; heading?: string; offset: number; line: number };

/** Parse a stored bookmark list; entries without a name, file or line are dropped. */
export function parseBookmarks(s: string): Bookmark[] {
  try {
    const v = JSON.parse(s);
    if (!Array.isArray(v)) return [];
    return v.filter((b) => b && typeof b.name === "string" && typeof b.file === "string" && Number.isInteger(b.line)).map((b) => ({
      name: b.name, file: b.file, line: b.line, offset: Number.isInteger(b.offset) ? b.offset : 0, ...(typeof b.heading === "string" && b.heading ? { heading: b.heading } : {}),
    }));
  } catch {
    return [];
  }
}

/** LIST with B added at the end, replacing one with the same name. */
export const addBookmark = (list: Bookmark[], b: Bookmark) => [...removeBookmark(list, b.name), b];
export const removeBookmark = (list: Bookmark[], name: string) => list.filter((b) => b.name !== name);
/** LIST with bookmarks in FROM pointing at TO (a renamed or moved note). */
export const moveBookmarks = (list: Bookmark[], from: string, to: string) => list.map((b) => (b.file === from ? { ...b, file: to } : b));
/** LIST with bookmarks in FILE anchored to the heading titled OLD (normalised, as `retitlePlan` returns it) re-anchored to NEW. */
export const retitleBookmarks = (list: Bookmark[], file: string, old: string, new_: string) =>
  list.map((b) => (b.file === file && b.heading != null && norm(b.heading) === old ? { ...b, heading: new_ } : b));

/** A bookmark NAME for LINE of FILE's TEXT, anchored to the nearest heading at or above it (none above the first). */
export function bookmarkAt(name: string, file: string, text: string, line: number, keywords?: string[]): Bookmark {
  const lines = text.split(/\r?\n/);
  for (let i = Math.min(line, lines.length - 1); i >= 0; i--) {
    const t = headTitle(lines[i], keywords);
    if (t != null) return t.trim() ? { name, file, heading: t, offset: line - i, line } : { name, file, offset: 0, line };
  }
  return { name, file, offset: 0, line };
}

/** 0-based line B points at in TEXT (null: the file is gone): its heading plus offset, else the stored line (clamped
 *  to the end of the note). Stale when the file is gone, the heading isn't found or the line is past the end. */
export function resolveBookmark(b: Bookmark, text: string | null, keywords?: string[]): { line: number; stale: boolean } {
  if (text == null) return { line: 0, stale: true };
  const n = text.split(/\r?\n/).length;
  const h = b.heading ? findTarget(text, `*${b.heading}`, keywords) : null;
  if (h != null) return { line: Math.min(h + b.offset, n - 1), stale: false };
  return { line: Math.min(b.line, n - 1), stale: !!b.heading || b.line >= n };
}
