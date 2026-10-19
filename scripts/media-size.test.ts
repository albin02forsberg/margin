// Run: npm test
import { test } from "node:test";
import assert from "node:assert/strict";
import { problems } from "./media-size.ts";

test("problems is empty when every clip is there and small", () => {
  assert.deepEqual(problems({ "docs/gifs/a.gif": 10, "docs/screenshots/x.png": 5 }, ["a"], 100, 1000), []);
});

test("problems names missing clips, oversized files and an oversized total", () => {
  const files = { "docs/gifs/a.gif": 1024 * 1024 * 2, "docs/screenshots/x.png": 1024 * 1024 };
  assert.deepEqual(problems(files, ["a", "b"], 1.5 * 1024 * 1024, 2.5 * 1024 * 1024), [
    "docs/gifs/b.gif is missing",
    "docs/gifs/a.gif is 2.00 MB, over 1.50 MB",
    "README media total 3.00 MB, over 2.50 MB",
  ]);
});
