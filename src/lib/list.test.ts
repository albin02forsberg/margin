// Run: npm test
import { test } from "node:test";
import assert from "node:assert/strict";
import { renumber, itemSwap } from "./list.ts";

/** itemSwap at 1-based line I of TEXT, direction D. */
const swap = (text: string, i: number, d: -1 | 1) => { const l = text.split("\n"); return itemSwap((j) => l[j - 1], i, d); };

test("itemSwap: item with its nested lines swaps with its sibling", () => {
  assert.deepEqual(swap("- a\n  - a1\n  text\n- b", 1, 1), [[1, 3], [4, 4]]);
  assert.deepEqual(swap("- a\n  - a1\n- b\n  - b1", 3, -1), [[1, 2], [3, 4]]);
});

test("itemSwap: a nested item only moves among its siblings", () => {
  assert.equal(swap("- a\n  - a1\n  - a2\n- b", 3, 1), "end");
  assert.deepEqual(swap("- a\n  - a1\n  - a2\n- b", 2, 1), [[2, 2], [3, 3]]);
});

test("itemSwap: ends, headings, non-items and continuation lines", () => {
  assert.equal(swap("- a\n- b", 1, -1), "end");
  assert.equal(swap("- a\n- b", 2, 1), "end");
  assert.equal(swap("* H\n- a", 1, 1), null);
  assert.equal(swap("- a\n* H\n- b", 1, 1), "end");
  assert.equal(swap("- a\n  more\n- b", 2, 1), null);
});

test("itemSwap: one blank line between items is fine, two end the list", () => {
  assert.deepEqual(swap("- a\n\n- b", 1, 1), [[1, 1], [3, 3]]);
  assert.equal(swap("- a\n\n\n- b", 1, 1), "end");
});

/** Apply renumber at line I of TEXT, return the new text. */
function fix(text: string, i: number) {
  const lines = text.split("\n");
  for (const c of renumber((j) => lines[j], i)) lines[c.line] = lines[c.line].slice(0, c.from) + c.insert + lines[c.line].slice(c.to);
  return lines.join("\n");
}

test("a new item in the middle shifts the ones below", () => {
  assert.equal(fix("1. a\n2. b\n3. new\n3. c\n4. d", 2), "1. a\n2. b\n3. new\n4. c\n5. d");
});

test("keeps the first item's start number and the delimiter", () => {
  assert.equal(fix("5) a\n7) b\n7) c", 2), "5) a\n6) b\n7) c");
});

test("skips deeper lines and single blanks, stops at two blanks", () => {
  const t = "1. a\n   text\n   1. sub\n   1. sub\n\n1. b\n\n\n1. other";
  assert.equal(fix(t, 5), "1. a\n   text\n   1. sub\n   1. sub\n\n2. b\n\n\n1. other");
});

test("stops at a heading, a bullet or text at the same indent", () => {
  assert.equal(fix("* H\n1. a\n1. b\n- x\n1. c", 1), "* H\n1. a\n2. b\n- x\n1. c");
  assert.equal(fix("intro\n3. a\n3. b", 2), "intro\n3. a\n4. b");
});

test("only touches items at the cursor's indent, and not other lines", () => {
  assert.equal(fix("1. a\n  1. x\n  1. y\n2. b", 2), "1. a\n  1. x\n  2. y\n2. b");
  assert.deepEqual(renumber((j) => ["- a", "text"][j], 0), []);
  assert.deepEqual(renumber((j) => ["1. a", "2. b"][j], 1), []);
});
