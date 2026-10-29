import { test } from "node:test";
import assert from "node:assert/strict";
import { back, drop, forward, newHistory, push } from "./navhistory.ts";

const A = { path: "a", line: 1 }, B = { path: "b", line: 2 }, C = { path: "c", line: 3 };

test("back and forward round trip", () => {
  const h = newHistory();
  push(h, A); push(h, B);
  assert.deepEqual(back(h, C), B);
  assert.deepEqual(back(h, B), A);
  assert.equal(back(h, A), undefined);
  assert.deepEqual(forward(h, A), B);
  assert.deepEqual(forward(h, B), C);
  assert.equal(forward(h, C), undefined);
});

test("push truncates forward, dedupes, caps", () => {
  const h = newHistory();
  push(h, A); back(h, B);
  push(h, C);
  assert.equal(forward(h, C), undefined);
  push(h, C);
  assert.equal(h.back.length, 1);
  for (let i = 0; i < 80; i++) push(h, { path: "x", line: i });
  assert.equal(h.back.length, 50);
  assert.equal(h.back[0].line, 30);
});

test("drop removes vanished notes", () => {
  const h = newHistory();
  push(h, A); push(h, B); back(h, C);
  drop(h, (p) => p !== "b");
  assert.deepEqual(h.back, [A]);
  assert.deepEqual(h.fwd, [C]);
});
