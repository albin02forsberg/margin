import { test } from "node:test";
import assert from "node:assert/strict";
import { findReviewFile, isoWeek, markers, type LedgerWeek, monthBody, pushedLine, putReview, reviewBody, reviewEntry, reviewRange, reviewWeekLabel, weekExtremes, yearBody, yearCount, yearLedgerLines } from "./review.ts";

test("reviewRange: the year runs Jan 1 to Dec 31, clipped to today, labelled YYYY (#1221)", () => {
  assert.deepEqual(reviewRange("year", new Date(2026, 10, 9)), { from: "2026-01-01", to: "2026-11-09", label: "2026" });
  assert.deepEqual(reviewRange("year", new Date(2025, 5, 1), undefined, new Date(2026, 0, 3)), { from: "2025-01-01", to: "2025-12-31", label: "2025" });
  assert.match(reviewEntry("2026", yearBody([], { projects: [], total: 0, expected: 0 }, [], null, 0, []), "year"), /^\* Yearly review 2026\n# review-begin year\n[\s\S]*# review-end year\n\*\* Went well/);
});

test("weekExtremes ranks only whole weeks inside the year that are over by today (#1221)", () => {
  const h = (n: number) => n * 3600;
  // 2026-01-01 is a Thursday: Monday weeks start 2026-01-05 (W02); the partial Dec 29 week is skipped though it is the quietest.
  const secs = { "2026-01-02": h(1), "2026-01-05": h(8), "2026-01-06": h(8), "2026-01-12": h(3), "2026-01-19": h(40), "2025-12-29": h(99) };
  assert.deepEqual(weekExtremes(secs, 2026, "monday", new Date(2026, 0, 25)), { busiest: { label: "2026-W04", hours: 40 }, quietest: { label: "2026-W03", hours: 3 } });
  // The running week (Jan 19..25) doesn't count before its last day; nor does a year's last partial week.
  assert.deepEqual(weekExtremes(secs, 2026, "monday", new Date(2026, 0, 24)), { busiest: { label: "2026-W02", hours: 16 }, quietest: { label: "2026-W03", hours: 3 } });
  assert.equal(weekExtremes(secs, 2026, "monday", new Date(2026, 0, 10)), null);
  assert.equal(weekExtremes({}, 2026, "monday", new Date(2027, 0, 5))!.busiest.label, "2026-W02"); // all zero: the first week
  // Sunday weeks start 2026-01-04, so Jan 4..10 holds the 16h.
  assert.deepEqual(weekExtremes(secs, 2026, "sunday", new Date(2026, 0, 17)), { busiest: { label: "2026-W02", hours: 16 }, quietest: { label: "2026-W03", hours: 3 } });
  // Saturday weeks: Jan 3..9 is the first whole one, Dec 26..Jan 1 of the next year the last is out (ends 2027-01-01).
  const dec = weekExtremes({ "2026-12-26": h(5) }, 2026, "saturday", new Date(2027, 1, 1))!;
  assert.equal(dec.busiest.hours, 0);
});

test("yearCount sums one year's days of a per-day map (#1221)", () => {
  assert.equal(yearCount({ "2025-12-31": 4, "2026-01-01": 2, "2026-11-09": 3 }, 2026), 5);
  assert.equal(yearCount(undefined, 2026), 0);
});

test("yearLedgerLines: spent and income against last year, the largest expense category on top (#1221)", () => {
  const z = Array(12).fill(0), mo = (jan: number, feb = 0) => [jan, feb, ...z.slice(2)];
  const eur = { currency: "EUR", income: mo(10000), expense: mo(-30000, -12000), categories: [[null, mo(-20000)], ["food", mo(-9000, -9000)], ["rent", mo(-1000)], ["pay", mo(10000)]] as [string | null, number[]][] };
  const last = { currency: "EUR", income: z, expense: mo(-50000), categories: [] };
  assert.deepEqual(yearLedgerLines([eur], [last]), ["Ledger: spent 420.00 EUR (last year 500.00 EUR), income 100.00 EUR (last year 0.00 EUR); top: #food 180.00 EUR"]);
  assert.deepEqual(yearLedgerLines([{ ...eur, categories: [] }], []), ["Ledger: spent 420.00 EUR (last year 0.00 EUR), income 100.00 EUR (last year 0.00 EUR)"]);
});

test("yearBody: weeks, notes edited and habit streaks longest first after the hours (#1221)", () => {
  const habit = (title: string, streak: number, best: number) => ({ title, habit: { streak, best, pct: 80 } });
  const body = yearBody([t("a", "2026-03-02")], { projects: [{ project: "Margin", worked: 900 }], total: 900, expected: 1600 }, [],
    { busiest: { label: "2026-W12", hours: 52.5 }, quietest: { label: "2026-W33", hours: 0 } }, 41, [habit("Run", 3, 9), habit("Read", 12, 30), { title: "x", habit: null }], ["Ledger: …"]);
  assert.equal(body, [
    "# review-begin year", "** Done (1)", "- work: 1",
    "** Hours per project (900h 00m of 1600h 00m expected)", "- Margin: 900h 00m", "Ledger: …",
    "Busiest week: 2026-W12 (52h 30m); quietest: 2026-W33 (0m)", "Notes edited: 41 (by last change)",
    "** Habit streaks (2)", "- Read: streak 12, best 30, 80% on time", "- Run: streak 3, best 9, 80% on time",
    "# review-end year",
  ].join("\n"));
  assert.doesNotMatch(yearBody([], { projects: [], total: 0, expected: 0 }, [], null, 0, []), /Busiest|Habit streaks/);
});

test("reviewWeekLabel names the ISO week holding most of the span", () => {
  assert.equal(reviewWeekLabel(new Date(2026, 9, 12)), "2026-W42"); // Monday
  assert.equal(reviewWeekLabel(new Date(2026, 9, 11)), "2026-W42"); // Sunday start: not W41
  assert.equal(reviewWeekLabel(new Date(2026, 9, 10)), "2026-W42"); // Saturday start
  assert.equal(reviewWeekLabel(new Date(2026, 11, 28)), "2026-W53"); // year boundary
  assert.equal(reviewWeekLabel(new Date(2026, 11, 27)), "2026-W53");
});

const t = (title: string, date: string, category = "work") => ({ title, category, date });

test("isoWeek numbers weeks from Monday, the Thursday picking the year", () => {
  assert.equal(isoWeek("2026-10-23"), "2026-W43");
  assert.equal(isoWeek("2026-10-19"), "2026-W43");
  assert.equal(isoWeek("2026-10-25"), "2026-W43");
  assert.equal(isoWeek("2027-01-01"), "2026-W53");
  assert.equal(isoWeek("2024-12-30"), "2025-W01");
});

test("reviewBody lists each part, with a placeholder when empty", () => {
  const body = reviewBody([t("Ship it", "2026-10-20")], [{ project: "Margin", worked: 2.5 }, { project: "Old", worked: 0 }], [], [t("Plan", "2026-10-27", "")], 1);
  assert.equal(body, [
    "# review-begin", "Inbox: 1 entry waiting.",
    "** Done (1)", "- 2026-10-20 Ship it (work)",
    "** Hours per project (2h 30m)", "- Margin: 2h 30m",
    "** Overdue (0)", "- nothing overdue",
    "** Scheduled next week (1)", "- 2026-10-27 Plan",
    "# review-end",
  ].join("\n"));
  assert.match(reviewBody([], [], [], [], 0), /no time tracked/);
});

test("reviewBody counts parked someday tasks after the inbox line, and leaves the line out at 0", () => {
  assert.equal(reviewBody([], [], [], [], 0, [], 2).split("\n")[2], "Someday/maybe: 2 parked tasks (Someday/maybe review).");
  assert.match(reviewBody([], [], [], [], 0, [], 1), /: 1 parked task \(/);
  assert.doesNotMatch(reviewBody([], [], [], [], 0), /Someday/);
});

test("reviewBody adds the estimates line after Done when tasks have an estimate and clocked time", () => {
  const body = reviewBody([{ ...t("Ship it", "2026-10-20"), effort: 60, spent: 72 }], [], [], [], 0).split("\n");
  assert.equal(body[body.indexOf("- 2026-10-20 Ship it (work)") + 1], "Estimates: 120% of plan (1 task).");
});

test("findReviewFile scans newest first in chunks and stops at the first matching chunk", async () => {
  const files = Array.from({ length: 70 }, (_, i) => `f${i}`), [B, E] = markers();
  const run = async (has: string[]) => {
    let reads = 0;
    const got = await findReviewFile(files, async (f) => { reads++; if (f === "f60") throw new Error("bad"); return has.includes(f) ? `${B}\nold\n${E}` : ""; }, "b");
    return [got, reads];
  };
  assert.deepEqual(await run([]), ["", 70]);
  assert.deepEqual(await run(["f69"]), ["f69", 32]);
  assert.deepEqual(await run(["f20", "f25"]), ["f25", 64]);
  assert.deepEqual(await run(["f60", "f50", "f65"]), ["f65", 32]);
});

test("putReview: no markers means a fresh entry; a re-run replaces only the marked span", () => {
  assert.equal(putReview("* 09:00 standup\n", "x"), null);
  const first = `* 09:00 standup\n${reviewEntry("2026-W43", reviewBody([], [], [], [], 0))}\n`;
  const edited = first.replace("** Went well", "** Went well\nshipped a lot").replace("Inbox:", "my note\nInbox:");
  const again = putReview(edited, reviewBody([t("Done now", "2026-10-23")], [], [], [], 3))!;
  assert.match(again, /\* 09:00 standup\n\* Weekly review 2026-W43\n# review-begin\nInbox: 3 entries/);
  assert.match(again, /- 2026-10-23 Done now \(work\)/);
  assert.match(again, /# review-end\n\*\* Went well\nshipped a lot\n\*\* To change\n$/);
  assert.doesNotMatch(again, /my note/); // edits inside the markers are overwritten
  assert.equal(again.split("# review-begin").length, 2);
});

test("putReview keeps CRLF line endings", () => {
  const out = putReview("a\r\n# review-begin\r\nold\r\n# review-end\r\nb", "# review-begin\nnew\n# review-end")!;
  assert.equal(out, "a\r\n# review-begin\r\nnew\r\n# review-end\r\nb");
});

test("reviewBody lists goals between Hours and Overdue, and omits the section without any", () => {
  const g = (title: string, done: number, total: number, days_left: number | null, bad_target = false) => ({ title, done, total, percent: total ? Math.round((100 * done) / total) : null, days_left, bad_target });
  const body = reviewBody([], [], [], [], 0, [g("Launch", 2, 3, 8), g("Old", 0, 0, -3), g("Idea", 1, 1, null), g("Typo", 0, 1, null, true)]).split("\n");
  const i = body.indexOf("** Goals (4)");
  assert.deepEqual(body.slice(i - 1, i + 6), ["- no time tracked", "** Goals (4)", "- Launch: 2/3 done (67%), 8d left", "- Old: no tasks, 3d overdue", "- Idea: 1/1 done (100%), no target", "- Typo: 0/1 done (0%), bad target", "** Overdue (0)"]);
  assert.ok(!reviewBody([], [], [], [], 0).includes("Goals"));
});

test("reviewBody lists the top 3 most-pushed open tasks, capped, stable and shortened", () => {
  const p = (title: string, pushed?: number) => ({ ...t(title, "2026-10-20"), pushed });
  const body = reviewBody([], [], [], [], 0, [], 0, [p("a", 3), p("b", 5), p("c", 3), p("d", 4), p("x".repeat(50), 9), p("low", 2), p("none")]).split("\n");
  assert.equal(body.find((l) => l.startsWith("Pushed:")), `Pushed: ${"x".repeat(39)}… (9x), b (5x), d (4x)`);
  assert.equal(pushedLine([p("a", 3), p("c", 3)]), "Pushed: a (3x), c (3x)");
  assert.ok(!reviewBody([], [], [], [], 0, [], 0, [p("low", 2)]).includes("Pushed"));
});

test("reviewBody adds ledger lines after the hours, per currency, only with entries (#1197)", () => {
  const lines = (l?: LedgerWeek) => reviewBody([], [{ project: "Margin", worked: 1 }], [], [], 0, [], 0, [], l).split("\n");
  assert.ok(!lines().some((l) => l.includes("Ledger") || l.includes("budget")));
  const eur = { currency: "EUR", spent: 41200, last: 53000, income: 0, top: ["food", 18000] as [string, number] };
  const one = lines([[eur], []]), at = one.indexOf("- Margin: 1h 00m");
  assert.equal(one[at + 1], "Ledger: spent 412.00 EUR (last week 530.00 EUR), income 0.00 EUR; top: #food 180.00 EUR");
  const two = lines([[eur, { currency: "SEK", spent: 0, last: 400, income: 5000, top: null }], [{ category: "food", currency: "EUR", budget: 40000, used: 43000 }, { category: "fun", currency: "EUR", budget: 5000, used: 5050 }]]);
  assert.deepEqual(two.slice(at + 2, at + 4), [
    "Ledger: spent 0.00 SEK (last week 4.00 SEK), income 50.00 SEK",
    "Over budget this month: #food 430.00/400.00 EUR, #fun 50.50/50.00 EUR",
  ]);
});

test("reviewRange gives the month's first day, the end clipped to today, and a YYYY-MM label (#1220)", () => {
  assert.deepEqual(reviewRange("month", new Date(2026, 10, 9)), { from: "2026-11-01", to: "2026-11-09", label: "2026-11" });
  assert.deepEqual(reviewRange("month", new Date(2024, 1, 10), undefined, new Date(2024, 5, 1)), { from: "2024-02-01", to: "2024-02-29", label: "2024-02" }); // leap Feb
  assert.equal(reviewRange("month", new Date(2025, 1, 3), undefined, new Date(2025, 5, 1)).to, "2025-02-28");
  assert.deepEqual(reviewRange("month", new Date(2026, 11, 31), undefined, new Date(2027, 0, 2)), { from: "2026-12-01", to: "2026-12-31", label: "2026-12" });
  assert.deepEqual(reviewRange("month", new Date(2027, 0, 1)), { from: "2027-01-01", to: "2027-01-01", label: "2027-01" });
  assert.deepEqual(reviewRange("week", new Date(2026, 9, 14)), { from: "2026-10-12", to: "2026-10-14", label: "2026-W42" });
});

test("monthBody: done by category, hours against expected, ledger against last month", () => {
  const done = [t("a", "2026-11-02"), t("b", "2026-11-03"), t("c", "2026-11-04", "home"), t("d", "2026-11-05", "")];
  const eur = { currency: "EUR", spent: 41200, last: 53000, income: 0, top: null };
  assert.equal(monthBody(done, { projects: [{ project: "Margin", worked: 30 }, { project: "Old", worked: 0 }], total: 30, expected: 48 }, [], [[eur], []]), [
    "# review-begin month", "** Done (4)", "- work: 2", "- home: 1", "- no category: 1",
    "** Hours per project (30h 00m of 48h 00m expected)", "- Margin: 30h 00m",
    "Ledger: spent 412.00 EUR (last month 530.00 EUR), income 0.00 EUR",
    "# review-end month",
  ].join("\n"));
  assert.match(monthBody([], { projects: [], total: 0, expected: 0 }), /nothing closed[\s\S]*no time tracked/);
});

test("putReview keeps week and month blocks in one file apart; re-runs replace only their own (#1220)", () => {
  const none = { projects: [], total: 0, expected: 0 };
  const week = reviewEntry("2026-W45", reviewBody([], [], [], [], 0)), month = reviewEntry("2026-11", monthBody([], none), "month");
  assert.match(month, /^\* Monthly review 2026-11\n# review-begin month\n/);
  assert.equal(putReview(`* x\n${week}\n`, "m", "month"), null); // a week block isn't a month block
  assert.equal(putReview(`* x\n${month}\n`, "w"), null); // nor the other way round
  const mb = monthBody([t("New", "2026-11-09")], none), once = putReview(`* x\n${week}\n${month}\n`, mb, "month")!;
  assert.equal(putReview(once, mb, "month"), once); // idempotent
  assert.ok(once.startsWith(`* x\n${week}\n* Monthly review 2026-11\n# review-begin month\n** Done (1)\n- work: 1\n`));
  assert.equal(once.split("# review-begin month").length, 2);
  assert.equal(putReview(once, reviewBody([], [], [], [], 7))!.split("Inbox: 7 entries").length, 2);
});
