import { test } from "node:test";
import assert from "node:assert/strict";
import { pushRecent, sortByRecent, parseRecents } from "./recents.ts";

test("pushRecent moves to front, dedupes and caps", () => {
  assert.deepEqual(pushRecent(["a", "b", "c"], "c"), ["c", "a", "b"]);
  assert.deepEqual(pushRecent(["a", "b"], "z", 2), ["z", "a"]);
});

test("sortByRecent: recents first in recency order, rest stable, unknown ids ignored", () => {
  const items = ["a", "b", "c", "d"].map((label) => ({ label }));
  assert.deepEqual(sortByRecent(items, ["gone", "c", "a"]).map((i) => i.label), ["c", "a", "b", "d"]);
  assert.deepEqual(sortByRecent(items, []).map((i) => i.label), ["a", "b", "c", "d"]);
});

test("parseRecents tolerates corrupt input", () => {
  assert.deepEqual(parseRecents("nope"), []);
  assert.deepEqual(parseRecents('{"a":1}'), []);
  assert.deepEqual(parseRecents('["a",1]'), ["a"]);
});
