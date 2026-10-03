import { test } from "node:test";
import assert from "node:assert/strict";
import { changes, GROUPS, refresh, type Field } from "./settings.ts";

const config = { notes_dir: "~/notes", expected_daily_hours: 8, profiles: ["Work", "Home"], reminders: true };
const fields: Field[] = GROUPS.flatMap((g) => g.fields).filter((f) => f.key in config);

test("only changed fields are sent, parsed", () => {
  const [form] = refresh({}, {}, config);
  assert.deepEqual(changes(fields, form, config), {});
  const edited = { ...form, expected_daily_hours: "7.5", profiles: " Work \n\nJob\n", reminders: false, notes_dir: " ~/org " };
  assert.deepEqual(changes(fields, edited, config), { notes_dir: "~/org", expected_daily_hours: 7.5, profiles: ["Work", "Job"], reminders: false });
  assert.throws(() => changes(fields, { ...form, expected_daily_hours: "" }, config), /not a number/);
});

test("a reload keeps unsaved edits and refreshes the rest", () => {
  const [form, base] = refresh({}, {}, config);
  const [next] = refresh({ ...form, notes_dir: "typing" }, base, { ...config, notes_dir: "disk", expected_daily_hours: 6 });
  assert.equal(next.notes_dir, "typing");
  assert.equal(next.expected_daily_hours, "6");
});
