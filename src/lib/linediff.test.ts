import { test } from "node:test";
import assert from "node:assert/strict";
import { lineDiff } from "./linediff.ts";

test("equal texts give no diff", () => assert.equal(lineDiff("a\nb", "a\nb"), ""));
test("a changed line, with context and elided runs", () => {
  const a = "1\n2\n3\n4\n5\n6\n7\n8\n9", b = "1\n2\n3\n4\nfive\n6\n7\n8\n9";
  assert.equal(lineDiff(a, b, 1), "…\n  4\n- 5\n+ five\n  6\n…");
});
test("insertions and deletions in the middle, two hunks", () => {
  assert.equal(lineDiff("a\nb\nc\nd\ne\nf\ng", "a\nx\nb\nc\nd\ne\ng", 0), "…\n+ x\n…\n- f\n…");
});
test("from empty and to empty", () => {
  assert.equal(lineDiff("", "a\nb"), "- \n+ a\n+ b");
  assert.equal(lineDiff("a\n", ""), "- a\n  ");
});
test("huge middles fall back to remove-then-add", () => {
  const a = Array.from({ length: 3000 }, (_, i) => `a${i}`).join("\n"), b = Array.from({ length: 3000 }, (_, i) => `b${i}`).join("\n");
  const d = lineDiff(a, b).split("\n");
  assert.equal(d.length, 6000);
  assert.equal(d[0], "- a0");
  assert.equal(d[3000], "+ b0");
});
