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

import { tasksToChecklist as t } from "./checklist.ts";
test("tasks -> checklist keywords", () => {
  assert.deepEqual(t(["** TODO a", "** DONE b", "** WAIT c :x:", "** plain"], ["TODO", "WAIT"], ["DONE"]),
    ["- [ ] a", "- [X] b", "- [ ] c :x:", "- plain"]);
});
test("tasks -> checklist body and dropped metadata", () => {
  assert.deepEqual(t(["* TODO a", "SCHEDULED: <2026-01-01>", ":PROPERTIES:", ":ID: x", ":END:", "note"], ["TODO"], ["DONE"]),
    ["- [ ] a", "  note"]);
});
test("tasks -> checklist refusals", () => {
  assert.equal(t(["* TODO a", "** TODO b"], ["TODO"], ["DONE"]), null);
  assert.equal(t(["* TODO a", ":LOGBOOK:", ":END:"], ["TODO"], ["DONE"]), null);
  assert.equal(t(["* TODO a", ":PROPERTIES:", ":FOO: 1", ":END:"], ["TODO"], ["DONE"]), null);
  assert.equal(t(["text"], ["TODO"], ["DONE"]), null);
});
test("round trip", () => {
  const l = ["- [ ] a", "  body", "- [X] b"];
  assert.deepEqual(t(c(l, 1, "TODO", "DONE"), ["TODO"], ["DONE"]), l);
});
