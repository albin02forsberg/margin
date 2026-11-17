import { test } from "node:test";
import assert from "node:assert/strict";
import { stale } from "./undo.ts";

test("stale flags only files edited since the recorded write", () => {
  const recs = [{ path: "a", after: "x" }, { path: "b", after: "y" }];
  assert.deepEqual(stale(recs, new Map([["a", "x"], ["b", "y"]])), []);
  assert.deepEqual(stale(recs, new Map([["a", "x"], ["b", "y!"]])), ["b"]);
  assert.deepEqual(stale(recs, new Map([["a", "x"], ["b", null]])), ["b"]);
  assert.deepEqual(stale(recs, new Map([["a", "x"]])), ["b"]);
});
