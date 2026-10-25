import { test } from "node:test";
import assert from "node:assert/strict";
import { money, shiftMonth } from "./ledger.ts";

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
