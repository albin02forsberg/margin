// Built-in features shipped as packages on the init-script API (#1419). Add one: a module here exporting a `Package`, listed below.
import type { Margin, Package } from "../commands.ts";
import insertTimestamp from "./insert-timestamp.ts";

export const PACKAGES: Package[] = [insertTimestamp];

/** Load every package whose id isn't in DISABLED (`disabled_packages`); a throwing one is reported by id and the rest still load. */
export function loadPackages(margin: Margin, disabled: string[], report: (msg: string) => void, pkgs = PACKAGES) {
  for (const p of pkgs) {
    if (disabled.includes(p.id)) continue;
    try { p.load(margin); } catch (e) { report(`⚠ package ${p.id}: ${e instanceof Error ? e.message : e}`); }
  }
}
