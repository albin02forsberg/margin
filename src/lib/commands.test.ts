import { test } from "node:test";
import assert from "node:assert/strict";
import { mergeCmds, registry, runScripts } from "./commands.ts";

test("scripts register commands; duplicates replace; errors are reported", () => {
  const r = registry(), errs: string[] = [];
  runScripts([
    { name: "a.js", text: "margin.register({ label: 'Hi', run: () => 1 });" },
    { name: "b.js", text: "throw new Error('boom')" },
    { name: "c.js", text: "margin.register({ label: 'Hi', run: () => 2 }); margin.register({ nope: 1 });" },
  ], r, (m) => errs.push(m));
  assert.equal(r.list.length, 1);
  assert.equal(r.list[0].run(), 2);
  assert.deepEqual(errs, ["⚠ init/b.js: boom", "⚠ init/c.js: register needs { label, run }"]);
});

test("mergeCmds puts user commands last and lets them override", () => {
  const m = mergeCmds([{ label: "A" }, { label: "B" }], [{ label: "A" }, { label: "C" }]);
  assert.deepEqual(m.map((c) => c.label), ["B", "A", "C"]);
});
