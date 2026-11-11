import { test } from "node:test";
import assert from "node:assert/strict";
import { reparents } from "./checklistfit.ts";

const d = (s: string) => s.split("\n");

test("cursor heading with a child reparents it", () => {
  assert.equal(reparents(d("* P\n** TODO B\n*** c"), 1, 1), true);
});
test("heading above is a deeper sibling's child", () => {
  assert.equal(reparents(d("* P\n** A\n*** A1\n** TODO B\n** TODO C"), 3, 4), true);
});
test("heading above is a same-level sibling", () => {
  assert.equal(reparents(d("* P\n** A\n** TODO B"), 2, 2), true);
});
test("childless siblings directly under the parent, or at the top", () => {
  assert.equal(reparents(d("* P\n** TODO a\n** TODO b\n* Q"), 1, 2), false);
  assert.equal(reparents(d("* TODO a\n* TODO b"), 0, 1), false);
});
