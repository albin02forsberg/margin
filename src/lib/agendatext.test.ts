import { test } from "node:test";
import assert from "node:assert/strict";
import { agendaText, copyRows } from "./agendatext.ts";

const r = (date: string, title: string, o: object = {}) => ({ date, title, priority: null, done: false, ...o });

test("empty list", () => assert.equal(agendaText([]), ""));
test("flat list with done, priority, effort", () =>
  assert.equal(
    agendaText([r("2026-10-20", "Write *report*", { priority: "A", effort: 90 }), r("2026-10-20", "Call", { done: true })]),
    "- [ ] [#A] Write *report* (1h 30m)\n- [x] Call",
  ));
test("week grouping adds a heading per day", () =>
  assert.equal(
    agendaText([r("2026-10-20", "A"), r("2026-10-20", "B"), r("2026-10-21", "C")], true),
    "## Tue 20 Oct\n- [ ] A\n- [ ] B\n\n## Wed 21 Oct\n- [ ] C",
  ));

test("copyRows drops birthdays", () => {
  const t = { kind: "task" }, b = { kind: "birthday" };
  assert.deepEqual(copyRows([t, b, t]), [t, t]);
  assert.deepEqual(copyRows([b]), []);
  assert.deepEqual(copyRows([]), []);
});

test("timed rows show the time or range before the priority; closed rows don't", () =>
  assert.equal(
    agendaText([r("2026-10-20", "Standup", { time: "09:00-10:30", priority: "A" }), r("2026-10-20", "Call", { time: "14:00" }), r("2026-10-20", "Done", { time: "16:12", kind: "closed", done: true }), r("2026-10-20", "Plain", { time: null })], true),
    "## Tue 20 Oct\n- [ ] 09:00-10:30 [#A] Standup\n- [ ] 14:00 Call\n- [x] Done\n- [ ] Plain",
  ));
