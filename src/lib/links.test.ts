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
