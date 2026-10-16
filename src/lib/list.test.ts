// Run: npm test
import { test } from "node:test";
import assert from "node:assert/strict";
import { renumber } from "./list.ts";

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
