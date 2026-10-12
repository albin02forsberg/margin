// Run: npm test
import { test } from "node:test";
import assert from "node:assert/strict";
import { diffChange, mergeEdits, type Change } from "./merge.ts";

const apply = (s: string, c: Change) => s.slice(0, c.from) + c.insert + s.slice(c.to);
const saved = "one\ntwo\nthree\n";
const merged = (mine: string, disk: string) => {
  const c = mergeEdits(saved, mine, disk);
  return c && apply(mine, c);
};

test("diffChange keeps the common prefix and suffix", () => {
  assert.deepEqual(diffChange("abc", "aXc"), { from: 1, to: 2, insert: "X" });
  assert.deepEqual(diffChange("aa", "aaa"), { from: 2, to: 2, insert: "a" });
  assert.deepEqual(diffChange("abc", "abc"), { from: 3, to: 3, insert: "" });
  for (const [a, b] of [["one\ntwo\n", "one\n2\ntwo\n"], ["", "x"], ["xyz", ""]]) assert.equal(apply(a, diffChange(a, b)), b);
});

test("mergeEdits applies a disk edit below yours, shifted by your edit", () => {
  assert.equal(merged("one edited\ntwo\nthree\n", "one\ntwo\nthree\nfour\n"), "one edited\ntwo\nthree\nfour\n");
  assert.equal(merged("one\n\ntwo\nthree\n", "one\ntwo\nthree!\n"), "one\n\ntwo\nthree!\n");
});

test("mergeEdits applies a disk edit above yours as is", () => {
  assert.equal(merged("one\ntwo\nthree\nmine\n", "zero\none\ntwo\nthree\n"), "zero\none\ntwo\nthree\nmine\n");
  assert.equal(merged("one\ntwo\nTHREE\n", "1\ntwo\nthree\n"), "1\ntwo\nTHREE\n");
});

test("mergeEdits flags overlapping edits as a conflict", () => {
  assert.equal(merged("one\nTWO\nthree\n", "one\ntwo!\nthree\n"), null); // same line
  assert.equal(merged("one\nTWO\nthree\n", "one\ntwo-three\n"), null); // touching edits count as overlapping
  assert.equal(merged(saved + "mine\n", saved + "disk\n"), null); // both insert at the same point
  assert.equal(mergeEdits("", "mine\n", "theirs\n"), null); // empty file
});
