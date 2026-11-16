import { test } from "node:test";
import assert from "node:assert/strict";
import { applyBinds, mergeCmds, registry, runScripts, session } from "./commands.ts";

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

const host = () => {
  const cfg: Record<string, unknown> = { font_size: 14 }, log: string[] = [], seen: unknown[] = [];
  return { config: () => cfg, setConfig: (k: string, v: unknown) => { cfg[k] = v; log.push(`set ${k}`); }, buffer: { seen }, reload: () => {}, cfg, log, seen };
};

test("hooks run in order; a throwing or rejecting hook is reported and the rest still run", async () => {
  const h = host(), errs: string[] = [];
  const s = session(h, (m) => errs.push(m));
  s.run([{ name: "a.js", text: `
    margin.on("save", (p) => { throw new Error("bad " + p); });
    margin.on("save", async () => { throw new Error("later"); });
    margin.on("save", (p) => margin.buffer.seen.push(p));
    margin.on("nope", () => {});` }], []);
  s.emit("save", "x.org");
  s.emit("open", "y.org"); // no open hooks: nothing happens
  await new Promise((r) => setTimeout(r));
  assert.deepEqual(h.seen, ["x.org"]);
  assert.deepEqual(errs, ["⚠ init/a.js: on needs ('save' | 'open' | 'startup', fn)", "⚠ init save hook: bad x.org", "⚠ init save hook: later"]);
});

test("config.set only changes the in-memory config through the host; get reads it", () => {
  const h = host(), s = session(h, () => {});
  s.run([{ name: "a.js", text: `margin.config.set("font_size", margin.config.get("font_size") + 2);` }], []);
  assert.equal(h.cfg.font_size, 16);
  assert.deepEqual(s.over, { font_size: 16 });
  assert.deepEqual(h.log, ["set font_size"]); // the host setter is the only way out: no save_config
});

test("a new session (reload) keeps nothing from the previous run", () => {
  const h = host(), text = `margin.command({ label: "U", run: () => 1 }); margin.bind("Ctrl+U", "U"); margin.on("startup", () => margin.buffer.seen.push(1)); margin.config.set("a", 1);`;
  const a = session(h, () => {});
  a.run([{ name: "a.js", text }], []);
  assert.deepEqual([a.cmds.length, a.binds.size, Object.keys(a.over).length], [1, 1, 1]);
  const b = session(h, () => {});
  assert.deepEqual([b.cmds.length, b.binds.size, Object.keys(b.over).length], [0, 0, 0]);
  b.emit("startup");
  assert.deepEqual(h.seen, []);
});

test("bind moves keys to the target command; unknown targets are reported", () => {
  const errs: string[] = [], s = session(host(), (m) => errs.push(m));
  s.run([{ name: "a.js", text: `margin.bind("Ctrl+K", "B"); margin.bind(["alt+x"], "Gone"); margin.bind([], "B");` }], ["A", "B"]);
  assert.deepEqual(errs, ["⚠ init/a.js: bind needs (keys, label)", "⚠ init: bind: no command “Gone”"]);
  const m = applyBinds([{ label: "A", keys: ["ctrl+k", "Ctrl+P"] }, { label: "B" }, { label: "C", keys: ["Ctrl+J"] }], s.binds);
  assert.deepEqual(m.map((c) => c.keys), [["Ctrl+P"], ["Ctrl+K"], ["Ctrl+J"]]);
});

test("mergeCmds puts user commands last and lets them override", () => {
  const m = mergeCmds([{ label: "A" }, { label: "B" }], [{ label: "A" }, { label: "C" }]);
  assert.deepEqual(m.map((c) => c.label), ["B", "A", "C"]);
});

/** A host whose buffer is one note: insert appends; emit("save") stands in for the autosave after each edit. */
const noteHost = () => {
  const h = { ...host(), doc: "", reloads: 0 };
  return Object.assign(h, { buffer: { path: "n.org", insert: (t: string) => { h.doc += t; } }, reload: () => { h.reloads++; } });
};
const tick = () => new Promise((r) => setTimeout(r));

for (const kind of ["sync", "async"]) test(`a ${kind} save hook that always edits causes one extra save, not a loop`, async () => {
  const h = noteHost(), s = session(h, () => {});
  let calls = 0;
  s.api.on("save", kind === "sync" ? () => { calls++; s.api.buffer.insert("."); } : async () => { await null; calls++; s.api.buffer.insert("."); });
  for (let i = 0; i < 3; i++) { s.emit("save", "n.org"); await tick(); } // user save, the save the hook caused, a later save
  assert.equal(calls, 2); // the save the hook caused skipped it
  assert.equal(h.doc, "..");
});

test("a throwing save hook clears its flag; edits outside hooks don't mark the buffer", () => {
  const h = noteHost(), errs: string[] = [], s = session(h, (m) => errs.push(m));
  let calls = 0;
  s.api.on("save", () => { calls++; throw new Error("x"); });
  s.api.buffer.insert("typed");
  s.emit("save", "n.org"); s.emit("save", "n.org");
  assert.equal(calls, 2);
  assert.equal(errs.length, 2);
});

test("reload() inside a startup hook is ignored; outside it works", async () => {
  const h = noteHost(), errs: string[] = [], s = session(h, (m) => errs.push(m));
  s.api.on("startup", () => s.api.reload());
  s.api.on("startup", async () => { await null; s.api.reload(); });
  s.emit("startup");
  await tick();
  assert.equal(h.reloads, 0);
  assert.deepEqual(errs, Array(2).fill("⚠ init: reload() ignored inside a startup hook"));
  s.api.reload();
  assert.equal(h.reloads, 1);
});
