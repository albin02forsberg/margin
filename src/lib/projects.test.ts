import { test } from "node:test";
import assert from "node:assert/strict";
import { cards, merge, toProject, toRow } from "./projects.ts";

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

test("cards: tracked and active projects, their share of the week and open tasks by category", () => {
  const p = (active: boolean) => ({ export_code: "", rounding: null, round_up: false, active });
  const t = (title: string, category: string) => ({ path: `/n/${category}.org`, line: 1, title, category, keyword: "TODO" });
  const tracked = [
    { project: "Acme", code: "A-1", worked: 3, billable: 3, month: 10 },
    { project: "Beta", code: "B", worked: 1, billable: 1, month: 1 },
    { project: "Old", code: "Old", worked: 0, billable: 0, month: 4 },
  ];
  const c = cards({ Acme: p(true), Zed: p(true), Hidden: p(false) }, tracked, [t("a", "acme"), t("b", "Acme"), t("x", "misc")]);
  assert.deepEqual(c.map((x) => [x.project, x.share, x.tasks.map((y) => y.title)]), [["Acme", 0.75, ["a", "b"]], ["Beta", 0.25, []], ["Old", 0, []], ["Zed", 0, []]]);
  assert.equal(c[3].code, "Zed"); // no export code: the name, as the backend does
  assert.deepEqual(cards({}, [], []), []);
});
