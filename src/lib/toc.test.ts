import { test } from "node:test";
import assert from "node:assert/strict";
import { tocDepth, tocDirective, tocEntries } from "./toc.ts";

const T = ["TODO"], D = ["DONE"];
test("tocDepth", () => {
  assert.equal(tocDepth("#+TOC: headlines"), 0);
  assert.equal(tocDepth("#+toc: headlines 2"), 2);
  assert.equal(tocDepth("#+TOC: tables"), null);
});
test("tocEntries strips keyword, priority, cookie, tags; skips blocks, noexport and COMMENT", () => {
  const doc = "#+TOC: headlines\n* TODO [#A] Intro [1/2] :tag:\n*** Deep\n#+begin_src sh\n* not\n#+end_src\n* Hidden :noexport:\n** Gone\n* DONE COMMENT Draft\n** Gone2\n* Commentary\n* Last";
  const e = tocEntries(doc, T, D);
  assert.deepEqual(e.map((x) => [x.line, x.level, x.title]), [[1, 1, "Intro"], [2, 3, "Deep"], [10, 1, "Commentary"], [11, 1, "Last"]]);
  assert.deepEqual(tocEntries(doc, T, D, 2).map((x) => x.title), ["Intro", "Commentary", "Last"]);
});
test("local TOC lists the subtree of the heading above it, depth counted from it", () => {
  assert.deepEqual(tocDirective("#+TOC: headlines 1 LOCAL"), { depth: 1, local: true });
  assert.deepEqual(tocDirective("#+TOC: headlines"), { depth: 0, local: false });
  const doc = "#+TOC: headlines local\n* A\n** A1\n#+TOC: headlines local\n*** A11\n**** A111\n** A2\n* B\n";
  const t = (at: number, depth = 0) => tocEntries(doc, T, D, depth, at).map((x) => x.title);
  assert.deepEqual(t(0), ["A", "A1", "A11", "A111", "A2", "B"]); // above any heading: whole document
  assert.deepEqual(t(3), ["A11", "A111"]);
  assert.deepEqual(t(3, 1), ["A11"]);
});
