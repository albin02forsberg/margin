import { test } from "node:test";
import assert from "node:assert/strict";
import { lineContext, syntaxAt, type SyntaxOption } from "./syntax.ts";
import { clockBlocks } from "./clocktable.ts";
import { tocDirective } from "./toc.ts";
import { printBibDirective } from "./cite.ts";
import { headerArgs, srcBlocks } from "./babel.ts";

const labels = (before: string, ctx: "drawer" | "code" | null = null) => syntaxAt(before, ctx)?.options.map((o) => o.label);
const pick = (before: string, label: string, ctx: "drawer" | "code" | null = null) => {
  const r = syntaxAt(before, ctx)!, o = r.options.find((x) => x.label === label) as SyntaxOption;
  assert.ok(o, `${label} offered after ${before}`);
  const text = before.slice(0, before.length - r.query.length) + o.insert, at = before.length - r.query.length + (o.cursor ?? o.insert.length);
  return text.slice(0, at) + "|" + text.slice(at);
};

test("#+ at line start offers keywords and block skeletons with the cursor inside", () => {
  assert.equal(syntaxAt("#+ti", null)?.query, "ti");
  assert.ok(labels("#+")!.includes("title") && labels("#+")!.includes("begin: clocktable"));
  assert.equal(pick("#+ti", "title"), "#+title: |");
  assert.equal(pick("  #+beg", "begin_quote"), "  #+begin_quote\n  |\n  #+end_quote");
  assert.equal(pick("#+b", "begin_src"), "#+begin_src |\n#+end_src");
  assert.equal(pick("#+b", "begin: tasks"), '#+begin: tasks :query "|"\n#+end:');
  // the query's case: `#+T` gives TITLE:, `#+BEG` gives #+BEGIN_QUOTE … #+END_QUOTE
  assert.equal(pick("#+T", "TITLE"), "#+TITLE: |");
  assert.equal(pick("#+BEG", "BEGIN: clocktable"), "#+BEGIN: clocktable :block thisweek|\n#+END:");
});

test("no completion mid-line, after a full keyword, or in a code body", () => {
  for (const b of ["text #+ti", "#+title:", "#+title: x", "x"]) assert.equal(syntaxAt(b, null), null, b);
  assert.equal(syntaxAt("#+ti", "code"), null);
});

test("every offered skeleton is one the app parses", () => {
  const all = syntaxAt("#+", null)!.options.map((o) => "#+" + o.insert);
  const blocks = all.filter((t) => t.startsWith("#+begin: ")).map((t) => t.replace('""', '"x"') + "\n");
  assert.deepEqual(blocks.flatMap((t) => clockBlocks(t).map((b) => b.name)), ["clocktable", "tasks", "columnview"]);
  assert.deepEqual(labels("#+begin: "), ["clocktable", "tasks", "columnview"]);
  assert.equal(pick("#+BEGIN: co", "columnview"), '#+BEGIN: columnview :query "|" :cols "ITEM"\n#+END:');
  assert.ok(all.some((t) => tocDirective(t)) && all.some(printBibDirective));
  assert.equal(srcBlocks(all.find((t) => t.startsWith("#+begin_src"))!.replace("src ", "src sh")).length, 1);
});

test("dynamic block options and values", () => {
  assert.deepEqual(labels("#+begin: clocktable :"), ["block", "project", "by"]);
  assert.deepEqual(labels("#+begin: clocktable :block today :"), ["project", "by"]);
  assert.deepEqual(labels("#+begin: tasks :"), ["query", "limit", "sort"]);
  const ranges = ["today", "yesterday", "thisweek", "lastweek", "thismonth", "lastmonth", "thisyear", "lastyear"];
  assert.deepEqual(labels("#+begin: clocktable :block th"), ranges);
  const detail = syntaxAt("#+begin: clocktable :", null)!.options.find((o) => o.label === "block")!.detail ?? "";
  for (const k of [...ranges, "YYYY-MM-DD"]) assert.ok(detail.includes(k), k);
  assert.deepEqual(labels("#+begin: clocktable :by "), ["project", "tag"]);
  assert.deepEqual(labels("#+begin: tasks :sort "), ["priority", "date", "title"]);
  assert.equal(syntaxAt("#+begin: clocktable :project ", null), null);
  assert.equal(syntaxAt("#+begin: other :", null), null);
});

test("src languages and header arguments babel.ts reads", () => {
  assert.deepEqual(labels("#+begin_src "), ["sh", "shell", "mermaid"]);
  assert.equal(pick("#+begin_src sh :ta", "tangle"), "#+begin_src sh :tangle |");
  for (const k of labels("#+begin_src sh :")!) assert.ok(k in headerArgs(`#+begin_src sh :${k} x`), k);
});

test("property keys only inside a :PROPERTIES: drawer", () => {
  assert.equal(pick(":EF", "EFFORT", "drawer"), ":EFFORT: |");
  assert.equal(syntaxAt(":EF", null), null);
  const ctx = (above: string) => lineContext(above.split("\n").reverse());
  assert.equal(ctx("* H\n:PROPERTIES:\n:ID: x"), "drawer");
  assert.equal(ctx("* H\n:PROPERTIES:\n:END:"), null);
  assert.equal(ctx(":PROPERTIES:\n:END:\n* H"), null);
  assert.equal(ctx("#+begin_src org\n* not a heading"), null); // a `*` line stops the scan
  assert.equal(ctx("#+begin_src org\ntext"), "code");
  assert.equal(ctx("#+begin_src sh\n#+end_src\nx"), null);
});
