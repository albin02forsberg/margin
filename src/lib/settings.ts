/** A config.toml setting on the settings page. KIND defaults to text; lists are edited one item per line,
 *  numbers (a list of them) on one line. */
export type Field = { key: string; label: string; kind?: "text" | "number" | "numbers" | "bool" | "list" | "multiline"; hint?: string; step?: number | "any" };
export type Form = Record<string, string | boolean>;

const rel = "relative to the notes folder";
export const GROUPS: { name: string; fields: Field[] }[] = [
  { name: "Folders", fields: [
    { key: "notes_dir", label: "Notes folder" },
    { key: "data_dir", label: "Time-tracking folder" },
    { key: "export_dir", label: "Export folder" },
    { key: "daily_dir", label: "Journal folder", hint: rel },
    { key: "journal_template", label: "Journal template", kind: "multiline", hint: "org text added to each new journal entry, e.g. * Plan; %t and %u expand to that day's date, prompts and %? don't; empty = nothing" },
    { key: "inbox", label: "Inbox file", hint: `new tasks go here, ${rel}` },
    { key: "attachments_dir", label: "Attachments folder", hint: rel },
    { key: "images_dir", label: "Images folder", hint: "pasted and dropped images go here, in the note's own folder and shared by its notes; empty = the attachments folder" },
    { key: "templates_dir", label: "Templates folder", hint: `one capture template per .org file, ${rel}; empty = off` },
  ] },
  { name: "Time", fields: [
    { key: "expected_daily_hours", label: "Expected hours per day", kind: "number", step: "any" },
    { key: "weekday_hours", label: "Expected hours per weekday", kind: "numbers", hint: "7 numbers, Monday to Sunday, e.g. 8 8 8 8 6 0 0 for a short Friday; empty = the hours per day Monday to Friday" },
    { key: "public_holidays", label: "Public holidays", hint: "se = Swedish red days count as 0 expected hours; none = no holidays" },
    { key: "extra_days_off", label: "Extra days off", kind: "list", hint: "one date per line, e.g. 2026-12-24; 0 expected hours like a holiday" },
    { key: "idle_threshold_minutes", label: "Ask about idle time after (minutes)", kind: "number", step: 1, hint: "0 = never" },
    { key: "focus_minutes", label: "Nudge a break after (minutes)", kind: "number", step: 1, hint: "of one running session, since clocking in or resuming; the top-bar clock counts down; 0 = off" },
    { key: "focus_break_minutes", label: "Suggested break (minutes)", kind: "number", step: 1 },
    { key: "profiles", label: "Profiles", kind: "list", hint: "one per line; each has its own time log" },
  ] },
  { name: "Reminders & desktop", fields: [
    { key: "reminders", label: "Desktop reminders", kind: "bool" },
    { key: "remind_before_minutes", label: "Remind before timed entries (minutes)", kind: "number", step: 1 },
    { key: "deadline_warning_days", label: "Warn about deadlines (days ahead)", kind: "number", step: 1, hint: "0 = on the day" },
    { key: "capture_shortcut", label: "Quick-capture shortcut", hint: "system-wide, e.g. Super+Shift+N; empty = off; if it doesn't fire (common on Wayland), bind `margin --capture` to a desktop shortcut" },
    { key: "close_to_tray", label: "Closing the window while tracking hides it to the tray", kind: "bool" },
    { key: "restore_tabs", label: "Reopen the last session's tabs on startup", kind: "bool" },
    { key: "nightly_updates", label: "Nightly updates", kind: "bool", hint: "update to daily, untested builds of the latest code; switching back waits for the next stable release" },
  ] },
  { name: "Editor", fields: [
    { key: "spellcheck", label: "Spell check org notes", kind: "bool", hint: "underlines misspelled words with the system spell checker; on Linux the webview needs a system checker (hunspell/enchant) installed" },
    { key: "spellcheck_languages", label: "Spell check languages", kind: "list", hint: "Linux only; one per line, e.g. sv_SE or en_US; needs that hunspell dictionary installed; empty = system locale" },
  ] },
  { name: "Tasks", fields: [
    { key: "todo_keywords", label: "Open keywords", kind: "list", hint: "one per line" },
    { key: "done_keywords", label: "Done keywords", kind: "list", hint: "one per line" },
    { key: "stuck_next_keywords", label: "Next-action keywords", kind: "list", hint: "one per line; a project (open task with open subtasks) with none of these below it, nothing scheduled and no deadline is stuck (Space x S)" },
    { key: "log_done", label: "Record when tasks are done", kind: "bool", hint: "adds CLOSED: [date time] below the heading, as org-log-done does; the board's Done column shows this week's" },
    { key: "calendar_file", label: "Calendar feed (.ics)", hint: "kept up to date with scheduled tasks and deadlines; empty = off" },
  ] },
  { name: "Integrations", fields: [
    { key: "activity_watcher", label: "Built-in activity watcher", kind: "bool", hint: "records the active app and window title every 5 s into the time-tracking folder's activity/ for suggestions in the Time view; stays on this computer, never uploaded. Used when ActivityWatch is off or not running. Wayland: KDE, Hyprland, Sway and other wlroots compositors, and GNOME with a small Shell extension (installed from here)" },
    { key: "activity_media_signal", label: "Mark calls from mic and camera use", kind: "bool", hint: "notes only whether a microphone or camera is in use (never audio, video or which app) in activity/media/, so a block spent mostly on a call becomes a meeting suggestion; works with ActivityWatch or the built-in watcher. macOS only for now (the mic needs macOS 14+)" },
    { key: "activity_retention_days", label: "Keep the activity log (days)", kind: "number", step: 1, hint: "older days are deleted; 0 = keep all" },
    { key: "activitywatch_url", label: "ActivityWatch server", hint: "e.g. http://localhost:5600; empty = off" },
    { key: "activity_exclude", label: "Activity to ignore", kind: "list", hint: "regexes over app names and window titles, one per line" },
    { key: "activity_meetings", label: "Meetings", kind: "list", hint: "regexes over app names, window titles and URLs, one per line" },
    { key: "activity_calendar", label: "Meeting calendar (.ics)", hint: "a file or an https:// URL; names meeting suggestions; empty = off" },
    { key: "ai_url", label: "Ollama server", hint: "local addresses only; pick the model with Space f a" },
  ] },
  { name: "AI chat", fields: [
    { key: "ai_source_notes", label: "Send matching notes", kind: "bool", hint: "these switches apply while “Use my notes” is ticked in the chat; excerpts of the notes that match the question and of the notes linked to them" },
    { key: "ai_source_tasks", label: "Send open tasks and the agenda", kind: "bool", hint: "when the question asks about tasks, today, the week, deadlines or meetings" },
    { key: "ai_source_time", label: "Send the time log", kind: "bool", hint: "the last 7 days, when the question asks about hours, time or projects" },
    { key: "ai_exclude", label: "Notes the AI never sees", kind: "list", hint: "one per line; a note whose path in the notes folder contains one (any case, e.g. private/ or diary) is never sent: not as a match, a linked note, the open note or for tasks" },
  ] },
];

/** The settings page's category chips: all, each field group, then the list cards. */
export const CATEGORIES = ["All", ...GROUPS.map((g) => g.name), "Saved views", "Capture templates"];
/** Whether a card or field of group NAME shows under category CAT and filter Q: Q (any case) is
 *  in the group name or one of TEXT (a field's label, hint and key). */
export function shows(cat: string, q: string, name: string, ...text: (string | undefined)[]) {
  const s = q.trim().toLowerCase();
  return (cat === "All" || cat === name) && (!s || [name, ...text].some((t) => t?.toLowerCase().includes(s)));
}

/** Whether ticking the activity watcher (BEFORE saved, AFTER ticked) first needs the macOS dialog:
 *  GRANTED is the Screen Recording permission (null off macOS), OK an earlier confirmation. */
export const needsMacPrompt = (granted: boolean | null, before: boolean, after: boolean, ok: boolean) =>
  granted === false && !before && after && !ok;

/** VALUE (from the config) as the form edits it. */
export const toForm = (f: Field, v: unknown): string | boolean =>
  f.kind === "bool" ? !!v : f.kind === "list" ? ((v ?? []) as string[]).join("\n") : f.kind === "numbers" ? ((v ?? []) as number[]).join(" ") : String(v ?? "");

/** The form's value V as config.toml stores it; throws on a bad number. */
export function fromForm(f: Field, v: string | boolean): unknown {
  if (f.kind === "bool") return v;
  const s = String(v);
  if (f.kind === "list") return s.split("\n").map((l) => l.trim()).filter(Boolean);
  if (f.kind === "numbers") return s.split(/[\s,;]+/).filter(Boolean).map((x) => {
    const n = Number(x);
    if (!Number.isFinite(n)) throw new Error(`${f.label}: ${x} is not a number`);
    return n;
  });
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

/** ROWS (edited since BASE) refreshed from the config's LIST (nulls as ""); returns [rows, base, whether a kept edit clashes]. */
export function refreshRows(rows: Row[], base: Row[], list: unknown): [Row[], Row[], boolean] {
  const next = ((list ?? []) as Record<string, string | null>[]).map((r) => Object.fromEntries(Object.entries(r).map(([k, v]) => [k, v ?? ""])));
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

/** TEMPLATES trimmed for save_config (empty file and heading left out); throws unless each has
 *  a one-character key of its own and a name. */
export function checkTemplates(tpls: Row[]): Row[] {
  return tpls.map((t, i) => {
    const r: Row = { key: (t.key ?? "").trim(), name: (t.name ?? "").trim(), body: t.body ?? "" };
    for (const k of ["file", "heading"]) if ((t[k] ?? "").trim()) r[k] = t[k].trim();
    const other = tpls.findIndex((u) => (u.key ?? "").trim() === r.key);
    if ([...r.key].length !== 1) throw new Error(`Template ${i + 1} needs a one-character key`);
    if (other < i) throw new Error(`Templates ${other + 1} and ${i + 1} have the same key`);
    if (!r.name) throw new Error(`Template ${i + 1} needs a name`);
    return r;
  });
}
