import { test } from "node:test";
import assert from "node:assert/strict";
import { changes, checkFeeds, checkSnippets, checkTemplates, checkViews, GROUPS, hide, needsMacPrompt, refresh, refreshRows, shows, type Field } from "./settings.ts";

const config = { notes_dir: "~/notes", expected_daily_hours: 8, profiles: ["Work", "Home"], reminders: true };
const fields: Field[] = GROUPS.flatMap((g) => g.fields).filter((f) => f.key in config);

test("only changed fields are sent, parsed", () => {
  const [form] = refresh({}, {}, config);
  assert.deepEqual(changes(fields, form, config), {});
  const edited = { ...form, expected_daily_hours: "7.5", profiles: " Work \n\nJob\n", reminders: false, notes_dir: " ~/org " };
  assert.deepEqual(changes(fields, edited, config), { notes_dir: "~/org", expected_daily_hours: 7.5, profiles: ["Work", "Job"], reminders: false });
  assert.throws(() => changes(fields, { ...form, expected_daily_hours: "" }, config), /not a number/);
});

test("weekday hours are numbers on one line", () => {
  const f = GROUPS.flatMap((g) => g.fields).find((f) => f.key === "weekday_hours")!;
  const cfg = { weekday_hours: [8, 8, 8, 8, 6.5, 0, 0] };
  const [form] = refresh({}, {}, cfg);
  assert.equal(form.weekday_hours, "8 8 8 8 6.5 0 0");
  assert.deepEqual(changes([f], { weekday_hours: "8, 8,8 8 6 0 0 " }, cfg), { weekday_hours: [8, 8, 8, 8, 6, 0, 0] });
  assert.deepEqual(changes([f], { weekday_hours: " " }, cfg), { weekday_hours: [] });
  assert.equal(refresh({}, {}, {})[0].weekday_hours, "");
  assert.throws(() => changes([f], { weekday_hours: "8 x" }, cfg), /x is not a number/);
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

test("snippets need a typeable key of their own; text is kept as typed", () => {
  assert.deepEqual(checkSnippets([{ key: " <mtg ", text: "  * %?\n" }]), [{ key: "mtg", text: "  * %?\n" }]);
  assert.throws(() => checkSnippets([{ key: "", text: "x" }]), /Snippet 1 needs a key/);
  assert.throws(() => checkSnippets([{ key: "a b", text: "x" }]), /needs a key/);
  assert.throws(() => checkSnippets([{ key: "m", text: "" }, { key: "<m", text: "" }]), /Snippets 1 and 2/);
});

test("feeds need an http(s) URL of their own; empty file and heading are left out", () => {
  assert.deepEqual(checkFeeds([{ name: " Blog ", url: " https://b.example/rss ", file: " ", heading: "" }]), [{ name: "Blog", url: "https://b.example/rss" }]);
  assert.deepEqual(checkFeeds([{ name: "", url: "http://x/feed", file: "news.org", heading: "Later" }]), [{ name: "", url: "http://x/feed", file: "news.org", heading: "Later" }]);
  assert.throws(() => checkFeeds([{ name: "a", url: "" }]), /Feed 1 needs an http\(s\) URL/);
  assert.throws(() => checkFeeds([{ name: "a", url: "file:///etc/passwd" }]), /needs an http/);
  assert.throws(() => checkFeeds([{ url: "https://a/x" }, { url: " https://a/x" }]), /Feeds 1 and 2 have the same URL/);
});

test("the macOS watcher dialog shows once, only when turning it on without the permission", () => {
  assert.equal(needsMacPrompt(false, false, true, false), true);
  assert.equal(needsMacPrompt(null, false, true, false), false, "not macOS");
  assert.equal(needsMacPrompt(true, false, true, false), false, "already granted");
  assert.equal(needsMacPrompt(false, false, true, true), false, "confirmed before");
  assert.equal(needsMacPrompt(false, true, true, false), false, "already on");
  assert.equal(needsMacPrompt(false, false, false, false), false, "turning it off");
});

test("category chips and the filter narrow the settings shown", () => {
  const shown = (cat: string, q: string) => GROUPS.flatMap((g) => g.fields.filter((f) => shows(cat, q, g.name, f.label, f.hint, f.key)).map((f) => f.key));
  assert.equal(shown("All", "").length, GROUPS.flatMap((g) => g.fields).length);
  assert.deepEqual(shown("Time", ""), ["expected_daily_hours", "weekday_hours", "public_holidays", "extra_days_off", "idle_threshold_minutes", "focus_minutes", "focus_break_minutes", "max_session_hours", "profiles"]);
  assert.deepEqual(shown("All", "IDLE"), ["idle_threshold_minutes"]);
  assert.deepEqual(shown("All", "tasks"), ["inbox", "todo_keywords", "done_keywords", "stuck_next_keywords", "log_done", "calendar_file", "ai_source_tasks", "ai_exclude"]); // inbox and the AI ones by label or hint, Tasks by group name
  assert.deepEqual(shown("Time", "keywords"), []);
  assert.ok(shows("All", "view", "Saved views") && !shows("Time", "", "Saved views"));
});

test("hide adds or drops one area of a list field, keeping the others", () => {
  assert.equal(hide("", "ledger", true), "ledger");
  assert.equal(hide("ledger", "habits", true), "ledger\nhabits");
  assert.equal(hide("ledger\nhabits", "ledger", false), "habits");
  assert.equal(hide("habits", "habits", false), "");
  assert.equal(hide("ledger", "ledger", true), "ledger"); // no duplicate
});
