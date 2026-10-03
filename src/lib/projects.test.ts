import { test } from "node:test";
import assert from "node:assert/strict";
import { merge, toProject, toRow } from "./projects.ts";

test("project rows round-trip, and round-up is off without rounding", () => {
  const p = { export_code: "ACME", rounding: 0.25, round_up: true, active: false };
  assert.deepEqual(toProject(toRow("Acme", p)), p);
  assert.deepEqual(toProject(toRow("X", { export_code: " X ", rounding: null, round_up: false, active: true })), { export_code: "X", rounding: null, round_up: false, active: true });
  assert.equal(toProject({ ...toRow("Acme", p), rounding: "" }).round_up, false);
});

test("merge keeps unsaved rows and refreshes the rest", () => {
  const a = { export_code: "A", rounding: 0.5, round_up: false, active: true };
  const edited = { ...toRow("A", a), code: "typing", dirty: true };
  const rows = merge([edited, toRow("B", a)], { A: { ...a, export_code: "disk" }, B: { ...a, export_code: "B2" }, C: a });
  assert.deepEqual(rows.map((r) => [r.name, r.code]), [["A", "typing"], ["B", "B2"], ["C", "A"]]);
});
