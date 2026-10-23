import { test } from "node:test";
import assert from "node:assert/strict";
import { addBookmark, bookmarkAt, moveBookmarks, parseBookmarks, removeBookmark, resolveBookmark, retitleBookmarks } from "./bookmarks.ts";

const note = "#+title: Log\nintro\n* Projects\n** TODO [#A] Next steps [1/2] :work:\n- one\n- two\n* Log\n";

test("bookmarkAt anchors to the nearest heading above, or the line before the first one", () => {
  assert.deepEqual(bookmarkAt("n", "a.org", note, 5), { name: "n", file: "a.org", heading: "Next steps [1/2]", offset: 2, line: 5 });
  assert.deepEqual(bookmarkAt("h", "a.org", note, 2), { name: "h", file: "a.org", heading: "Projects", offset: 0, line: 2 });
  assert.deepEqual(bookmarkAt("t", "a.org", note, 1), { name: "t", file: "a.org", offset: 0, line: 1 });
});

test("resolveBookmark follows a moved heading, falls back to the line, and flags stale ones", () => {
  const b = bookmarkAt("n", "a.org", note, 5);
  assert.deepEqual(resolveBookmark(b, note), { line: 5, stale: false });
  // lines added above, and the cookie changed: still found by title
  const moved = "new\nlines\n" + note.replace("[1/2]", "[2/2]");
  assert.deepEqual(resolveBookmark(b, moved), { line: 7, stale: false });
  // heading renamed: the stored line, dimmed
  assert.deepEqual(resolveBookmark(b, note.replace("Next steps", "Later")), { line: 5, stale: true });
  // heading gone and the note shorter than the stored line
  assert.deepEqual(resolveBookmark(b, "* Other\nx"), { line: 1, stale: true });
  // no heading: just the line
  assert.deepEqual(resolveBookmark(bookmarkAt("t", "a.org", note, 1), note), { line: 1, stale: false });
  assert.deepEqual(resolveBookmark(b, null), { line: 0, stale: true });
});

test("add replaces by name, remove and move", () => {
  const a = { name: "a", file: "x.org", offset: 0, line: 1 }, b = { name: "b", file: "y.org", offset: 0, line: 2 };
  assert.deepEqual(addBookmark([a, b], { ...a, line: 9 }), [b, { ...a, line: 9 }]);
  assert.deepEqual(removeBookmark([a, b], "a"), [b]);
  assert.deepEqual(moveBookmarks([a, b], "x.org", "z.org"), [{ ...a, file: "z.org" }, b]);
});

test("parseBookmarks drops bad entries and falls back to []", () => {
  assert.deepEqual(parseBookmarks("{oops"), []);
  assert.deepEqual(parseBookmarks('{"a":1}'), []);
  assert.deepEqual(parseBookmarks('[{"name":"a","file":"x","line":3,"heading":"H","offset":1},{"name":"b"},null,{"name":"c","file":"y","line":0}]'),
    [{ name: "a", file: "x", line: 3, offset: 1, heading: "H" }, { name: "c", file: "y", line: 0, offset: 0 }]);
});

test("retitleBookmarks re-anchors bookmarks on a renamed heading in that file only", () => {
  const a = bookmarkAt("a", "a.org", note, 2), b = { ...a, name: "b", file: "b.org" };
  assert.deepEqual(retitleBookmarks([a, b], "a.org", "projects", "Work"), [{ ...a, heading: "Work" }, b]);
  assert.equal(retitleBookmarks([a], "a.org", "other", "Work")[0], a);
});
