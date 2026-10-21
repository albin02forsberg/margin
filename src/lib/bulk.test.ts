import { test } from "node:test";
import assert from "node:assert/strict";
import { markKey, orderForEdit, outermost, tagChange } from "./bulk.ts";

test("bulk order: grouped by file, bottom line first; input untouched", () => {
  const xs = [{ path: "/b.org", line: 2 }, { path: "/a.org", line: 1 }, { path: "/b.org", line: 9 }, { path: "/a.org", line: 7 }];
  assert.deepEqual(orderForEdit(xs).map(markKey), ["/a.org:7", "/a.org:1", "/b.org:9", "/b.org:2"]);
  assert.equal(markKey(xs[0]), "/b.org:2");
  assert.deepEqual(orderForEdit([]), []);
});

test("bulk tags: + adds where missing, - removes where present, nothing toggles", () => {
  assert.deepEqual(tagChange(["a"], { add: true, tag: "b" }), ["a", "b"]);
  assert.deepEqual(tagChange(["a", "b"], { add: true, tag: "b" }), ["a", "b"]);
  assert.deepEqual(tagChange(["a", "b"], { add: false, tag: "a" }), ["b"]);
  assert.deepEqual(tagChange(["b"], { add: false, tag: "a" }), ["b"]);
});

test("bulk move: a marked heading under another marked one goes with its parent", () => {
  const t = "* P\n** C\nbody\n*** G\n* Q\n** R\n";
  assert.deepEqual(outermost(t, [0, 1, 3, 5]), [0, 5]);
  assert.deepEqual(outermost(t, [1, 3, 4]), [1, 4]);
  assert.deepEqual(outermost(t, [3, 5]), [3, 5]);
});
