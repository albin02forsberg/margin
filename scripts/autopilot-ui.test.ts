// Run: npm test
import { test } from "node:test";
import assert from "node:assert/strict";
import { cut, sizeOf, readyRow, createArgs, issueNo, vlen, padVisible, columns, budget, clip } from "./autopilot-ui-lib.mjs";

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

test("vlen ignores ANSI codes and counts emoji as two cells", () => {
  assert.equal(vlen("\x1b[1mab\x1b[0m"), 2);
  assert.equal(vlen("💤 x"), 4);
  assert.equal(vlen("⠋x"), 2);
});

test("padVisible pads by visible width", () => {
  assert.equal(padVisible("\x1b[31mab\x1b[0m", 4), "\x1b[31mab\x1b[0m  ");
  assert.equal(padVisible("abcd", 2), "abcd");
});

test("columns pads the left side and fills the shorter side", () => {
  assert.deepEqual(columns(["a", "bb"], ["1"], 3, "|"), ["a  |1", "bb |"]);
});

test("budget drops Recent to its minimum, then totals, then cuts Up next", () => {
  assert.deepEqual(budget(30, 6, 2, 3, 8), { up: 6, totals: 2, recent: 8 });
  assert.deepEqual(budget(10, 6, 2, 3, 8), { up: 6, totals: 0, recent: 4 });
  assert.deepEqual(budget(8, 6, 2, 3, 8), { up: 5, totals: 0, recent: 3 });
  assert.deepEqual(budget(2, 6, 2, 3, 8), { up: 1, totals: 0, recent: 3 });
});

test("clip shows how many lines were cut", () => {
  const more = (n: number) => `+${n}`;
  assert.deepEqual(clip(["a", "b"], 3, more), ["a", "b"]);
  assert.deepEqual(clip(["a", "b", "c", "d"], 2, more), ["a", "+3"]);
});

test("issueNo reads the number from the gh output", () => {
  assert.equal(issueNo("https://github.com/o/r/issues/123\n"), "123");
  assert.equal(issueNo("nothing"), "");
});
