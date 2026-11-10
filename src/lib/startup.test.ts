import test from "node:test";
import assert from "node:assert/strict";
import { startupFolds, startupMode } from "./startup.ts";

test("startupMode", () => {
  assert.equal(startupMode("#+TITLE: x\n#+startup: Overview hideblocks\n* A"), "overview");
  assert.equal(startupMode("#+STARTUP: content"), "content");
  assert.equal(startupMode("#+STARTUP: indent overview"), "overview");
  assert.equal(startupMode("#+STARTUP: bogus"), "showall");
  assert.equal(startupMode("#+STARTUP: showeverything"), "showall");
  assert.equal(startupMode(""), "showall");
  assert.equal(startupMode("text\n#+STARTUP: overview"), "showall");
});

const doc = ["pre", "* A", "body", "** A1", "x", "* B", "** B1", "* C"];
test("overview folds top-level subtrees", () => {
  assert.deepEqual(startupFolds(doc, "overview"), [{ from: 2, to: 5 }, { from: 6, to: 7 }]);
});
test("content folds each heading body only", () => {
  assert.deepEqual(startupFolds(doc, "content"), [{ from: 2, to: 3 }, { from: 4, to: 5 }]);
});
test("fold/nofold/show<N>levels tokens", () => {
  assert.equal(startupMode("#+STARTUP: fold"), "overview");
  assert.equal(startupMode("#+STARTUP: nofold"), "showall");
  assert.equal(startupMode("#+STARTUP: show3levels"), 3);
  assert.equal(startupMode("#+STARTUP: show9levels"), "showall");
});
test("show2levels hides bodies of level 1 and subtrees of level 2", () => {
  const d = ["* A", "body", "** A1", "x", "*** A11", "y", "* B", "** B1", "z"];
  assert.deepEqual(startupFolds(d, 2), [{ from: 1, to: 2 }, { from: 3, to: 6 }, { from: 8, to: 9 }]);
  assert.deepEqual(startupFolds(d, 1), startupFolds(d, "overview"));
  assert.deepEqual(startupFolds(d, 5), startupFolds(d, "content"));
});
test("showall and no headings fold nothing", () => {
  assert.deepEqual(startupFolds(doc, "showall"), []);
  assert.deepEqual(startupFolds(["a", "b"], "overview"), []);
});
