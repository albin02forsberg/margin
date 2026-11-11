import test from "node:test";
import assert from "node:assert/strict";
import { headProp, startupFolds, startupMode } from "./startup.ts";

test("startupMode", () => {
  assert.equal(startupMode("#+TITLE: x\n#+startup: Overview hideblocks\n* A"), "overview");
  assert.equal(startupMode("#+STARTUP: content"), "content");
  assert.equal(startupMode("#+STARTUP: indent overview"), "overview");
  assert.equal(startupMode("#+STARTUP: bogus"), "showall");
  assert.equal(startupMode("#+STARTUP: showeverything"), "showeverything");
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

const P = (v: string) => [":PROPERTIES:", `:VISIBILITY: ${v}`, ":END:"];
test("headProp reads the drawer under a heading, after one planning line", () => {
  const d = ["* A", "SCHEDULED: <2026-01-01>", ":PROPERTIES:", ":ID: x1", ":VISIBILITY: folded", ":END:", "* B", ":ID: no"];
  assert.equal(headProp(d, 1, "ID"), "x1");
  assert.equal(headProp(d, 1, "visibility"), "folded");
  assert.equal(headProp(d, 7, "ID"), undefined);
});
// lines: 1 * A, 2-4 drawer, 5 body, 6 ** A1, 7 x, 8 *** A11, 9 y, 10 * B, 11 b
const vis = (v: string) => ["* A", ...P(v), "body", "** A1", "x", "*** A11", "y", "* B", "b"];
test(":VISIBILITY: folded / children / content / all under showall", () => {
  assert.deepEqual(startupFolds(vis("folded"), "showall"), [{ from: 1, to: 9 }]);
  assert.deepEqual(startupFolds(vis("children"), "showall"), [{ from: 6, to: 9 }]);
  assert.deepEqual(startupFolds(vis("content"), "showall"), [{ from: 1, to: 5 }, { from: 6, to: 7 }, { from: 8, to: 9 }]);
  assert.deepEqual(startupFolds(vis("all"), "showall"), []);
  assert.deepEqual(startupFolds(vis("bogus"), "showall"), []);
});
test(":VISIBILITY: replaces the file-wide folds inside its subtree only", () => {
  assert.deepEqual(startupFolds(vis("all"), "overview"), [{ from: 10, to: 11 }]);
  assert.deepEqual(startupFolds(vis("children"), "overview"), [{ from: 6, to: 9 }, { from: 10, to: 11 }]);
  assert.deepEqual(startupFolds(vis("folded"), "showeverything"), []);
});
test("nested :VISIBILITY: inside a handled subtree is skipped; ancestors keep their folds", () => {
  const d = ["* A", ...P("all"), "** A1", ...P("folded"), "x"];
  assert.deepEqual(startupFolds(d, "showall"), []);
  const e = ["* A", "** A1", ...P("folded"), "x", "** A2", "y"];
  assert.deepEqual(startupFolds(e, "overview"), [{ from: 1, to: 8 }, { from: 2, to: 6 }]);
  assert.deepEqual(startupFolds(e, "showall"), [{ from: 2, to: 6 }]);
});

test(":VISIBILITY: children under overview drops the outer fold and keeps the rest", () => {
  const d = ["* A", ...P("children"), "** B", "b", "** C", "c", "* D", "d"];
  assert.deepEqual(startupFolds(d, "overview"), [{ from: 5, to: 6 }, { from: 7, to: 8 }, { from: 9, to: 10 }]);
});
