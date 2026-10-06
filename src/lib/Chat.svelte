<script lang="ts" module>
  export type ChatMsg = { role: "user" | "assistant"; content: string };
</script>

<script lang="ts">
  import { tick } from "svelte";

  /** Chat with the local model (#205). History lives here, in memory only; ASK sends it, with the open NOTE (its name) when ticked. */
  let { ask, note }: { ask: (msgs: ChatMsg[], withNote: boolean) => Promise<string>; note: string | null } = $props();
  let msgs = $state<ChatMsg[]>([]);
  let text = $state("");
  let withNote = $state(true);
  let busy = $state(false);
  let error = $state("");
  let list = $state<HTMLElement>();
  let input = $state<HTMLTextAreaElement>();

  const scroll = async () => { await tick(); list?.scrollTo({ top: list.scrollHeight }); };

  export const focus = () => input?.focus();

  async function send() {
    const q = text.trim();
    if (!q || busy) return;
    msgs.push({ role: "user", content: q });
    [text, busy, error] = ["", true, ""];
    await scroll();
    try {
      msgs.push({ role: "assistant", content: await ask($state.snapshot(msgs), withNote && !!note) });
    } catch (e) {
      msgs.pop(); // unanswered: back into the box, so the history stays question, answer, …
      [text, error] = [q, String(e)];
    } finally {
      busy = false;
      await scroll();
    }
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); send(); }
  }
</script>

<div class="chat">
  <h3>AI chat <button disabled={busy || !msgs.length} onclick={() => { msgs = []; error = ""; }}>Clear</button></h3>
  <div class="msgs" bind:this={list}>
    {#each msgs as m}
      <p class={m.role}>{m.content}</p>
    {:else}
      <p class="dim">Ask the local model anything{note ? ", say about the open note" : ""}. Nothing leaves this machine, and the chat isn't saved.</p>
    {/each}
    {#if busy}<p class="dim">Thinking…</p>{/if}
    {#if error}<p class="err">⚠ {error}</p>{/if}
  </div>
  {#if note}<label><input type="checkbox" bind:checked={withNote} /> Include {note}</label>{/if}
  <textarea bind:this={input} bind:value={text} {onkeydown} rows="3" placeholder="Message (Enter sends, Shift+Enter new line)"></textarea>
</div>

<style>
  .chat { display: flex; flex-direction: column; height: 100%; gap: var(--s2); }
  h3 { display: flex; align-items: center; justify-content: space-between; margin: var(--s1) 6px 0; font-size: var(--fs-sm); color: var(--dim); text-transform: uppercase; letter-spacing: 0.05em; }
  h3 button { background: none; border: 0; color: var(--link); font-size: var(--fs-sm); cursor: pointer; text-transform: none; }
  h3 button:disabled { color: var(--dim); cursor: default; }
  .msgs { flex: 1; overflow-y: auto; min-height: 0; }
  p { margin: 0 0 var(--s2); padding: 6px var(--s2); border-radius: var(--radius); white-space: pre-wrap; overflow-wrap: anywhere; font-size: var(--fs-md); user-select: text; }
  .user { background: var(--sel); margin-left: var(--s5); }
  .assistant { background: var(--active); margin-right: var(--s5); }
  .dim { color: var(--dim); font-size: var(--fs-sm); }
  .err { color: var(--todo); font-size: var(--fs-sm); }
  label { color: var(--dim); font-size: var(--fs-sm); display: flex; gap: var(--s1); align-items: center; overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
  textarea { resize: none; background: var(--bg); color: var(--fg); border: 1px solid var(--border); border-radius: var(--radius); padding: 6px var(--s2); font: var(--fs-md) var(--sans); }
  textarea:focus { outline: none; border-color: var(--accent); }
</style>
