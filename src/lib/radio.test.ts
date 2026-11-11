// Run: npm test
import { test } from "node:test";
import assert from "node:assert/strict";
import { mentions, radio } from "./radio.ts";

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
