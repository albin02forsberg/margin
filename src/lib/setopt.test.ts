import { test } from "node:test";
import assert from "node:assert/strict";
import { applySet } from "./setopt.ts";

const off = { number: false, relativenumber: false };
test("applySet", () => {
  assert.deepEqual(applySet(off, ["nu"]), { number: true, relativenumber: false });
  assert.deepEqual(applySet(off, ["number", "rnu"]), { number: true, relativenumber: true });
  assert.deepEqual(applySet({ number: true, relativenumber: true }, ["nonumber", "nornu"]), off);
  assert.deepEqual(applySet(off, ["bogus"]), off);
});
