import { test } from "node:test";
import assert from "node:assert/strict";
import { srcAt, srcBlocks, resultsChange, diagramKey, headerArgs, tangleFiles, parseVars, named, callAt, babelJob, shellVars } from "./babel.ts";
import { execFileSync } from "node:child_process";

test("headerArgs reads :key value pairs, quoted values and bare keys", () => {
  assert.deepEqual(headerArgs('  #+BEGIN_SRC sh :tangle bin/x.sh :shebang "#!/bin/sh -e"  :mkdirp yes :noweb'), { tangle: "bin/x.sh", shebang: "#!/bin/sh -e", mkdirp: "yes", noweb: "" });
  assert.deepEqual(headerArgs("#+begin_src sh"), {});
  assert.deepEqual(headerArgs("#+begin_src python :var a=1 b=2"), { var: "a=1 b=2" });
});

test("tangleFiles groups blocks by target, in order, and skips the rest", () => {
  const t = [
    "* Setup",
    '#+begin_src sh :tangle ./bin/run.sh :shebang "#!/bin/sh"',
    "echo one",
    "#+end_src",
    "#+begin_src sh",
    "not tangled",
    "#+end_src",
    "  #+begin_src conf :tangle no",
    "  skipped",
    "  #+end_src",
    "  #+begin_src sh :tangle bin\\run.sh :mkdirp yes",
    "    if true; then",
    "      echo two",
    "    fi",
    "",
    "  #+end_src",
    "#+begin_src python :tangle yes",
    "print(1)",
    "#+end_src",
  ].join("\n");
  assert.deepEqual(tangleFiles(t, "setup.org"), [
    { path: "bin/run.sh", content: "#!/bin/sh\necho one\n\nif true; then\n  echo two\nfi\n", exec: true, mkdirp: true, blocks: 2 },
    { path: "setup.py", content: "print(1)\n", exec: false, mkdirp: false, blocks: 1 },
  ]);
  assert.deepEqual(tangleFiles("#+begin_src org :tangle yes\n* x\n#+end_src", "a.org"), []);
  assert.deepEqual(tangleFiles("#+begin_src sh\nls\n#+end_src", "a.org"), []);
  assert.deepEqual(tangleFiles("#+begin_src sh :tangle e.sh\n#+end_src", "a.org"), [{ path: "e.sh", content: "\n", exec: false, mkdirp: false, blocks: 1 }]);
});

test("diagramKey ignores indentation, escapes and blank lines (same fixture as export.rs)", () => {
  const [b] = srcBlocks("  #+begin_src mermaid\n    graph TD  \n\n      A --> B\n    ,* x\n  #+end_src\n");
  assert.equal(diagramKey(b.code), "graph TD\nA --> B\n* x");
});

const apply = (text: string, c: { from: number; to: number; insert: string }) => text.slice(0, c.from) + c.insert + text.slice(c.to);
const run = (text: string, pos: number, out: string) => apply(text, resultsChange(text, srcAt(text, pos)!, out));

const doc = "* H\n#+begin_src sh :results output\nls\n,* x\n,#+end_src\n#+end_src\nafter";

test("srcAt finds the enclosing block, its begin and end lines included", () => {
  const s = srcAt(doc, doc.indexOf("ls"))!;
  assert.deepEqual(s, { lang: "sh", code: "ls\n* x\n#+end_src", indent: "", from: 4, end: doc.indexOf("\nafter") });
  assert.equal(srcAt(doc, doc.indexOf("#+begin"))?.lang, "sh");
  assert.equal(srcAt(doc, doc.indexOf("\nafter"))?.lang, "sh", "end of the #+end_src line");
  assert.equal(srcAt(doc, 0), null);
  assert.equal(srcAt(doc, doc.indexOf("after")), null);
  assert.equal(srcAt("#+begin_src sh\necho", 16), null, "unclosed");
  assert.equal(srcAt("#+BEGIN_SRC\n#+END_SRC", 0)?.code, "", "empty, no language");
  assert.equal(srcAt("#+begin_quote\nx\n#+end_quote", 15), null, "not a src block");
});

test("srcBlocks lists every closed block (a mermaid diagram among them) and stops at an unclosed one", () => {
  const t = "#+BEGIN_SRC mermaid\ngraph TD\n  A --> B\n#+END_SRC\ntext\n  #+begin_src sh\nls\n  #+end_src\n#+begin_src mermaid\nopen";
  assert.deepEqual(srcBlocks(t).map((s) => [s.lang, s.code, t.slice(s.from, s.end)]), [
    ["mermaid", "graph TD\n  A --> B", "#+BEGIN_SRC mermaid\ngraph TD\n  A --> B\n#+END_SRC"],
    ["sh", "ls", "  #+begin_src sh\nls\n  #+end_src"],
  ]);
  assert.deepEqual(srcBlocks("#+begin_quote\nx\n#+end_quote"), []);
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

test("resultsChange replaces block, drawer, table and file-link results; unclosed ones are left alone", () => {
  const head = "#+begin_src sh\n#+end_src\n";
  for (const old of ["#+begin_example\nx\n: y\n#+end_example", "#+BEGIN_SRC sh\nx\n#+END_SRC", ":RESULTS:\nx\n:END:", "| a | b |\n|---+---|\n| 1 | 2 |", "[[file:a.png]]"]) {
    assert.equal(run(`${head}#+RESULTS:\n${old}\nafter`, 0, "n"), `${head}#+RESULTS:\n: n\nafter`, old);
  }
  assert.equal(run(`${head}#+RESULTS:\n| a |\npara | x`, 0, "n"), `${head}#+RESULTS:\n: n\npara | x`, "table stops at first non-table line");
  const open = `${head}#+RESULTS:\n#+begin_example\nx\nafter`;
  assert.equal(run(open, 0, "n"), `${head}#+RESULTS:\n: n\n#+begin_example\nx\nafter`, "unclosed block kept");
  const drawer = `${head}#+RESULTS:\n:RESULTS:\nx`;
  assert.equal(run(drawer, 0, "n"), `${head}#+RESULTS:\n: n\n:RESULTS:\nx`, "unclosed drawer kept");
});

test("headerArgs keeps quoted var values whole and joins repeated :var", () => {
  assert.deepEqual(headerArgs('#+begin_src sh :var a="x y" :results output :var b=totals'), { var: 'a="x y" b=totals', results: "output" });
  assert.deepEqual(parseVars('a="x, y" b=totals,c=2'), [["a", '"x, y"'], ["b", "totals"], ["c", "2"]]);
});

test("shellVars quotes values so ', $() and backquotes stay text", { skip: process.platform === "win32" }, () => {
  const v = "it's $(echo pwned) `id` \"q\"\nline 2";
  assert.equal(shellVars([["x", "a'b"]]), "x='a'\\''b'\n");
  assert.equal(execFileSync("sh", ["-c", shellVars([["x", v]]) + 'printf %s "$x"']).toString(), v);
});

const nb = [
  "#+NAME: totals",
  "#+begin_src sh",
  "echo 42",
  "#+end_src",
  "",
  "#+RESULTS: totals",
  ": 42",
  "",
  "#+NAME: fresh",
  "#+begin_src sh :var n=totals",
  'echo "$n"',
  "#+end_src",
  "",
  "#+NAME: greet",
  "#+HEADER: :results output",
  '#+begin_src sh :var who="world" n=fresh',
  'echo "hi $who"',
  "#+end_src",
  "",
  "#+NAME: tbl",
  "| a | 1 |",
  "",
  "#+RESULTS: loose",
  "#+begin_example",
  "  x",
  "    y",
  "#+end_example",
  "",
  '#+CALL: greet(who="you", n=3)',
  "#+CALL: nope()",
].join("\n");

test("named finds a named block with its results, a named table and #+RESULTS: name", () => {
  assert.equal(named(nb, "totals")?.result, "42");
  assert.equal(named(nb, "totals")?.src?.code, "echo 42");
  assert.deepEqual([named(nb, "fresh")?.src?.code, named(nb, "fresh")?.result], ['echo "$n"', null]);
  assert.deepEqual(named(nb, "tbl"), { src: null, result: "| a | 1 |" });
  assert.deepEqual(named(nb, "loose"), { src: null, result: "x\n  y" });
  assert.equal(named(nb, "missing"), null);
});

test("callAt reads a #+CALL: line and its arguments", () => {
  assert.deepEqual(callAt('x\n  #+call: f(a=1, b="c d")\ny', 5), { name: "f", args: [["a", "1"], ["b", '"c d"']], indent: "  ", end: 27 });
  assert.equal(callAt("#+CALL: f", 0)?.name, "f");
  assert.equal(callAt("text", 0), null);
});

test("babelJob resolves :var from results, runs a missing dependency first and applies #+CALL: args", () => {
  const at = (s: string) => nb.indexOf(s);
  const fresh = babelJob(nb, at('echo "$n"'));
  assert.ok(fresh && "src" in fresh);
  assert.deepEqual([fresh.vars, fresh.name, fresh.at], [[["n", "42"]], "fresh", fresh.src]);
  // greet needs fresh's results: fresh runs first, with its own vars resolved.
  assert.deepEqual(babelJob(nb, at('echo "hi')), { need: fresh });
  const call = babelJob(nb, at("#+CALL: greet"));
  assert.ok(call && "src" in call);
  assert.deepEqual([call.vars, call.name, call.src.code, call.at.end], [[["who", "you"], ["n", "3"]], "", 'echo "hi $who"', at("\n#+CALL: nope")]);
  assert.deepEqual(babelJob(nb, at("#+CALL: nope")), { error: "No code block named “nope” in this note." });
  assert.equal(babelJob(nb, at("| a")), null);
});

test("babelJob reports unknown names, cycles and deep dependencies without running anything", () => {
  const blk = (name: string, vars: string, res = "") => `#+NAME: ${name}\n#+begin_src sh :var ${vars}\necho\n#+end_src\n${res}\n`;
  assert.deepEqual(babelJob(blk("a", "x=ghost"), 20), { error: ":var x=ghost: no #+NAME: ghost in this note; nothing ran." });
  assert.deepEqual(babelJob(blk("a", "x=a", "#+RESULTS: a\n: 1"), 20), { error: ":var x=a is a cycle (a → a); nothing ran." });
  assert.deepEqual(babelJob(blk("a", "x=b") + blk("b", "y=a"), 20), { error: ":var y=a is a cycle (a → b → a); nothing ran." });
  assert.deepEqual(babelJob(blk("a", "x=b") + blk("b", "y=c") + blk("c", "z=1"), 20), { error: "“c” has no results yet: run it first (dependencies run one level deep)." });
  assert.deepEqual(babelJob(blk("a", "1x=2"), 20), { error: ":var 1x: not a shell variable name." });
});

test("resultsChange writes #+RESULTS: name and replaces it on a rerun", () => {
  const t = "#+NAME: n\n#+begin_src sh\necho\n#+end_src\nafter";
  const once = apply(t, resultsChange(t, srcAt(t, 12)!, "1", "n"));
  assert.equal(once, "#+NAME: n\n#+begin_src sh\necho\n#+end_src\n\n#+RESULTS: n\n: 1\nafter");
  assert.equal(apply(once, resultsChange(once, srcAt(once, 12)!, "2", "n")), "#+NAME: n\n#+begin_src sh\necho\n#+end_src\n\n#+RESULTS: n\n: 2\nafter");
  assert.equal(named(once, "n")?.result, "1");
});
