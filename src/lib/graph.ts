// Force layout for the note graph (Fruchterman–Reingold, no dependency).

export type Pt = { x: number; y: number };

/** Positions for N nodes joined by EDGES, node 0 pinned at the origin. Deterministic: same input, same picture. */
// ponytail: O(n²) repulsion per step; the backend caps graphs at 150 nodes, use a quadtree to lift that.
export function layout(n: number, edges: [number, number][], steps = 300, k = 60): Pt[] {
  // Start on a golden-angle spiral so nothing overlaps and reruns don't jump around.
  const p: Pt[] = Array.from({ length: n }, (_, i) => ({ x: k * Math.sqrt(i) * Math.cos(i * 2.4), y: k * Math.sqrt(i) * Math.sin(i * 2.4) }));
  for (let s = 0, t = k; s < steps; s++, t *= 0.98) {
    const d: Pt[] = p.map(() => ({ x: 0, y: 0 }));
    const push = (i: number, j: number, f: (dist: number) => number) => {
      const dx = p[i].x - p[j].x, dy = p[i].y - p[j].y, dist = Math.hypot(dx, dy) || 0.01, m = f(dist) / dist;
      d[i].x += dx * m; d[i].y += dy * m; d[j].x -= dx * m; d[j].y -= dy * m;
    };
    for (let i = 0; i < n; i++) for (let j = i + 1; j < n; j++) push(i, j, (r) => (k * k) / r);
    for (const [i, j] of edges) push(i, j, (r) => -(r * r) / k);
    for (let i = 1; i < n; i++) {
      const len = Math.hypot(d[i].x, d[i].y) || 1, m = Math.min(len, t) / len;
      p[i].x += d[i].x * m - p[i].x * 0.01; // slight pull to the centre keeps islands close
      p[i].y += d[i].y * m - p[i].y * 0.01;
    }
  }
  return p;
}
