import { test } from "node:test";
import assert from "node:assert/strict";
import { pickStale } from "./stale.ts";

const items: [string, string, number][] = [["old", "", 0], ["new", "", 990]];
test("empty gives empty", () => assert.equal(pickStale([], 1000, 0.5), ""));
test("older notes own most of the range", () => {
  assert.equal(pickStale(items, 1000, 0), "old");
  assert.equal(pickStale(items, 1000, 0.9), "old");
  assert.equal(pickStale(items, 1000, 0.999), "new");
});
