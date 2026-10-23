<script lang="ts">
  // Flashcard review: the question, Space/Enter reveals the answer, 1-4 grades it; "again" comes back at the end.
  import { tick } from "svelte";
  import type { Card, Grade } from "./drill";

  const labels = ["Again", "Hard", "Good", "Easy"];
  let active = $state(false);
  let queue = $state<Card[]>([]);
  let shown = $state(false);
  let done = $state(0);
  let box = $state<HTMLElement>();
  let prevFocus: HTMLElement | null = null;
  let busy = false; // a grade is being saved
  let onGrade: (c: Card, g: Grade) => Promise<boolean> = async () => true;
  let closed: (n: number) => void = () => {};

  export const isOpen = () => active;

  /** Review CARDS; GRADE saves one (false: skip the card). Resolves with the number graded. */
  export async function open(cards: Card[], grade: (c: Card, g: Grade) => Promise<boolean>): Promise<number> {
    prevFocus = document.activeElement as HTMLElement | null;
    [queue, shown, done, onGrade, active] = [[...cards], false, 0, grade, true];
    await tick();
    box?.focus();
    return new Promise((r) => (closed = r));
  }

  function close() {
    active = false;
    prevFocus?.focus();
    closed(done);
  }

  async function rate(g: Grade) {
    const c = queue[0];
    if (!c || !shown || busy) return;
    busy = true;
    const ok = await onGrade(c, g).finally(() => (busy = false));
    if (ok) done++;
    queue = ok && g === 1 ? [...queue.slice(1), c] : queue.slice(1);
    shown = false;
    if (!queue.length) close();
  }

  function key(e: KeyboardEvent) {
    e.stopPropagation();
    if (e.key === "Escape") close();
    else if ((e.key === " " || e.key === "Enter") && !shown) shown = true;
    else if (shown && /^[1-4]$/.test(e.key)) rate(+e.key as Grade);
    else return;
    e.preventDefault();
  }
</script>

{#if active && queue[0]}
  <div class="backdrop" onmousedown={close} role="presentation"></div>
  <div class="dialog" bind:this={box} onkeydown={key} role="dialog" aria-label="Flashcard review" tabindex="-1">
    <h2>Review · {queue.length} left</h2>
    <div class="q">{queue[0].title}</div>
    {#if shown}
      <div class="a">{queue[0].body}</div>
      <footer>
        {#each labels as l, i}
          <button type="button" onclick={() => rate((i + 1) as Grade)}><kbd>{i + 1}</kbd> {l}</button>
        {/each}
      </footer>
    {:else}
      <footer><span>Space or ↵ shows the answer · Esc stops</span><button type="button" class="primary" onclick={() => (shown = true)}>Show answer</button></footer>
    {/if}
  </div>
{/if}

<style>
  .backdrop { position: fixed; inset: 0; z-index: 499; background: var(--scrim); }
  .dialog {
    position: fixed; top: 12%; left: 50%; transform: translateX(-50%); z-index: 500;
    width: min(560px, calc(100vw - 32px)); background: var(--card); border: 1px solid var(--border);
    border-radius: var(--r); box-shadow: var(--pshadow); padding: 18px 20px; outline: none; box-sizing: border-box;
  }
  h2 { margin: 0 0 var(--s3); font-size: var(--fs-md); color: var(--dim); font-weight: 600; text-transform: uppercase; letter-spacing: 0.05em; }
  .q { font-size: var(--fs-lg, 1.2em); font-weight: 600; }
  .a { margin-top: var(--s3); padding-top: var(--s3); border-top: 1px solid var(--border); white-space: pre-wrap; font-family: var(--mono); max-height: 50vh; overflow: auto; }
  footer { display: flex; gap: var(--s2); align-items: center; margin-top: 18px; }
  footer span { flex: 1; color: var(--dim); font-size: var(--fs-xs); }
  footer button { background: var(--bg); border: 1px solid var(--border); color: var(--fg); border-radius: var(--radius); padding: 7px 14px; font: var(--fs-md) var(--sans); cursor: pointer; }
  footer .primary { background: var(--accent); border-color: var(--accent); color: var(--bg); font-weight: 600; }
  kbd { color: var(--dim); font: var(--fs-xs) var(--mono); }
</style>
