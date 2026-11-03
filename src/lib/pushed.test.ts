import { test } from "node:test";
import assert from "node:assert/strict";
import { isPushed, PUSHED_MIN } from "./pushed.ts";

test("isPushed flags at the threshold", () => {
  assert.equal(PUSHED_MIN, 3);
  assert.equal(isPushed(undefined), false);
  assert.equal(isPushed(2), false);
  assert.equal(isPushed(3), true);
  assert.equal(isPushed(7), true);
});
