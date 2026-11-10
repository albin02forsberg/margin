import test from "node:test";
import assert from "node:assert/strict";
import { headingKeys, keyLines, parseFolds, putFolds } from "./folds.ts";

const doc = ["#+title: T", "* Notes", "text", "** Notes", "* Notes", "* Task", "SCHEDULED: <2026-11-10 Tue>", ":PROPERTIES:", ":ID: abc", ":END:", "body"];

test("headingKeys: occurrence index for duplicate titles, :ID: when present", () => {
  assert.deepEqual(headingKeys(doc, [2, 3, 4, 5, 6]), ["h:* Notes#0", "h:** Notes#0", "h:* Notes#1", "id:abc"]);
});

test("keyLines: survives lines shifting and renamed headings, drops stale keys", () => {
  const keys = headingKeys(doc, [5, 6]);
  const edited = ["new line", ...doc.slice(0, 5), "* Renamed task", ...doc.slice(6)];
  assert.deepEqual(keyLines(edited, [...keys, "h:* Gone#0"]), [6, 7]);
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
