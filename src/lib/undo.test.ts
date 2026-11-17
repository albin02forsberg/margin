import { test } from "node:test";
import assert from "node:assert/strict";
import { serial, stale } from "./undo.ts";

test("stale flags only files edited since the recorded write", () => {
  const recs = [{ path: "a", after: "x" }, { path: "b", after: "y" }];
  assert.deepEqual(stale(recs, new Map([["a", "x"], ["b", "y"]])), []);
  assert.deepEqual(stale(recs, new Map([["a", "x"], ["b", "y!"]])), ["b"]);
  assert.deepEqual(stale(recs, new Map([["a", "x"], ["b", null]])), ["b"]);
  assert.deepEqual(stale(recs, new Map([["a", "x"]])), ["b"]);
});

test("serial runs overlapping calls in order and survives a rejection", async () => {
  const log: string[] = [];
  const f = serial(async (n: number) => {
    log.push(`start${n}`);
    await new Promise((r) => setTimeout(r, 10 - n * 3));
    if (n === 1) throw new Error("x");
    log.push(`end${n}`);
    return n;
  });
  const rs = await Promise.allSettled([f(1), f(2), f(3)]);
  assert.deepEqual(log, ["start1", "start2", "end2", "start3", "end3"]);
  assert.deepEqual(rs.map((r) => r.status), ["rejected", "fulfilled", "fulfilled"]);
});
