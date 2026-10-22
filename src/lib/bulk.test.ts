import { test } from "node:test";
import assert from "node:assert/strict";
import { markKey, orderForEdit, reresolve, outermost, tagChange } from "./bulk.ts";

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

test("bulk marks follow their task across a refresh (#649)", () => {
  const mk = (path: string, line: number, title: string) => ({ path, line, title });
  const marks = new Map([["/a.org:5", mk("/a.org", 5, "C")]]);
  // line shifted up after A was ticked off
  assert.deepEqual([...reresolve(marks, [mk("/a.org", 3, "B"), mk("/a.org", 4, "C")]).keys()], ["/a.org:4"]);
  // unchanged
  assert.deepEqual([...reresolve(marks, [mk("/a.org", 5, "C")]).keys()], ["/a.org:5"]);
  // deleted / renamed / other file
  assert.equal(reresolve(marks, [mk("/a.org", 5, "D"), mk("/b.org", 5, "C")]).size, 0);
  // duplicate titles: nearest wins, each task claimed once
  const two = new Map([["/a.org:5", mk("/a.org", 5, "T")], ["/a.org:9", mk("/a.org", 9, "T")]]);
  const r = reresolve(two, [mk("/a.org", 3, "T"), mk("/a.org", 8, "T"), mk("/a.org", 20, "T")]);
  assert.deepEqual([...r.keys()].sort(), ["/a.org:3", "/a.org:8"]);
});
