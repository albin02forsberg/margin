import { test } from "node:test";
import assert from "node:assert/strict";
import { costLine, money, shiftMonth } from "./ledger.ts";

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
