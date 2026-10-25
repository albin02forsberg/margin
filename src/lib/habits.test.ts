import { test } from "node:test";
import assert from "node:assert/strict";
import { spark } from "./habits.ts";

test("spark bins a habit bar into weeks ending today", () => {
  assert.equal(spark("xxxxxxx"), "█");
  assert.equal(spark("......."), "▁");
  // 10 days: an older 3-day chunk (2 of 3 done), then the last 7 (none).
  assert.equal(spark("xx.!!!!!!!"), "▆▁");
  assert.equal(spark(".x.x.x.x.x.x.x"), "▄▅");
  assert.equal(spark(""), "");
});
