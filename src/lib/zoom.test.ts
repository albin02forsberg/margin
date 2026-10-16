import { test } from "node:test";
import assert from "node:assert/strict";
import { nextZoom, parseZoom } from "./zoom.ts";

test("zoom steps by 0.1 without float drift, clamped to 0.5-3", () => {
  let z = 1;
  for (let i = 0; i < 3; i++) z = nextZoom(z, "in");
  assert.equal(z, 1.3);
  assert.equal(nextZoom(0.5, "out"), 0.5);
  assert.equal(nextZoom(3, "in"), 3);
  assert.equal(nextZoom(2.4, "reset"), 1);
});

test("stored zoom: garbage is 1, out of range is clamped", () => {
  for (const raw of [null, "", "abc", "NaN", "-2", "0", "Infinity"]) assert.equal(parseZoom(raw), 1, String(raw));
  assert.equal(parseZoom("1.2"), 1.2);
  assert.equal(parseZoom("9"), 3);
  assert.equal(parseZoom("0.1"), 0.5);
});
