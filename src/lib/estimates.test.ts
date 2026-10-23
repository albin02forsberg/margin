import { test } from "node:test";
import assert from "node:assert/strict";
import { estimateLine, estimates, pct } from "./estimates.ts";

const t = (title: string, category: string, effort: number | null, spent: number | null) => ({ title, category, effort, spent });

test("estimates sums per project and in total, worst overruns first, skipping unestimated and unclocked tasks", () => {
  const r = estimates([
    t("a", "work", 60, 90), t("b", "work", 30, 20), t("c", "home", 60, 60), t("d", "home", 0, 50),
    t("e", "home", null, 10), t("f", "work", 45, null), t("g", "side", 10, 100),
  ]);
  assert.deepEqual(r.total, { n: 4, est: 160, spent: 270 });
  assert.deepEqual(r.projects.map((p) => [p.project, p.n, p.est, p.spent]), [["side", 1, 10, 100], ["work", 2, 90, 110], ["home", 1, 60, 60]]);
  assert.deepEqual(r.worst.map((w) => w.title), ["g", "a"]);
  assert.equal(r.unclocked, 1);
  assert.equal(pct(r.total), "169%");
});

test("estimates keeps the 5 largest overruns", () => {
  const r = estimates([1, 2, 3, 4, 5, 6, 7].map((i) => t(`t${i}`, "w", 10, 10 + i)));
  assert.deepEqual(r.worst.map((w) => w.title), ["t7", "t6", "t5", "t4", "t3"]);
});

test("estimateLine is null without data", () => {
  assert.equal(estimateLine([]), null);
  assert.equal(estimateLine([t("a", "w", 30, null)]), null);
  assert.equal(estimateLine([t("a", "w", 60, 72)]), "Estimates: 120% of plan (1 task).");
});
