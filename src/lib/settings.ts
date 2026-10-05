/** A config.toml setting on the settings page. KIND defaults to text; lists are edited one item per line. */
export type Field = { key: string; label: string; kind?: "text" | "number" | "bool" | "list"; hint?: string; step?: number | "any" };
export type Form = Record<string, string | boolean>;

const rel = "relative to the notes folder";
export const GROUPS: { name: string; fields: Field[] }[] = [
  { name: "Folders", fields: [
    { key: "notes_dir", label: "Notes folder" },
    { key: "data_dir", label: "Time-tracking folder" },
    { key: "export_dir", label: "Export folder" },
    { key: "daily_dir", label: "Journal folder", hint: rel },
    { key: "inbox", label: "Inbox file", hint: `new tasks go here, ${rel}` },
    { key: "attachments_dir", label: "Attachments folder", hint: rel },
    { key: "templates_dir", label: "Templates folder", hint: `one capture template per .org file, ${rel}; empty = off` },
  ] },
  { name: "Time", fields: [
    { key: "expected_daily_hours", label: "Expected hours per day", kind: "number", step: "any" },
    { key: "idle_threshold_minutes", label: "Ask about idle time after (minutes)", kind: "number", step: 1, hint: "0 = never" },
    { key: "profiles", label: "Profiles", kind: "list", hint: "one per line; each has its own time log" },
  ] },
  { name: "Reminders & desktop", fields: [
    { key: "reminders", label: "Desktop reminders", kind: "bool" },
    { key: "remind_before_minutes", label: "Remind before timed entries (minutes)", kind: "number", step: 1 },
    { key: "deadline_warning_days", label: "Warn about deadlines (days ahead)", kind: "number", step: 1, hint: "0 = on the day" },
    { key: "capture_shortcut", label: "Quick-capture shortcut", hint: "system-wide, e.g. Super+Shift+N; empty = off; if it doesn't fire (common on Wayland), bind `margin --capture` to a desktop shortcut" },
    { key: "close_to_tray", label: "Closing the window while tracking hides it to the tray", kind: "bool" },
  ] },
  { name: "Tasks", fields: [
    { key: "todo_keywords", label: "Open keywords", kind: "list", hint: "one per line" },
    { key: "done_keywords", label: "Done keywords", kind: "list", hint: "one per line" },
    { key: "calendar_file", label: "Calendar feed (.ics)", hint: "kept up to date with scheduled tasks and deadlines; empty = off" },
  ] },
  { name: "Integrations", fields: [
    { key: "activitywatch_url", label: "ActivityWatch server", hint: "e.g. http://localhost:5600; empty = off" },
    { key: "activity_exclude", label: "Activity to ignore", kind: "list", hint: "regexes over app names and window titles, one per line" },
    { key: "activity_meetings", label: "Meetings", kind: "list", hint: "regexes over app names, window titles and URLs, one per line" },
    { key: "activity_calendar", label: "Meeting calendar (.ics)", hint: "names meeting suggestions; empty = off" },
    { key: "ai_url", label: "Ollama server", hint: "local addresses only; pick the model with Space f a" },
  ] },
];

/** VALUE (from the config) as the form edits it. */
export const toForm = (f: Field, v: unknown): string | boolean =>
  f.kind === "bool" ? !!v : f.kind === "list" ? ((v ?? []) as string[]).join("\n") : String(v ?? "");

/** The form's value V as config.toml stores it; throws on a bad number. */
export function fromForm(f: Field, v: string | boolean): unknown {
  if (f.kind === "bool") return v;
  const s = String(v);
  if (f.kind === "list") return s.split("\n").map((l) => l.trim()).filter(Boolean);
  if (f.kind !== "number") return s.trim();
  const n = s.trim() === "" ? NaN : Number(s);
  if (!Number.isFinite(n)) throw new Error(`${f.label}: not a number`);
  return n;
}

/** FIELDS whose FORM value differs from CONFIG, ready for save_config. */
export function changes(fields: Field[], form: Form, config: Record<string, unknown>): Record<string, unknown> {
  const out: Record<string, unknown> = {};
  for (const f of fields) if (form[f.key] !== toForm(f, config[f.key])) out[f.key] = fromForm(f, form[f.key]);
  return out;
}

/** FORM refreshed from CONFIG, keeping fields edited since BASE (what the form last loaded);
 *  returns [form, base, keys of kept edits whose saved value changed meanwhile]. */
export function refresh(form: Form, base: Form, config: Record<string, unknown>): [Form, Form, string[]] {
  const next: Form = {}, kept: Form = {}, clash: string[] = [];
  for (const f of GROUPS.flatMap((g) => g.fields)) {
    next[f.key] = toForm(f, config[f.key]);
    const edited = f.key in form && form[f.key] !== base[f.key];
    kept[f.key] = edited ? form[f.key] : next[f.key];
    if (edited && next[f.key] !== base[f.key]) clash.push(f.key);
  }
  return [kept, next, clash];
}

/** A `[[views]]`/`[[templates]]` entry on the settings page. */
export type Row = Record<string, string>;
const same = (a: unknown, b: unknown) => JSON.stringify(a) === JSON.stringify(b);

/** ROWS (edited since BASE) refreshed from the config's LIST; returns [rows, base, whether a kept edit clashes]. */
export function refreshRows(rows: Row[], base: Row[], list: unknown): [Row[], Row[], boolean] {
  const next = ((list ?? []) as Row[]).map((r) => ({ ...r }));
  const edited = !same(rows, base);
  return [edited ? rows : next.map((r) => ({ ...r })), next, edited && !same(next, base)];
}

/** VIEWS trimmed for save_config; throws when one lacks a name or query. */
export function checkViews(views: Row[]): Row[] {
  const out = views.map((v) => ({ name: (v.name ?? "").trim(), query: (v.query ?? "").trim() }));
  const bad = out.findIndex((v) => !v.name || !v.query);
  if (bad >= 0) throw new Error(`View ${bad + 1} needs a name and a query`);
  return out;
}
