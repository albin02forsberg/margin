// Run: npm test
import { test } from "node:test";
import assert from "node:assert/strict";
import { addFootnote, footnoteTarget, nextLabel } from "./footnote.ts";

/** Apply addFootnote at the | in TEXT; returns the result with | at the cursor. */
function add(text: string) {
  const pos = text.indexOf("|"), plain = text.replace("|", "");
  const r = addFootnote(plain, pos);
  let out = "", last = 0;
  for (const c of [...r.changes].sort((a, b) => a.from - b.from)) { out += plain.slice(last, c.from) + c.insert; last = c.from; }
  out += plain.slice(last);
  return out.slice(0, r.cursor) + "|" + out.slice(r.cursor);
}

const doc = "* Note\nSee [fn:1] and [fn:x].\n#+begin_src\n[fn:1] not this\n#+end_src\n* Footnotes\n[fn:1] The definition.\n[fn:x]\n";

test("a reference goes to its definition's text, a definition to its first reference", () => {
  const ref = doc.indexOf("[fn:1]");
  assert.deepEqual(footnoteTarget(doc, ref + 2), { label: "1", def: false, to: doc.indexOf("The definition") });
  assert.deepEqual(footnoteTarget(doc, ref + 6), { label: "1", def: false, to: doc.indexOf("The definition") });
  assert.deepEqual(footnoteTarget(doc, doc.indexOf("[fn:1] The")), { label: "1", def: true, to: ref });
  assert.deepEqual(footnoteTarget(doc, doc.indexOf("[fn:x].") + 1), { label: "x", def: false, to: doc.lastIndexOf("[fn:x]") + 6 });
});

test("not on a followable footnote", () => {
  assert.equal(footnoteTarget(doc, 2), null);
  assert.equal(footnoteTarget(doc, doc.indexOf("See")), null);
  assert.equal(footnoteTarget(doc, doc.indexOf("[fn:1] not") + 1), null); // inside a src block
  assert.equal(footnoteTarget("a [fn:n:inline] b", 4), null);
  assert.deepEqual(footnoteTarget("a [fn:9] b", 4), { label: "9", def: false, to: null });
});

test("nextLabel takes the highest number, ignoring names and blocks", () => {
  assert.equal(nextLabel(""), 1);
  assert.equal(nextLabel("x [fn:2] [fn:10] [fn:z]\n#+begin_example\n[fn:99]\n#+end_example"), 11);
});

test("addFootnote creates the Footnotes heading at the end", () => {
  assert.equal(add("Hello|"), "Hello[fn:1]\n\n* Footnotes\n[fn:1] |");
  assert.equal(add("* A\nHi| there\n"), "* A\nHi[fn:1] there\n\n* Footnotes\n[fn:1] |");
  assert.equal(add("|"), "[fn:1]\n\n* Footnotes\n[fn:1] |");
});

test("addFootnote appends to an existing Footnotes section, even when it isn't last", () => {
  assert.equal(add("a [fn:1] b|\n* Footnotes\n[fn:1] one"), "a [fn:1] b[fn:2]\n* Footnotes\n[fn:1] one\n[fn:2] |");
  assert.equal(add("x|\n* Footnotes\n[fn:1] one\n\n* Later\nz\n"), "x[fn:2]\n* Footnotes\n[fn:1] one\n[fn:2] |\n\n* Later\nz\n");
  assert.equal(add("* Footnotes\n[fn:1] one\n* Later\nz|"), "* Footnotes\n[fn:1] one\n[fn:2] |\n* Later\nz[fn:2]");
});
