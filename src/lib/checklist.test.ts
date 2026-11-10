import { test } from "node:test";
import assert from "node:assert/strict";
import { checklistToTasks as c, levelAt, listRange } from "./checklist.ts";

test("flat list, X and space", () => {
  assert.deepEqual(c(["- [ ] a", "- [X] b", "- [-] c @bob <2026-01-01>"], 2, "TODO", "DONE"),
    ["** TODO a", "** DONE b", "** TODO c @bob <2026-01-01>"]);
});
test("nested items and text stay", () => {
  assert.deepEqual(c(["intro", "- [ ] a", "  - [ ] sub", "  more", "+ [x] b"], 1, "T", "D"),
    ["intro", "* T a", "  - [ ] sub", "  more", "* D b"]);
});
test("level from heading above", () => {
  assert.equal(levelAt(["* A", "** B", "- [ ] x"], 2), 3);
  assert.equal(levelAt(["- [ ] x"], 0), 1);
});
test("range stops at blank lines and headings", () => {
  const l = ["* H", "text", "- [ ] a", "- [ ] b", "", "- [ ] c", "* N", "- [ ] d"];
  assert.deepEqual(listRange(l, 2), [1, 3]);
  assert.deepEqual(listRange(l, 7), [7, 7]);
});
