// Run: npm test
import { test } from "node:test";
import assert from "node:assert/strict";
import { layout } from "./graph.ts";

test("layout pins the centre, keeps nodes apart and linked nodes closer", () => {
  const p = layout(4, [[0, 1], [0, 2], [1, 2]]);
  assert.deepEqual(p[0], { x: 0, y: 0 });
  assert.ok(p.every((q) => Number.isFinite(q.x) && Number.isFinite(q.y)));
  const dist = (a: number, b: number) => Math.hypot(p[a].x - p[b].x, p[a].y - p[b].y);
  assert.ok(dist(1, 2) > 20);
  assert.ok(dist(0, 1) < dist(0, 3)); // 3 is unlinked
  assert.deepEqual(layout(4, [[0, 1]]), layout(4, [[0, 1]]));
});
