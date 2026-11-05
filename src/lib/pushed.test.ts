import { test } from "node:test";
import assert from "node:assert/strict";
import { cancelKeyword, isPushed, PUSHED_MIN, stuckMenu } from "./pushed.ts";

test("isPushed flags at the threshold", () => {
  assert.equal(PUSHED_MIN, 3);
  assert.equal(isPushed(undefined), false);
  assert.equal(isPushed(2), false);
  assert.equal(isPushed(3), true);
  assert.equal(isPushed(7), true);
});

test("stuck menu only for pushed rows; drop needs a cancel keyword", () => {
  assert.deepEqual(stuckMenu(2, ["DONE", "CANCELLED"]).keys, {});
  assert.deepEqual(stuckMenu(3, ["DONE", "CANCELLED"]).keys, { p: "park", d: "drop", b: "split" });
  assert.deepEqual(stuckMenu(4, ["DONE"]).keys, { p: "park", b: "split" });
  assert.equal(cancelKeyword(["DONE", "CANCELED"]), "CANCELED");
  assert.equal(cancelKeyword(["DONE"]), undefined);
});
