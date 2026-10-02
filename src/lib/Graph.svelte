<script lang="ts" module>
  export type NoteGraph = { nodes: { path: string; title: string; hops: number }[]; edges: [number, number][]; hidden: number };
</script>

<script lang="ts">
  // Local note graph: the current note (centre) and everything within two links of it.
  import { layout } from "$lib/graph";

  let { graph, open }: { graph: NoteGraph; open: (path: string) => void } = $props();

  const pts = $derived(layout(graph.nodes.length, graph.edges));
  const box = $derived.by(() => {
    const xs = pts.map((p) => p.x), ys = pts.map((p) => p.y), pad = 50;
    const x = Math.min(...xs) - pad, y = Math.min(...ys) - pad;
    return `${x} ${y} ${Math.max(...xs) + pad - x} ${Math.max(...ys) + pad - y}`;
  });
</script>

<svg viewBox={box} role="list">
  {#each graph.edges as [a, b]}
    <line x1={pts[a].x} y1={pts[a].y} x2={pts[b].x} y2={pts[b].y} />
  {/each}
  {#each graph.nodes as n, i (n.path)}
    <g role="listitem" class="h{n.hops}" transform="translate({pts[i].x} {pts[i].y})">
      <a href="#{n.path}" onclick={(e) => { e.preventDefault(); if (i) open(n.path); }}>
        <title>{n.title}</title>
        <circle r={i ? 6 : 9} />
        <text y="20">{n.title.length > 24 ? n.title.slice(0, 23) + "…" : n.title}</text>
      </a>
    </g>
  {/each}
</svg>
{#if graph.hidden}<p class="more">+{graph.hidden} more notes not shown</p>{/if}

<style>
  svg { width: 100%; height: min(60vh, 420px); display: block; font: var(--fs-xs) var(--sans); }
  line { stroke: var(--border); stroke-width: 1.5; }
  circle { fill: var(--link); stroke: var(--panel); stroke-width: 2; }
  text { fill: var(--fg); text-anchor: middle; }
  .h0 circle { fill: var(--accent); }
  .h2 circle { fill: var(--dim); }
  .more { margin: 0; text-align: center; color: var(--dim); font: var(--fs-xs) var(--sans); }
  .h2 text { fill: var(--dim); }
  a { cursor: pointer; }
  a:hover circle { stroke: var(--fg); }
  a:hover text { text-decoration: underline; }
</style>
