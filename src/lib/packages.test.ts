import { test } from "node:test";
import assert from "node:assert/strict";
import { session } from "./commands.ts";
import { loadPackages, PACKAGES } from "./packages/index.ts";

const host = () => {
  const h = { doc: "", config: () => ({}), setConfig: () => {}, buffer: { path: "n.org", insert: (t: string) => { h.doc += t; } }, reload: () => {} };
  return h;
};

test("bundled packages have unique ids", () => {
  assert.equal(new Set(PACKAGES.map((p) => p.id)).size, PACKAGES.length);
});

test("an enabled package registers its command; a disabled one doesn't", () => {
  const h = host(), s = session(h, () => {});
  loadPackages(s.api, [], () => {});
  const c = s.cmds.find((c) => c.label === "Insert timestamp now (inactive)");
  assert.equal(c?.leader, "i D");
  c!.run();
  assert.match(h.doc, /^\[\d{4}-\d\d-\d\d \w{3} \d\d:\d\d\]$/);
  const off = session(host(), () => {});
  loadPackages(off.api, ["insert-timestamp"], () => {});
  assert.equal(off.cmds.length, 0);
});

test("a user script's command with the same label replaces the package's; a throwing package is reported", () => {
  const errs: string[] = [], s = session(host(), (m) => errs.push(m));
  loadPackages(s.api, [], (m) => errs.push(m), [...PACKAGES, { id: "bad", load: () => { throw new Error("nope"); } }]);
  s.run([{ name: "a.js", text: `margin.command({ label: "Insert timestamp now (inactive)", run: () => "mine" });` }], []);
  assert.deepEqual(s.cmds.map((c) => c.run()), ["mine"]);
  assert.deepEqual(errs, ["⚠ package bad: nope"]);
});
