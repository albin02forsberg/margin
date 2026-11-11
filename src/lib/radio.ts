// Org radio targets (#1265): `<<<Term>>>` makes every plain, whole-word mention of
// "Term" in the same note (any case) a link to it. The file is never rewritten.

/** A radio target: no whitespace or angle brackets at either end. */
const TARGET = /<<<([^<>\s](?:[^<>\n]*[^<>\s])?)>>>/g;
/** What a mention can't sit inside: links and (radio) targets. */
const SKIP = /\[\[[^\]]+\](?:\[[^\]]+\])?\]|<<<?[^<>\n]+>>>?/g;
const MAX = 200;

/** The radio targets of one note: a combined mention regex, the original spelling by normalised term and
 *  the 0-based lines inside src/example blocks (no mentions there). */
export type Radio = { re: RegExp; terms: Map<string, string>; code: Set<number> } | null;

const key = (s: string) => s.replace(/\s+/g, " ").toLowerCase();
const esc = (s: string) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");

/** The radio targets in TEXT outside src/example blocks (first spelling of a term wins, at most 200), or null. */
export function radio(text: string): Radio {
  if (!text.includes("<<<")) return null;
  const terms = new Map<string, string>(), code = new Set<number>();
  let inCode = false;
  for (const [i, line] of text.split("\n").entries()) {
    const t = line.trimStart().toLowerCase();
    if (inCode || /^#\+begin_(src|example)\b/.test(t)) {
      code.add(i);
      inCode = !(inCode && /^#\+end_(src|example)\b/.test(t));
      continue;
    }
    for (const m of line.matchAll(TARGET)) if (terms.size < MAX && !terms.has(key(m[1]))) terms.set(key(m[1]), m[1]);
  }
  if (!terms.size) return null;
  const alts = [...terms.values()].sort((a, b) => b.length - a.length).map((t) => esc(t).replace(/\s+/g, "\\s+"));
  return { re: new RegExp(`(?<![\\p{L}\\p{N}_])(?:${alts.join("|")})(?![\\p{L}\\p{N}_])`, "giu"), terms, code };
}

/** Mentions of R's targets in LINE as 0-based offsets and the target's original spelling, outside links and targets. */
export function mentions(line: string, r: Radio): { from: number; to: number; target: string }[] {
  if (!r) return [];
  const skip = [...line.matchAll(SKIP)].map((m) => [m.index!, m.index! + m[0].length]);
  const out = [];
  for (const m of line.matchAll(r.re)) {
    const from = m.index!, to = from + m[0].length;
    const target = r.terms.get(key(m[0]));
    if (target && !skip.some(([a, b]) => from < b && to > a)) out.push({ from, to, target });
  }
  return out;
}
