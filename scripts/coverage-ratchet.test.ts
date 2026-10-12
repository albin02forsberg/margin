// Run: npm test
import { test } from "node:test";
import assert from "node:assert/strict";
import { parseLcov, regressions, summarize } from "./coverage-ratchet.ts";

const lcov = `SF:/repo/src-tauri/src/a.rs
LF:10
LH:5
end_of_record
SF:/repo/src-tauri/src/main.rs
LF:3
LH:0
end_of_record
SF:src/lib/b.ts
BRF:4
BRH:3
LH:2
LF:2
end_of_record
`;

test("parseLcov makes paths relative and sums lines and branches", () => {
  const r = parseLcov(lcov, "/repo");
  assert.deepEqual(Object.keys(r), ["src-tauri/src/a.rs", "src-tauri/src/main.rs", "src/lib/b.ts"]);
  assert.deepEqual(r["src/lib/b.ts"], { lines: { hit: 2, found: 2 }, branches: { hit: 3, found: 4 } });
});

test("summarize drops excluded files and omits branches without data", () => {
  const r = parseLcov(lcov, "/repo");
  assert.deepEqual(summarize(r, { "src-tauri/src/main.rs": "entry point" }), {
    lines: 58.33, branches: 75, files: { "src-tauri/src/a.rs": 50, "src/lib/b.ts": 100 },
  });
  const rust = summarize(parseLcov(lcov.split("SF:src/lib")[0], "/repo"), {});
  assert.equal(rust.lines, 38.46);
  assert.equal("branches" in rust, false);
});

test("regressions fail only below baseline minus tolerance", () => {
  const base = { lines: 80, branches: 70, files: { "a.rs": 50, "gone.rs": 90 } };
  assert.deepEqual(regressions("rust", { lines: 79.6, branches: 70, files: { "a.rs": 50, "new.rs": 0 } }, base, 0.5), []);
  assert.deepEqual(regressions("rust", { lines: 79.4, branches: 69, files: { "a.rs": 49.4 } }, base, 0.5), [
    "rust lines: 79.4% < baseline 80%",
    "rust branches: 69% < baseline 70%",
    "rust a.rs lines: 49.4% < baseline 50%",
  ]);
  assert.deepEqual(regressions("rust", { lines: 0, files: {} }, undefined, 0.5), [], "no baseline never fails");
});
