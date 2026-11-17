import { test } from "node:test";
import assert from "node:assert/strict";
import { tocDepth, tocDirective, tocEditMatters, tocEntries } from "./toc.ts";

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

test("tocEditMatters", () => {
  assert.equal(tocEditMatters("plain", "plainx", false), false);
  assert.equal(tocEditMatters("* Head", "* Heads", false), false); // no TOC in the note: headings don't matter
  assert.equal(tocEditMatters("#+TO", "#+TOC: headlines", false), true);
  assert.equal(tocEditMatters("plain", "plainx", true), false);
  assert.equal(tocEditMatters("plain", "pl\nain", true), true); // entries hold line numbers
  assert.equal(tocEditMatters("* Head", "* Heads", true), true);
  assert.equal(tocEditMatters("x", "*x", true), true);
  assert.equal(tocEditMatters("#+end_src", "", true), true);
});

// The fast path must equal a full rebuild: whenever tocEditMatters says no, the widgets (directive line, entries) are unchanged.
test("tocEditMatters agrees with a full rebuild over random edits", () => {
  let seed = 7;
  const rnd = (n: number) => ((seed = (seed * 1103515245 + 12345) % 2 ** 31), seed % n);
  const pick = <V,>(a: V[]) => a[rnd(a.length)];
  const LINES = ["* A", "** B :noexport:", "*** TODO C", "text", "", "#+TOC: headlines 2", "#+toc: headlines local", "#+begin_src", "* in src", "#+end_src"];
  const BITS = ["x", " ", "\n", "*", "#", "+", "\n* N\n", "#+TOC: headlines", "#+begin_example", "#+end_example", "word"];
  const widgets = (doc: string) => {
    let block = false;
    return doc.split("\n").flatMap((t, i) => {
      if (/^\s*#\+begin_/i.test(t)) block = true;
      else if (/^\s*#\+end_/i.test(t)) block = false;
      const d = block ? null : tocDirective(t);
      return d ? [[i, tocEntries(doc, T, D, d.depth, d.local ? i : -1)]] : [];
    });
  };
  const start = (s: string, p: number) => (p ? s.lastIndexOf("\n", p - 1) + 1 : 0), end = (s: string, p: number) => (s.indexOf("\n", p) + 1 || s.length + 1) - 1;
  let doc = "", fast = 0;
  for (let i = 0; i < 3000; i++) {
    if (i % 50 === 0) doc = Array.from({ length: 12 }, () => pick(LINES)).join("\n");
    const from = rnd(doc.length + 1), to = Math.min(doc.length, from + rnd(4)), ins = rnd(3) ? pick(BITS) : "";
    const now = doc.slice(0, from) + ins + doc.slice(to);
    const has = doc.split("\n").some((l) => tocDirective(l));
    if (!tocEditMatters(doc.slice(start(doc, from), end(doc, to)), now.slice(start(now, from), end(now, from + ins.length)), has)) {
      fast++;
      assert.deepEqual(widgets(now), has ? widgets(doc) : [], JSON.stringify({ doc, now }));
    }
    doc = now;
  }
  assert.ok(fast > 100, `fast path taken ${fast} times`);
});
