import test from "node:test";
import assert from "node:assert/strict";
import { findTarget, splitTarget } from "./links.ts";

const doc = "#+title: T\n* TODO [#A] Install :tag:\n:PROPERTIES:\n:CUSTOM_ID: setup\n:END:\nbody text\n** Usage\r\nmore\n";

test("splitTarget", () => {
  assert.deepEqual(splitTarget("file:a.org::*Foo"), { file: "a.org", search: "*Foo" });
  assert.deepEqual(splitTarget("file:a.org"), { file: "a.org", search: "" });
  assert.deepEqual(splitTarget("*Install"), { file: "", search: "*Install" });
  assert.deepEqual(splitTarget("#setup"), { file: "", search: "#setup" });
  assert.deepEqual(splitTarget("C:\\n\\a.org::5"), { file: "C:\\n\\a.org", search: "5" });
  assert.deepEqual(splitTarget("a.org"), { file: "a.org", search: "" });
});

test("findTarget", () => {
  assert.equal(findTarget(doc, "*install"), 1);
  assert.equal(findTarget(doc, "*Usage"), 6);
  assert.equal(findTarget(doc, "*Nope"), null);
  assert.equal(findTarget(doc, "#setup"), 1);
  assert.equal(findTarget(doc, "#nope"), null);
  assert.equal(findTarget(doc, "3"), 2);
  assert.equal(findTarget(doc, "999"), 8);
  assert.equal(findTarget(doc, "body"), 5);
  assert.equal(findTarget(doc, ""), null);
});

test("findTarget: custom keywords and statistics cookies", () => {
  const d = "* DOING Write report\n* Release [2/5]\n* TODO Ship   it [40%] :work:\n* C++ [/]\n";
  assert.equal(findTarget(d, "*Write report"), null);
  assert.equal(findTarget(d, "*DOING Write report"), 0);
  assert.equal(findTarget(d, "*Write report", ["TODO", "DOING", "DONE"]), 0);
  assert.equal(findTarget(d, "*Ship it", ["DOING"]), null);
  assert.equal(findTarget(d, "*Ship it"), 2);
  assert.equal(findTarget(d, "*Release"), 1);
  assert.equal(findTarget(d, "*Release [2/5]"), 1);
  assert.equal(findTarget(d, "*Release [3/5]"), 1);
  assert.equal(findTarget(d, "*C++", ["C++", "DONE"]), null);
  assert.equal(findTarget(d, "*c++"), 3);
  assert.equal(findTarget("* A+ Go\n", "*Go", ["A+"]), 0);
});
