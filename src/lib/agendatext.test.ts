import { test } from "node:test";
import assert from "node:assert/strict";
import { agendaText } from "./agendatext.ts";

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
