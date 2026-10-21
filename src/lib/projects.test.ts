import { test } from "node:test";
import assert from "node:assert/strict";
import { budgetUsage, cards, merge, toProject, toRow } from "./projects.ts";

test("project rows round-trip, and round-up is off without rounding", () => {
  const none = { budget_hours: null, budget_period: "month" };
  const p = { export_code: "ACME", rounding: 0.25, round_up: true, active: false, ...none };
  assert.deepEqual(toProject(toRow("Acme", p)), p);
  assert.deepEqual(toProject(toRow("X", { export_code: " X ", rounding: null, round_up: false, active: true })), { export_code: "X", rounding: null, round_up: false, active: true, ...none });
  assert.equal(toProject({ ...toRow("Acme", p), rounding: "" }).round_up, false);
});

test("budgets round-trip; blank, zero or junk is none; the period is week or month", () => {
  const p = { export_code: "A", rounding: null, round_up: false, active: true, budget_hours: 20, budget_period: "week" };
  assert.deepEqual(toProject(toRow("A", p)), p);
  const legacy = toRow("A", { export_code: "A", rounding: null, round_up: false, active: true });
  assert.deepEqual([legacy.budget, legacy.period], ["", "month"]);
  assert.deepEqual(toProject(legacy).budget_hours, null);
  for (const budget of ["", " ", "0", "-3", "abc", null]) assert.equal(toProject({ ...legacy, budget }).budget_hours, null, String(budget));
  assert.equal(toProject({ ...legacy, budget: 7.5 }).budget_hours, 7.5); // a number input binds a number
  assert.equal(toProject({ ...legacy, period: "year" }).budget_period, "month");
});

test("budgetUsage: weekly uses this week, else this month; over past 100%", () => {
  const t = { worked: 6, month: 25 };
  assert.deepEqual(budgetUsage({ budget_hours: 20 }, t), { used: 25, total: 20, pct: 125, over: true, period: "month" });
  assert.deepEqual(budgetUsage({ budget_hours: 8, budget_period: "week" }, t), { used: 6, total: 8, pct: 75, over: false, period: "week" });
  assert.equal(budgetUsage({ budget_hours: 20, budget_period: "bogus" }, t)?.period, "month");
  assert.equal(budgetUsage({ budget_hours: 20 }, { worked: 0, month: 20 })?.over, false); // exactly on budget
  for (const p of [undefined, {}, { budget_hours: null }, { budget_hours: 0 }]) assert.equal(budgetUsage(p, t), null);
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
