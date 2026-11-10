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
test("showall and no headings fold nothing", () => {
  assert.deepEqual(startupFolds(doc, "showall"), []);
  assert.deepEqual(startupFolds(["a", "b"], "overview"), []);
});
