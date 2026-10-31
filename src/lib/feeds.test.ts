import test from "node:test";
import assert from "node:assert/strict";
import { feedsDue } from "./feeds.ts";

test("feedsDue: off, not yet, due, never, clock backwards", () => {
  assert.equal(feedsDue(0, 0, 1e6), false);
  assert.equal(feedsDue(0, NaN, 1e6), false);
  assert.equal(feedsDue(1000, 1, 1000 + 3599), false);
  assert.equal(feedsDue(1000, 1, 1000 + 3600), true);
  assert.equal(feedsDue(0, 6, 1e6), true);
  assert.equal(feedsDue(5000, 1, 1000), true);
});
