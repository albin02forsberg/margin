import test from "node:test";
import assert from "node:assert/strict";
import { chip, citeAt, citeQuery, cites, citeText, type BibEntry } from "./cite.ts";

test("cites: keys with ranges, prefixes and suffixes ignored", () => {
  const l = "As [cite:@a;@b] and [cite/t:see @smith20 p. 3; @lee.] ok [[x]] [cite:no keys]";
  const c = cites(l);
  assert.deepEqual(c.map((c) => c.keys.map((k) => k.key)), [["a", "b"], ["smith20", "lee"]]);
  assert.equal(l.slice(c[0].from, c[0].to), "[cite:@a;@b]");
  for (const k of c.flatMap((c) => c.keys)) assert.equal(l.slice(k.from, k.to), `@${k.key}`);
  assert.deepEqual(cites("[[cite:x]] [cite:@a"), []);
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
