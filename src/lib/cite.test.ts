import test from "node:test";
import assert from "node:assert/strict";
import { bibEditMatters, bibKeywords, bibLine, bibliography, chip, citeAt, citeQuery, cites, citeText, printBibDirective, type BibEntry } from "./cite.ts";

test("cites: keys with ranges, prefixes and suffixes ignored", () => {
  const l = "As [cite:@a;@b] and [cite/t:see @smith20 p. 3; @lee.] ok [[x]] [cite:no keys]";
  const c = cites(l);
  assert.deepEqual(c.map((c) => c.keys.map((k) => k.key)), [["a", "b"], ["smith20", "lee"]]);
  assert.equal(l.slice(c[0].from, c[0].to), "[cite:@a;@b]");
  for (const k of c.flatMap((c) => c.keys)) assert.equal(l.slice(k.from, k.to), `@${k.key}`);
  assert.deepEqual(cites("[[cite:x]] [cite:@a"), []);
});

test("bibKeywords: the note's #+BIBLIOGRAPHY: lines only", () => {
  const lines = ["#+title: x", "#+BIBLIOGRAPHY: a.bib", "  #+bibliography: ~/b.bib", "#+print_bibliography:", "text"];
  assert.equal(bibKeywords(lines), "#+BIBLIOGRAPHY: a.bib\n  #+bibliography: ~/b.bib");
  assert.equal(bibKeywords([]), "");
});

test("citeAt: the key under the column, else the cite's first key", () => {
  const l = "x [cite:see @a p. 3;@b] y";
  assert.equal(citeAt(l, l.indexOf("@b") + 1), "b");
  assert.equal(citeAt(l, l.indexOf("@a")), "a");
  assert.equal(citeAt(l, l.indexOf("see")), "a");
  assert.equal(citeAt(l, 0), null);
  assert.equal(citeAt("[cite:none]", 3), null);
});

test("citeQuery: the @ prefix inside an open cite", () => {
  assert.equal(citeQuery("see [cite:@sm"), "sm");
  assert.equal(citeQuery("[cite/t:@a; @"), "");
  assert.equal(citeQuery("[cite:@a] @bo"), null);
  assert.equal(citeQuery("mail @bo"), null);
});

test("chip and citeText", () => {
  const e: BibEntry = { key: "a", author: "Smith, Jane", year: "2020", title: "T", container: "Nature", label: "Smith 2020" };
  const bib = new Map([["a", e]]);
  assert.deepEqual(chip(["a", "zz"], bib), { text: "Smith 2020; @zz", missing: true });
  assert.deepEqual(chip(["a"], bib), { text: "Smith 2020", missing: false });
  assert.equal(citeText(e), "Smith, Jane (2020). T. Nature.");
  assert.equal(citeText({ ...e, author: "", year: "", container: "" }), "a. T.");
});

test("bibliography: first-citation order, dedup, unknown keys, code blocks skipped", () => {
  const e = (key: string): BibEntry => ({ key, author: "", year: "", title: `T${key}`, container: "", label: `L${key}` });
  const bib = new Map([["a", e("a")], ["b", e("b")]]);
  const text = "x [cite:@b;@a]\n#+begin_src org\n[cite:@c]\n#+END_SRC\n[cite/t:@zz] [cite:@a]\n#+begin_example\n[cite:@d]\n#+end_example";
  const r = bibliography(text, bib);
  assert.deepEqual(r.map((x) => x.key), ["b", "a", "zz"]);
  assert.equal(r[0].entry, bib.get("b"));
  assert.equal(r[2].entry, undefined);
  assert.deepEqual(bibliography("no cites here", bib), []);
  assert.equal(bibLine("a", e("a")), "La. Ta.");
  assert.equal(bibLine("a", { ...e("a"), label: "", title: "" }), "a.");
  assert.equal(bibLine("zz"), "[zz?]");
});

test("printBibDirective", () => {
  assert.ok(printBibDirective("#+PRINT_BIBLIOGRAPHY:"));
  assert.ok(printBibDirective("  #+print_bibliography: :style numeric"));
  assert.ok(!printBibDirective("text #+PRINT_BIBLIOGRAPHY:"));
  assert.ok(!printBibDirective("#+PRINT_BIBLIOGRAPHY"));
});

test("bibEditMatters: only cite or #+ lines", () => {
  assert.ok(!bibEditMatters("plain text", "plain texts"));
  assert.ok(bibEditMatters("see [cite:@a]", "see [cite:@"));
  assert.ok(bibEditMatters("x", "#+PRINT_BIBLIOGRAPHY:"));
  assert.ok(bibEditMatters("#+begin_src", "x"));
});
