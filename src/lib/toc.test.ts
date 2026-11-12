import { test } from "node:test";
import assert from "node:assert/strict";
import { tocDepth, tocEntries } from "./toc.ts";

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
