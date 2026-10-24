/** Line diff from A to B as `- ` / `+ ` / `  ` lines, CONTEXT unchanged lines around each change and `…` between hunks; "" when equal.
 *  LCS on what's left after the common prefix and suffix; past ~4M cells the middle shows as all removed, then all added. */
export function lineDiff(a: string, b: string, context = 2): string {
  if (a === b) return "";
  const x = a.split("\n"), y = b.split("\n");
  let p = 0, s = 0;
  while (p < x.length && p < y.length && x[p] === y[p]) p++;
  while (s < x.length - p && s < y.length - p && x[x.length - 1 - s] === y[y.length - 1 - s]) s++;
  const A = x.slice(p, x.length - s), B = y.slice(p, y.length - s), n = A.length, m = B.length;
  const ops: [string, string][] = x.slice(0, p).map((l) => [" ", l]);
  if (n * m > 4_000_000) ops.push(...A.map((l): [string, string] => ["-", l]), ...B.map((l): [string, string] => ["+", l]));
  else {
    const L = new Uint32Array((n + 1) * (m + 1)), w = m + 1; // L[i*w+j]: LCS of A[i..], B[j..]
    for (let i = n - 1; i >= 0; i--) for (let j = m - 1; j >= 0; j--) L[i * w + j] = A[i] === B[j] ? L[(i + 1) * w + j + 1] + 1 : Math.max(L[(i + 1) * w + j], L[i * w + j + 1]);
    let i = 0, j = 0;
    while (i < n || j < m) {
      if (i < n && j < m && A[i] === B[j]) ops.push([" ", A[i++]]), j++;
      else if (i < n && (j == m || L[(i + 1) * w + j] >= L[i * w + j + 1])) ops.push(["-", A[i++]]);
      else ops.push(["+", B[j++]]);
    }
  }
  ops.push(...x.slice(x.length - s).map((l): [string, string] => [" ", l]));
  const near = ops.map(() => false);
  ops.forEach(([t], k) => { if (t != " ") for (let d = -context; d <= context; d++) if (ops[k + d]) near[k + d] = true; });
  const out: string[] = [];
  ops.forEach(([t, l], k) => {
    if (near[k]) out.push(`${t} ${l}`);
    else if (k > 0 && near[k - 1]) out.push("…");
  });
  if (!near[0]) out.unshift("…");
  return out.join("\n");
}
