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
