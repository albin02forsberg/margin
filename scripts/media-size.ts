// README media cap (#481): every clip in GIFS exists, each file under EACH bytes and all of
// docs/screenshots + docs/gifs under TOTAL. Usage: node scripts/media-size.ts
import { readdirSync, statSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

/** The clips e2e/screenshots.ts records, as docs/gifs/<name>.gif. */
export const GIFS = ["capture", "today", "time", "links", "vim"];
export const EACH = 1.5 * 1024 * 1024;
export const TOTAL = 10 * 1024 * 1024;
const DIRS = ["docs/screenshots", "docs/gifs"];
const mb = (n: number) => `${(n / 1024 / 1024).toFixed(2)} MB`;

/** What's wrong with FILES (path -> bytes): missing clips, files over EACH, a total over TOTAL. */
export function problems(files: Record<string, number>, gifs = GIFS, each = EACH, total = TOTAL): string[] {
  const out = gifs.map((g) => `docs/gifs/${g}.gif`).filter((f) => !(f in files)).map((f) => `${f} is missing`);
  for (const [f, n] of Object.entries(files)) if (n > each) out.push(`${f} is ${mb(n)}, over ${mb(each)}`);
  const sum = Object.values(files).reduce((a, b) => a + b, 0);
  if (sum > total) out.push(`README media total ${mb(sum)}, over ${mb(total)}`);
  return out;
}

/** Sizes of the media files under ROOT's docs/screenshots and docs/gifs. */
export function scan(root = "."): Record<string, number> {
  const out: Record<string, number> = {};
  for (const d of DIRS) {
    let names: string[] = [];
    try { names = readdirSync(join(root, d)); } catch { continue; }
    for (const n of names) if (/\.(png|gif)$/.test(n)) out[`${d}/${n}`] = statSync(join(root, d, n)).size;
  }
  return out;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const files = scan();
  for (const [f, n] of Object.entries(files)) console.log(`${mb(n).padStart(9)}  ${f}`);
  const bad = problems(files);
  for (const p of bad) console.error(p);
  process.exit(bad.length ? 1 : 0);
}
