/** A project as `tc_projects` / `tc_save_project` see it (projects.toml). */
export type Project = { export_code: string; rounding: number | null; round_up: boolean; active: boolean };
/** One row of the project settings page; ROUNDING is "" for none. */
export type Row = { name: string; code: string; rounding: string; up: boolean; active: boolean; dirty: boolean };

export const ROUNDINGS: [string, string][] = [["0.25", "Quarter hour"], ["0.5", "Half hour"], ["1", "Whole hour"], ["", "Don't round"]];

export const toRow = (name: string, p: Project): Row =>
  ({ name, code: p.export_code, rounding: p.rounding == null ? "" : String(p.rounding), up: p.round_up, active: p.active, dirty: false });

/** Round-up only means something with rounding, so it's saved off without. */
export const toProject = (r: Row): Project =>
  ({ export_code: r.code.trim(), rounding: r.rounding ? +r.rounding : null, round_up: !!r.rounding && r.up, active: r.active });

/** Rows for PROJECTS, keeping the unsaved edits in OLD. */
export const merge = (old: Row[], projects: Record<string, Project>): Row[] =>
  Object.entries(projects).map(([n, p]) => old.find((r) => r.name === n && r.dirty) ?? toRow(n, p));

/** A `tc_dashboard` projects row: hours this week (worked, billable) and this calendar month. */
export type Tracked = { project: string; code: string; worked: number; billable: number; month: number };
/** An open task as `todos` returns it (the fields the cards use). */
export type Task = { path: string; line: number; title: string; category: string; keyword: string | null };
export type Card = Tracked & { share: number; tasks: Task[] };

/** A card per project shown in projects.toml or tracked this month, most worked this week first. A project's open tasks
 *  are those whose org category is its name (any case); a file's category defaults to its name, so acme.org is Acme's. */
export function cards(projects: Record<string, Project>, tracked: Tracked[], tasks: Task[]): Card[] {
  const total = tracked.reduce((s, p) => s + p.worked, 0);
  const rows = [...tracked];
  for (const [n, p] of Object.entries(projects)) if (p.active && !rows.some((r) => r.project === n)) rows.push({ project: n, code: p.export_code || n, worked: 0, billable: 0, month: 0 });
  return rows
    .map((r) => ({ ...r, share: total > 0 ? r.worked / total : 0, tasks: tasks.filter((t) => t.category.toLowerCase() === r.project.toLowerCase()) }))
    .sort((a, b) => b.worked - a.worked || b.month - a.month || a.project.localeCompare(b.project));
}
