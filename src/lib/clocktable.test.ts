import { test } from "node:test";
import assert from "node:assert/strict";
import { clockBlocks } from "./clocktable.ts";

const fill = (text: string, body: (args: string) => string) =>
  clockBlocks(text).reduceRight((t, b) => t.slice(0, b.from) + body(b.args) + t.slice(b.to), text);

test("clockBlocks finds each block's options and body span", () => {
  const text = "* Report\n#+BEGIN: clocktable :block thismonth\n| old |\n#+END:\ntext\n  #+begin: clocktable :project Acme :block today\n#+end:\n";
  const bs = clockBlocks(text);
  assert.deepEqual(bs.map((b) => b.args), [":block thismonth", ":project Acme :block today"]);
  assert.equal(text.slice(bs[0].from, bs[0].to), "| old |\n");
  assert.equal(text.slice(bs[1].from, bs[1].to), "");
  const out = fill(text, (a) => `| ${a} |\n`);
  assert.equal(out, "* Report\n#+BEGIN: clocktable :block thismonth\n| :block thismonth |\n#+END:\ntext\n  #+begin: clocktable :project Acme :block today\n| :project Acme :block today |\n#+end:\n");
  assert.equal(fill(out, (a) => `| ${a} |\n`), out); // idempotent
});

test("clockBlocks names tasks blocks next to clocktables", () => {
  const text = '#+BEGIN: tasks :query "tag:work due:<7d" :limit 5\n- old\n#+END:\n#+begin: CLOCKTABLE :block today\n#+end:\n';
  const bs = clockBlocks(text);
  assert.deepEqual(bs.map((b) => [b.name, b.args]), [["tasks", ':query "tag:work due:<7d" :limit 5'], ["clocktable", ":block today"]]);
  assert.equal(text.slice(bs[0].from, bs[0].to), "- old\n");
});

test("clockBlocks names columnview blocks", () => {
  const text = '#+BEGIN: columnview :query "tag:book" :cols "ITEM RATING"\n| old |\n#+END:\n';
  const [b] = clockBlocks(text);
  assert.deepEqual([b.name, b.args, text.slice(b.from, b.to)], ["columnview", ':query "tag:book" :cols "ITEM RATING"', "| old |\n"]);
});

test("clockBlocks skips unterminated blocks and other dynamic blocks", () => {
  assert.deepEqual(clockBlocks("#+BEGIN: clocktable :block today\nno end\n"), []);
  assert.deepEqual(clockBlocks("#+BEGIN: clocktable :block today"), []);
  assert.deepEqual(clockBlocks("#+BEGIN: columnviews\n#+END:\n#+BEGIN: clocktablex\n#+END:\n#+BEGIN: tasksx\n#+END:\n"), []);
  const text = "#+BEGIN: clocktable :block today\nlost\n#+BEGIN: clocktable :block thisweek\n#+END:\n";
  assert.deepEqual(clockBlocks(text).map((b) => b.args), [":block thisweek"]);
});
