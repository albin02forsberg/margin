// Coverage ratchet (#252): fails when coverage drops below `coverage-baseline.json`.
// Usage: node scripts/coverage-ratchet.ts [--write] rust=lcov-rust.info ts=lcov-ts.info
// Files in `coverage-exclude.json` (path -> reason) are left out. A suite or file with no
// baseline never fails; `--write` records the current numbers (raise it as gaps close).
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { isAbsolute, relative } from "node:path";
import { fileURLToPath } from "node:url";

type Count = { hit: number; found: number };
export type Report = Record<string, { lines: Count; branches: Count }>;
/** Percentages, rounded to 2 decimals; `branches` only where lcov has branch data. */
export type Suite = { lines: number; branches?: number; files: Record<string, number> };
export type Baseline = { tolerance: number; suites: Record<string, Suite> };

/** Lines and branches per file from an lcov file; absolute paths become relative to ROOT. */
export function parseLcov(text: string, root: string): Report {
  const out: Report = {};
  let cur: Report[string] | undefined;
  for (const line of text.split("\n")) {
    const i = line.indexOf(":");
    const [k, v] = i < 0 ? [line.trim(), ""] : [line.slice(0, i), line.slice(i + 1).trim()];
    if (k === "SF") {
      const f = isAbsolute(v) ? relative(root, v) : v;
      cur = out[f] ??= { lines: { hit: 0, found: 0 }, branches: { hit: 0, found: 0 } };
    } else if (cur && k === "LF") cur.lines.found += +v;
    else if (cur && k === "LH") cur.lines.hit += +v;
    else if (cur && k === "BRF") cur.branches.found += +v;
    else if (cur && k === "BRH") cur.branches.hit += +v;
    else if (k === "end_of_record") cur = undefined;
  }
  return out;
}

const pct = (c: Count) => (c.found ? Math.round((10000 * c.hit) / c.found) / 100 : 100);

/** The percentages of REPORT without the EXCLUDEd files. */
export function summarize(report: Report, exclude: Record<string, string>): Suite {
  const lines = { hit: 0, found: 0 }, branches = { hit: 0, found: 0 }, files: Record<string, number> = {};
  for (const [f, r] of Object.entries(report).sort()) {
    if (f in exclude) continue;
    lines.hit += r.lines.hit; lines.found += r.lines.found;
    branches.hit += r.branches.hit; branches.found += r.branches.found;
    files[f] = pct(r.lines);
  }
  return { lines: pct(lines), ...(branches.found ? { branches: pct(branches) } : {}), files };
}

/** One message per metric of CUR that is more than TOL points below BASE. */
export function regressions(name: string, cur: Suite, base: Suite | undefined, tol: number): string[] {
  if (!base) return [];
  const out: string[] = [];
  const check = (what: string, now: number | undefined, was: number | undefined) => {
    if (now !== undefined && was !== undefined && now < was - tol) out.push(`${name} ${what}: ${now}% < baseline ${was}%`);
  };
  check("lines", cur.lines, base.lines);
  check("branches", cur.branches, base.branches);
  for (const [f, now] of Object.entries(cur.files)) check(`${f} lines`, now, base.files[f]);
  return out;
}

function main(args: string[]) {
  const write = args.includes("--write");
  const read = (f: string) => (existsSync(f) ? JSON.parse(readFileSync(f, "utf8")) : undefined);
  const exclude: Record<string, string> = read("coverage-exclude.json") ?? {};
  const baseline: Baseline | undefined = read("coverage-baseline.json");
  const next: Baseline = { tolerance: baseline?.tolerance ?? 0.5, suites: { ...baseline?.suites } };
  const failed: string[] = [];
  for (const arg of args.filter((a) => a.includes("="))) {
    const [name, file] = arg.split("=");
    const cur = summarize(parseLcov(readFileSync(file, "utf8"), process.cwd()), exclude); // a missing lcov file throws
    const base = baseline?.suites[name];
    console.log(`${name}: lines ${cur.lines}%${cur.branches !== undefined ? `, branches ${cur.branches}%` : ""}` +
      (base ? ` (baseline ${base.lines}%${base.branches !== undefined ? `, ${base.branches}%` : ""})` : " (no baseline yet)"));
    failed.push(...regressions(name, cur, base, next.tolerance));
    next.suites[name] = cur;
  }
  if (write) {
    writeFileSync("coverage-baseline.json", JSON.stringify(next, null, 2) + "\n");
    console.log("Wrote coverage-baseline.json");
  } else if (failed.length) {
    console.log(`\nCoverage dropped more than ${next.tolerance} points below coverage-baseline.json:`);
    for (const m of failed) console.log(`- ${m}`);
    console.log("Add tests, or exclude untestable code in coverage-exclude.json with a reason.");
    process.exitCode = 1;
  } else if (baseline) console.log("Coverage is at or above the baseline.");
}

if (process.argv[1] === fileURLToPath(import.meta.url)) main(process.argv.slice(2));
