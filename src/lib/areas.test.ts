import { test } from "node:test";
import { strict as assert } from "node:assert";
import { visibleCmds } from "./areas.ts";

test("visibleCmds drops the commands of hidden areas only", () => {
  const cmds = ["Ledger: add an entry…", "Go to Ledger (month totals)", "Habits: overview", "Flashcards: review", "Go to Today", "Tasks: goals"].map((label) => ({ label }));
  const labels = (h: string[]) => visibleCmds(cmds, h).map((c) => c.label);
  assert.equal(visibleCmds(cmds, []), cmds);
  assert.deepEqual(labels(["Ledger"]), ["Habits: overview", "Flashcards: review", "Go to Today", "Tasks: goals"]);
  assert.deepEqual(labels(["habits", "flashcards", "nope"]), ["Ledger: add an entry…", "Go to Ledger (month totals)", "Go to Today", "Tasks: goals"]);
});
