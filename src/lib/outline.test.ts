import { test } from "node:test";
import assert from "node:assert/strict";
import { buildOutline, headingAt, visibleOutline } from "./outline.ts";

const doc = "#+title: N\nintro\n* TODO [#A] Plan the trip [1/3] :travel:work:\nbody\n** DONE Book [[file:x.org][hotel]]\n*** Confirm\n** NEXT Pack\n* Notes\n*bold* not a heading\n* :solo:\n";
const items = buildOutline(doc, ["TODO", "NEXT"], ["DONE"]);

test("buildOutline: levels, keywords, cookies and tags", () => {
  assert.deepEqual(items.map((i) => [i.line, i.level, i.title, i.keyword, i.done, i.cookie, i.tags]), [
    [2, 1, "Plan the trip", "TODO", false, "[1/3]", ["travel", "work"]],
    [4, 2, "Book hotel", "DONE", true, null, []],
    [5, 3, "Confirm", null, false, null, []],
    [6, 2, "Pack", "NEXT", false, null, []],
    [7, 1, "Notes", null, false, null, []],
    [9, 1, "", null, false, null, ["solo"]],
  ]);
  // Only configured keywords count; a percent cookie works too.
  assert.deepEqual(buildOutline("* WAIT Call [50%]", ["TODO"], ["DONE"]).map((i) => [i.title, i.keyword, i.cookie]), [["WAIT Call", null, "[50%]"]]);
});

test("headingAt: the heading containing the line", () => {
  assert.equal(headingAt(items, 0), -1); // before the first heading
  assert.equal(headingAt(items, 2), 0);
  assert.equal(headingAt(items, 3), 0); // body between headings
  assert.equal(headingAt(items, 5), 2);
  assert.equal(headingAt(items, 100), 5);
  assert.equal(headingAt([], 3), -1);
});

test("visibleOutline: filter by title or tag, hide done subtrees", () => {
  const t = (xs: typeof items) => xs.map((i) => i.title);
  assert.equal(visibleOutline(items, "", false).length, items.length);
  assert.deepEqual(t(visibleOutline(items, "", true)), ["Plan the trip", "Pack", "Notes", ""]);
  assert.deepEqual(t(visibleOutline(items, "PACK", false)), ["Pack"]);
  assert.deepEqual(t(visibleOutline(items, "travel", false)), ["Plan the trip"]);
  assert.deepEqual(t(visibleOutline(items, "confirm", true)), []); // inside a hidden done subtree
});
