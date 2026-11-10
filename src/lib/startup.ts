// `#+STARTUP:` initial folding (org interop): overview / content / showall / showeverything / show<N>levels (N as a number),
// then each heading's `:VISIBILITY:` property (except under showeverything). Text-only so node tests can import it.
export type StartupMode = "overview" | "content" | "showall" | "showeverything" | number;

const lvl = (t: string) => /^(\*+) /.exec(t)?.[1].length ?? 0;
const isHead = (t: string) => /^\*+ /.test(t);

/** Property KEY in the drawer right under heading line N (1-based), after at most one planning line. */
export function headProp(lines: string[], n: number, key: string): string | undefined {
  const re = new RegExp(`^\\s*:${key}:\\s*(\\S+)\\s*$`, "i");
  for (let j = n; j < Math.min(n + 2, lines.length) && !isHead(lines[j]); j++) {
    if (!/^\s*:PROPERTIES:\s*$/i.test(lines[j])) continue;
    for (let k = j + 1; k < lines.length && !/^\s*:END:\s*$/i.test(lines[k]) && !isHead(lines[k]); k++) {
      const m = re.exec(lines[k]);
      if (m) return m[1];
    }
    return;
  }
}

/** Mode from the first `#+STARTUP:` keyword in the leading `#+` block; unknown tokens mean showall. */
export function startupMode(doc: string): StartupMode {
  for (const l of doc.split("\n")) {
    const m = /^#\+startup:\s*(.*)$/i.exec(l);
    if (m) {
      for (const tok of m[1].toLowerCase().split(/\s+/)) {
        if (tok === "overview" || tok === "content" || tok === "showall" || tok === "showeverything") return tok;
        if (tok === "fold") return "overview";
        if (tok === "nofold") return "showall";
        const n = /^show([2-5])levels$/.exec(tok);
        if (n) return +n[1];
      }
      return "showall";
    }
    if (l.trim() && !l.startsWith("#+")) break;
  }
  return "showall";
}

const VIS = ["folded", "children", "content", "all", "showall"];

/** 1-based inclusive line ranges to fold: the end of `from` through the end of `to`. */
export function startupFolds(lines: string[], mode: StartupMode): { from: number; to: number }[] {
  if (mode === "showeverything") return [];
  const hs = lines.map((t, i) => [i + 1, lvl(t)] as const).filter(([, l]) => l);
  // Index of the last heading in heading K's subtree (SUB) or K itself, and the last line that covers.
  const last = (k: number, sub = true) => { let j = k; while (sub && j + 1 < hs.length && hs[j + 1][1] > hs[k][1]) j++; return j; };
  const end = (k: number, sub = true) => (hs[last(k, sub) + 1]?.[0] ?? lines.length + 1) - 1;
  let out: { from: number; to: number }[] = [];
  const add = (k: number, sub = true) => { const to = end(k, sub); if (to > hs[k][0]) out.push({ from: hs[k][0], to }); };
  if (mode !== "showall") {
    const top = Math.min(...hs.map(([, l]) => l));
    // show<N>levels: headings above level N fold their body only, level N folds its whole subtree (overview = 1).
    const depth = mode === "overview" ? 1 : typeof mode === "number" ? mode : Infinity;
    hs.forEach(([, l], k) => { const rel = l - top + 1; if (rel <= depth) add(k, rel === depth); });
  }
  // `:VISIBILITY:` replaces the folds inside its subtree (ancestors stay as they are); nested ones inside it are skipped, like org.
  for (let k = 0; k < hs.length; k++) {
    const v = headProp(lines, hs[k][0], "VISIBILITY")?.toLowerCase();
    if (!v || !VIS.includes(v)) continue;
    const n = hs[k][0], e = end(k), z = last(k);
    out = out.filter((f) => f.from < n || f.from > e);
    if (v === "folded") add(k);
    else if (v === "children") for (let j = k + 1; j <= z; j = last(j) + 1) add(j);
    else if (v === "content") for (let j = k; j <= z; j++) add(j, false);
    k = z;
  }
  return out.sort((a, b) => a.from - b.from);
}
