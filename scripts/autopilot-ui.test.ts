// Run: npm test
import { test } from "node:test";
import assert from "node:assert/strict";
import { cut, sizeOf, readyRow, createArgs, issueNo } from "./autopilot-ui-lib.mjs";

test("cut keeps short text and ends long text in an ellipsis", () => {
  assert.equal(cut("abc", 5), "abc");
  assert.equal(cut("abcdef", 4), "abc…");
  assert.equal(cut("abc", 1), "");
});

test("sizeOf reads a size label first, then the plan comment", () => {
  const plan = { body: "🤖 *Claude:* ## Plan\n**Size M**, two commits" };
  assert.equal(sizeOf({ labels: [{ name: "size:s" }], comments: [plan] }), "S");
  assert.equal(sizeOf({ labels: [], comments: [{ body: "hi" }, plan] }), "M");
  assert.equal(sizeOf({ labels: [], comments: [{ body: "**Size L** but not a plan" }] }), "");
});

test("readyRow aligns number and size and cuts the title to the width", () => {
  const i = { number: 42, title: "A rather long issue title here", labels: [{ name: "size:M" }], comments: [] };
  assert.equal(readyRow(i, 30), "#42    M  A rather long i…");
  assert.equal(readyRow({ ...i, labels: [] }, 60), "#42    ·  A rather long issue title here");
});

test("createArgs passes title and body as separate argv entries", () => {
  assert.deepEqual(createArgs("o/r", "Fix `x`; rm -rf", ""), ["issue", "create", "-R", "o/r", "--title", "Fix `x`; rm -rf", "--body", "Queued by the owner from the autopilot dashboard."]);
  assert.equal(createArgs("o/r", "t", "b").at(-1), "b");
});

test("issueNo reads the number from the gh output", () => {
  assert.equal(issueNo("https://github.com/o/r/issues/123\n"), "123");
  assert.equal(issueNo("nothing"), "");
});
