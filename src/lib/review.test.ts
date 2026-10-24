import { test } from "node:test";
import assert from "node:assert/strict";
import { isoWeek, putReview, reviewBody, reviewEntry } from "./review.ts";

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

test("reviewBody adds the estimates line after Done when tasks have an estimate and clocked time", () => {
  const body = reviewBody([{ ...t("Ship it", "2026-10-20"), effort: 60, spent: 72 }], [], [], [], 0).split("\n");
  assert.equal(body[body.indexOf("- 2026-10-20 Ship it (work)") + 1], "Estimates: 120% of plan (1 task).");
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
