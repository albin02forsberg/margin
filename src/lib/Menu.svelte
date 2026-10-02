<script lang="ts" module>
  export type MenuNode = { key: string; label: string; run?: () => void; children?: MenuNode[] };
</script>

<script lang="ts">
  // Which-key style leader menu (replaces the C-c t transient).
  let { root }: { root: MenuNode[] } = $props();
  let path = $state<MenuNode[]>([]);
  let active = $state(false);

  const node = $derived(path.length ? path[path.length - 1].children! : root);

  export const isOpen = () => active;
  export function show() {
    path = [];
    active = true;
  }

  function key(e: KeyboardEvent) {
    if (!active) return;
    e.preventDefault();
    e.stopPropagation();
    if (["Shift", "Control", "Alt", "Meta"].includes(e.key)) return;
    if (e.key === "Escape" || (e.ctrlKey && e.key === "g")) return void (active = false);
    if (e.key === "Backspace") return void (path.length ? path.pop() : (active = false));
    const k = e.key === " " ? "SPC" : e.key;
    const hit = node.find((n) => n.key === k);
    if (!hit) return void (active = false);
    if (hit.children) path.push(hit);
    else {
      active = false;
      hit.run?.();
    }
  }

  $effect(() => {
    if (!active) return;
    window.addEventListener("keydown", key, true);
    return () => window.removeEventListener("keydown", key, true);
  });
</script>

{#if active}
  <div class="menu">
    <div class="crumb">SPC {path.map((p) => p.key).join(" ")} <span>{path.at(-1)?.label ?? ""}</span></div>
    <div class="grid">
      {#each node as n}
        <div><kbd>{n.key}</kbd> <span class:group={n.children}>{n.children ? "+" : ""}{n.label}</span></div>
      {/each}
    </div>
  </div>
{/if}

<style>
  .menu {
    position: fixed; left: 0; right: 0; bottom: 0; z-index: 400; background: var(--panel);
    border-top: 1px solid var(--border); padding: var(--s2) var(--s4) var(--s3); font: var(--fs-md) var(--mono);
  }
  .crumb { color: var(--accent); margin-bottom: 6px; }
  .crumb span { color: var(--dim); margin-left: var(--s2); }
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(220px, 1fr)); gap: 3px var(--s4); }
  kbd { color: var(--todo); font: inherit; display: inline-block; min-width: 2.5ch; }
  .group { color: var(--link); }
</style>
