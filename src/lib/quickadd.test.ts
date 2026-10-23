import { test } from "node:test";
import assert from "node:assert/strict";
import { parseQuickAdd } from "./quickadd.ts";

test("tags and priority from the end", () => {
  const q = parseQuickAdd("Call Anna #work #home !a");
  assert.deepEqual([q.title, q.tags, q.priority], ["Call Anna", ["work", "home"], "A"]);
});
test("date candidates longest first", () => {
  const q = parseQuickAdd("Call Anna fri 14:00 #work !A");
  assert.equal(q.title, "Call Anna fri 14:00");
  assert.deepEqual(q.dates, [{ title: "Call Anna", date: "fri 14:00" }, { title: "Call Anna fri", date: "14:00" }]);
});
test("mid-title tokens untouched", () => {
  const q = parseQuickAdd("monday #1 meeting notes");
  assert.deepEqual([q.title, q.tags, q.dates], ["monday #1 meeting notes", [], []]);
});
test("title never empty", () => {
  assert.equal(parseQuickAdd("#work").title, "#work");
  assert.deepEqual(parseQuickAdd("tomorrow").dates, []);
  assert.equal(parseQuickAdd("#").title, "#");
  assert.deepEqual(parseQuickAdd("x #a #a").tags, ["a"]);
});
