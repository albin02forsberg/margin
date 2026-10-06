import { test } from "node:test";
import assert from "node:assert/strict";
import { changes, checkTemplates, checkViews, GROUPS, needsMacPrompt, refresh, refreshRows, type Field } from "./settings.ts";

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
  const edited = { ...form, notes_dir: "typing", reminders: false };
  const [next, , clash] = refresh(edited, base, { ...config, notes_dir: "disk", expected_daily_hours: 6 });
  assert.equal(next.notes_dir, "typing");
  assert.equal(next.expected_daily_hours, "6");
  assert.deepEqual(clash, ["notes_dir"]); // reminders was edited too, but not changed on disk
  assert.deepEqual(refresh(edited, base, config)[2], []);
});

test("views are trimmed and need a name and query", () => {
  assert.deepEqual(checkViews([{ name: " A ", query: "tag:a " }]), [{ name: "A", query: "tag:a" }]);
  assert.throws(() => checkViews([{ name: "A", query: "a" }, { name: "B", query: " " }]), /View 2/);
});

test("rows keep unsaved edits across a reload", () => {
  const [rows, base] = refreshRows([], [], [{ name: "A", query: "a" }]);
  assert.deepEqual(refreshRows(rows, base, [{ name: "B", query: "b" }])[0], [{ name: "B", query: "b" }]);
  const edited = [...rows, { name: "", query: "" }];
  assert.deepEqual(refreshRows(edited, base, [{ name: "A", query: "a" }]), [edited, base, false]);
  assert.equal(refreshRows(edited, base, [])[2], true);
});

test("templates need a unique one-character key and a name", () => {
  const t = { key: " j ", name: " Journal ", file: " ", heading: " Log ", body: "* %?\n" };
  assert.deepEqual(checkTemplates([t]), [{ key: "j", name: "Journal", body: "* %?\n", heading: "Log" }]);
  assert.throws(() => checkTemplates([{ ...t, key: "jj" }]), /one-character/);
  assert.throws(() => checkTemplates([{ ...t, key: "" }]), /one-character/);
  assert.throws(() => checkTemplates([t, { ...t, key: "k" }, { ...t, key: "j" }]), /Templates 1 and 3/);
  assert.throws(() => checkTemplates([{ ...t, name: "" }]), /needs a name/);
  assert.deepEqual(refreshRows([], [], [{ key: "n", heading: null }])[1], [{ key: "n", heading: "" }]);
});

test("the macOS watcher dialog shows once, only when turning it on without the permission", () => {
  assert.equal(needsMacPrompt(false, false, true, false), true);
  assert.equal(needsMacPrompt(null, false, true, false), false, "not macOS");
  assert.equal(needsMacPrompt(true, false, true, false), false, "already granted");
  assert.equal(needsMacPrompt(false, false, true, true), false, "confirmed before");
  assert.equal(needsMacPrompt(false, true, true, false), false, "already on");
  assert.equal(needsMacPrompt(false, false, false, false), false, "turning it off");
});
