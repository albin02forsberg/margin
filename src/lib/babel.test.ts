import { test } from "node:test";
import assert from "node:assert/strict";
import { srcAt, resultsChange } from "./babel.ts";

const apply = (text: string, c: { from: number; to: number; insert: string }) => text.slice(0, c.from) + c.insert + text.slice(c.to);
const run = (text: string, pos: number, out: string) => apply(text, resultsChange(text, srcAt(text, pos)!, out));

const doc = "* H\n#+begin_src sh :results output\nls\n,* x\n,#+end_src\n#+end_src\nafter";

test("srcAt finds the enclosing block, its begin and end lines included", () => {
  const s = srcAt(doc, doc.indexOf("ls"))!;
  assert.deepEqual(s, { lang: "sh", code: "ls\n* x\n#+end_src", indent: "", end: doc.indexOf("\nafter") });
  assert.equal(srcAt(doc, doc.indexOf("#+begin"))?.lang, "sh");
  assert.equal(srcAt(doc, doc.indexOf("\nafter"))?.lang, "sh", "end of the #+end_src line");
  assert.equal(srcAt(doc, 0), null);
  assert.equal(srcAt(doc, doc.indexOf("after")), null);
  assert.equal(srcAt("#+begin_src sh\necho", 16), null, "unclosed");
  assert.equal(srcAt("#+BEGIN_SRC\n#+END_SRC", 0)?.code, "", "empty, no language");
  assert.equal(srcAt("#+begin_quote\nx\n#+end_quote", 15), null, "not a src block");
});

test("resultsChange inserts #+RESULTS: after a blank line, as : lines", () => {
  const t = "#+begin_src sh\necho\n#+end_src\nnext";
  assert.equal(run(t, 0, "a\n\n#+end_src\n"), "#+begin_src sh\necho\n#+end_src\n\n#+RESULTS:\n: a\n:\n: #+end_src\nnext");
  assert.equal(run("#+begin_src sh\n#+end_src", 0, ""), "#+begin_src sh\n#+end_src\n\n#+RESULTS:", "at the end of the doc, empty output");
  assert.equal(run("  #+begin_src sh\n  #+end_src\n", 0, "x"), "  #+begin_src sh\n  #+end_src\n\n  #+RESULTS:\n  : x\n", "indented");
});

test("resultsChange replaces the results that follow, shorter or longer", () => {
  const t = "#+begin_src sh\n#+end_src\n\n#+RESULTS:\n: old 1\n: old 2\n:\n\ntext";
  assert.equal(run(t, 0, "new"), "#+begin_src sh\n#+end_src\n\n#+RESULTS:\n: new\n\ntext");
  assert.equal(run(t, 0, "1\n2\n3\n4"), "#+begin_src sh\n#+end_src\n\n#+RESULTS:\n: 1\n: 2\n: 3\n: 4\n\ntext");
  const direct = "#+begin_src sh\n#+end_src\n#+results[ab12]:\n: old\n:not a result line";
  assert.equal(run(direct, 0, "n"), "#+begin_src sh\n#+end_src\n#+RESULTS:\n: n\n:not a result line");
  const other = "#+begin_src sh\n#+end_src\n\n* Heading\n#+RESULTS:\n: x";
  assert.equal(run(other, 0, "n"), "#+begin_src sh\n#+end_src\n\n#+RESULTS:\n: n\n\n* Heading\n#+RESULTS:\n: x", "only results right after the block");
});
