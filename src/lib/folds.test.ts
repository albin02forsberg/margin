import test from "node:test";
import assert from "node:assert/strict";
import { foldsToStore, headingKeys, sameKeys, keyLines, parseFolds, putFolds } from "./folds.ts";

const doc = ["#+title: T", "* Notes", "text", "** Notes", "* Notes", "* Task", "SCHEDULED: <2026-11-10 Tue>", ":PROPERTIES:", ":ID: abc", ":END:", "body"];

test("headingKeys: occurrence index for duplicate titles, :ID: when present", () => {
  assert.deepEqual(headingKeys(doc, [2, 3, 4, 5, 6]), ["h:*|Notes#0", "h:**|Notes#0", "h:*|Notes#1", "id:abc"]);
});

test("keyLines: survives lines shifting and renamed headings, drops stale keys", () => {
  const keys = headingKeys(doc, [5, 6]);
  const edited = ["new line", ...doc.slice(0, 5), "* Renamed task", ...doc.slice(6)];
  assert.deepEqual(keyLines(edited, [...keys, "h:*|Gone#0"]), [6, 7]);
  assert.deepEqual(keyLines(doc, []), []);
});

test("parseFolds validates; putFolds keeps the newest MAX", () => {
  assert.deepEqual(parseFolds("nope"), {});
  assert.deepEqual(parseFolds("[1]"), {});
  assert.deepEqual(parseFolds('{"a":["x"],"b":[1],"c":"x"}'), { a: ["x"] });
  let m = putFolds({}, "/a", ["1"], 2);
  m = putFolds(m, "/b", [], 2);
  m = putFolds(m, "/a", ["2"], 2);
  m = putFolds(m, "/c", ["3"], 2);
  assert.deepEqual(m, { "/a": ["2"], "/c": ["3"] });
});

test("keys ignore TODO keyword, priority and tags but not other words", () => {
  const k = (t: string) => headingKeys([t], [1])[0];
  assert.equal(k("* TODO [#A] Write :work:home:"), "h:*|Write#0");
  assert.equal(k("* DONE Write"), k("* Write"));
  assert.equal(k("* API notes"), "h:*|API notes#0");
  assert.equal(headingKeys(["* NEXT Go"], [1], ["NEXT"])[0], "h:*|Go#0");
  assert.equal(k("* Status [1/3]"), "h:*|Status [1/3]#0");
});

test("sameKeys ignores order", () => {
  assert.ok(sameKeys(["a", "b"], ["b", "a"]));
  assert.ok(!sameKeys(["a"], ["a", "b"]));
  assert.ok(!sameKeys(["a", "a"], ["a", "b"]));
});

test("foldsToStore: null when equal to the #+STARTUP: folds", () => {
  const ov = "#+STARTUP: overview\n* A\nx\n** B\ny\n* C\nz";
  assert.equal(foldsToStore(ov, ["h:*|A#0", "h:*|C#0"]), null);
  assert.deepEqual(foldsToStore(ov, []), []);
  assert.equal(foldsToStore("* A\nx", []), null);
  assert.deepEqual(foldsToStore("* A\nx", ["h:*|A#0"]), ["h:*|A#0"]);
});

test("sameKeys is set equality with equal lengths", () => {
  assert.ok(sameKeys([], []));
  assert.ok(sameKeys(["a", "a"], ["a", "a"]));
  assert.ok(sameKeys(["a", "a", "b"], ["a", "b", "b"]));
  assert.ok(!sameKeys(["a", "b"], ["a", "c"]));
});

test("foldsToStore stays correct and fast on 10000 headings", () => {
  const doc = "#+STARTUP: overview\n" + Array.from({ length: 10000 }, (_, i) => `* H${i}\nbody`).join("\n");
  const lines = doc.split("\n");
  const all = headingKeys(lines, lines.map((_, i) => i + 1));
  const t = Date.now();
  assert.equal(foldsToStore(doc, all.slice().reverse()), null);
  assert.deepEqual(foldsToStore(doc, all.slice(1)), all.slice(1));
  assert.ok(Date.now() - t < 1000);
});
