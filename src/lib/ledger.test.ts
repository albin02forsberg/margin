import { test } from "node:test";
import assert from "node:assert/strict";
import { budgetBar, costLine, filterRows, money, savingsRate, shiftMonth, sumByCurrency } from "./ledger.ts";

test("money formats signed cents without float error", () => {
  assert.equal(money(-1250, "EUR"), "−12.50 EUR");
  assert.equal(money(120005, "SEK"), "+1200.05 SEK");
  assert.equal(money(7, "EUR", false), "0.07 EUR");
  assert.equal(money(0, "EUR"), "0.00 EUR");
});

test("shiftMonth crosses year boundaries", () => {
  assert.equal(shiftMonth("2026-01", -1), "2025-12");
  assert.equal(shiftMonth("2026-12", 1), "2027-01");
  assert.equal(shiftMonth("2026-10", -22), "2024-12");
  assert.equal(shiftMonth("2026-10", 0), "2026-10");
});

test("costLine shows a project's cost and income, leaving out zeros", () => {
  assert.equal(costLine({ currency: "EUR", income: 90000, expense: -34000 }), "Ledger · cost 340.00 EUR · income 900.00 EUR");
  assert.equal(costLine({ currency: "SEK", income: 0, expense: -500 }), "Ledger · cost 5.00 SEK");
  assert.equal(costLine({ currency: "USD", income: 1, expense: 0 }), "Ledger · income 0.01 USD");
});

test("budgetBar clamps the fill and flags overspending", () => {
  assert.deepEqual(budgetBar(1350, 2000), { pct: 68, over: false });
  assert.deepEqual(budgetBar(2500, 2000), { pct: 100, over: true });
  assert.deepEqual(budgetBar(-300, 2000), { pct: 0, over: false });
});

test("filterRows matches #category, @payee and free text; sumByCurrency totals per currency", () => {
  const r = (text: string, category: string | null, payee: string | null, cents = -100, currency = "EUR") => ({ text, category, payee, cents, currency });
  const rows = [r("Lunch", "Food", "Cafe"), r("Bus", null, "SL", -250, "SEK"), r("Salary", "Pay", null, 5000)];
  assert.equal(filterRows(rows, "").length, 3);
  assert.deepEqual(filterRows(rows, "#food").map((x) => x.text), ["Lunch"]);
  assert.deepEqual(filterRows(rows, "#-").map((x) => x.text), ["Bus"]);
  assert.deepEqual(filterRows(rows, "@sl").map((x) => x.text), ["Bus"]);
  assert.equal(filterRows(rows, "#fo").length, 0);
  assert.deepEqual(filterRows(rows, "SAL").map((x) => x.text), ["Salary"]);
  assert.deepEqual(filterRows(rows, "cafe").map((x) => x.text), ["Lunch"]);
  assert.deepEqual(sumByCurrency(rows), [["EUR", 4900], ["SEK", -250]]);
});

test("savingsRate is net over income, null without income", () => {
  assert.equal(savingsRate(100000, 25000), 25);
  assert.equal(savingsRate(100000, -50000), -50);
  assert.equal(savingsRate(0, -5000), null);
});
