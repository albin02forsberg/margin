// Run: npm test
import { test } from "node:test";
import assert from "node:assert/strict";
import { mentions, radio, radioEditMatters } from "./radio.ts";

const doc = "* Glossary\n<<<Margin>>> is the app. <<<Margin App>>> too. <<<C++>>>\n#+begin_src sh\n<<<Hidden>>>\n#+end_src\n<<< >>> <<<>>> <<<margin>>>\n";
const r = radio(doc);
const words = (line: string) => mentions(line, r).map((m) => [line.slice(m.from, m.to), m.target]);

test("targets outside code blocks, first spelling wins", () => {
  assert.deepEqual([...r!.terms.values()], ["Margin", "Margin App", "C++"]);
  assert.deepEqual([...r!.code], [2, 3, 4]);
  assert.equal(radio("no targets here"), null);
  assert.equal(radio("#+begin_src\n<<<x>>>\n#+end_src"), null);
});

test("whole-word, case-insensitive mentions; longest term wins", () => {
  assert.deepEqual(words("I use margin, MARGIN app and Margins."), [["margin", "Margin"], ["MARGIN app", "Margin App"]]);
  assert.deepEqual(words("C++ and xC++ but not C+"), [["C++", "C++"]]);
  assert.deepEqual(words("Marginå åMargin Margin_x Margin2 (Margin)"), [["Margin", "Margin"]]);
  assert.deepEqual(words("Margin\tapp"), [["Margin\tapp", "Margin App"]]);
});

test("not inside links or targets", () => {
  assert.deepEqual(words("<<<Margin>>> [[Margin]] [[x][Margin]] <<Margin>> Margin"), [["Margin", "Margin"]]);
  assert.equal(mentions("Margin", null).length, 0);
});

test("a target cap keeps the regex bounded", () => {
  const many = Array.from({ length: 300 }, (_, i) => `<<<t${i}>>>`).join(" ");
  assert.equal(radio(many)!.terms.size, 200);
});

test("radioEditMatters", () => {
  assert.equal(radioEditMatters("plain", "plainx", false), false);
  assert.equal(radioEditMatters("plain", "pl\nain", false), false);
  assert.equal(radioEditMatters("plain", "pl\nain", true), true); // `code` holds line numbers
  assert.equal(radioEditMatters("a <<T>>> b", "a <<<T>>> b", false), true); // marker across the edit boundary
  assert.equal(radioEditMatters("#+begin_src", "", false), true);
});

// The fast path must equal a full rebuild: whenever radioEditMatters says no, radio() of the edited note is unchanged.
test("radioEditMatters agrees with a full rebuild over random edits", () => {
  let seed = 11;
  const rnd = (n: number) => ((seed = (seed * 1103515245 + 12345) % 2 ** 31), seed % n);
  const pick = <V,>(a: V[]) => a[rnd(a.length)];
  const LINES = ["<<<Term>>> here", "a term", "text", "", "#+begin_src sh", "<<<Code>>>", "#+end_src", "<<<Other Thing>>>"];
  const BITS = ["x", " ", "\n", "<", ">", "#", "+", "<<<New>>>", "\n#+begin_example\n", "#+end_example", "word"];
  const start = (s: string, p: number) => (p ? s.lastIndexOf("\n", p - 1) + 1 : 0), end = (s: string, p: number) => (s.indexOf("\n", p) + 1 || s.length + 1) - 1;
  let doc = "", fast = 0;
  for (let i = 0; i < 3000; i++) {
    if (i % 50 === 0) doc = Array.from({ length: 10 }, () => pick(LINES)).join("\n");
    const from = rnd(doc.length + 1), to = Math.min(doc.length, from + rnd(4)), ins = rnd(3) ? pick(BITS) : "";
    const now = doc.slice(0, from) + ins + doc.slice(to), prev = radio(doc);
    if (!radioEditMatters(doc.slice(start(doc, from), end(doc, to)), now.slice(start(now, from), end(now, from + ins.length)), !!prev)) {
      fast++;
      assert.deepEqual(radio(now), prev, JSON.stringify({ doc, now }));
    }
    doc = now;
  }
  assert.ok(fast > 100, `fast path taken ${fast} times`);
});
