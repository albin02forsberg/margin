// `#+STARTUP:` initial folding (org interop): overview / content / showall. Text-only so node tests can import it.
export type StartupMode = "overview" | "content" | "showall";

const lvl = (t: string) => /^(\*+) /.exec(t)?.[1].length ?? 0;

/** Mode from the first `#+STARTUP:` keyword in the leading `#+` block; unknown tokens mean showall. */
export function startupMode(doc: string): StartupMode {
  for (const l of doc.split("\n")) {
    const m = /^#\+startup:\s*(.*)$/i.exec(l);
    if (m) {
      for (const tok of m[1].toLowerCase().split(/\s+/)) {
        if (tok === "overview" || tok === "content") return tok;
        if (tok === "showall" || tok === "showeverything") return "showall";
      }
      return "showall";
    }
    if (l.trim() && !l.startsWith("#+")) break;
  }
  return "showall";
}

/** 1-based inclusive line ranges to fold: the end of `from` through the end of `to`. */
export function startupFolds(lines: string[], mode: StartupMode): { from: number; to: number }[] {
  const out: { from: number; to: number }[] = [];
  if (mode === "showall") return out;
  const hs = lines.map((t, i) => [i + 1, lvl(t)] as const).filter(([, l]) => l);
  const top = Math.min(...hs.map(([, l]) => l));
  hs.forEach(([n, l], k) => {
    let end = lines.length;
    for (let j = k + 1; j < hs.length; j++) {
      if (mode === "content" || hs[j][1] <= l) { end = hs[j][0] - 1; break; }
    }
    if ((mode === "overview" ? l === top : true) && end > n) out.push({ from: n, to: end });
  });
  return out;
}
