<script lang="ts" module>
  /** An excerpt the answer drew on ("Use my notes", #304): of a note (LINE is 0-based), or of the open tasks, agenda or time log (#468). */
  export type Source = { kind: "note" | "tasks" | "agenda" | "time"; path: string; title: string; line: number; text: string };
  const KINDS = { tasks: "Tasks", agenda: "Agenda", time: "Time log" };
  /** SOURCES: what an answer was given, [] when no note matched; TOOLS: a line per tool call the model made for it (#963). The backend ignores both. */
  export type ChatMsg = { role: "user" | "assistant"; content: string; sources?: Source[]; tools?: string[] };
</script>

<script lang="ts">
  import { tick } from "svelte";
  import { cleanChat, toolLine, type ToolEvent } from "./chatctx";

  /** Chat with the local model (#205). History lives here, in memory only; ASK sends it, with the open NOTE (its name) when ticked and the matching notes when USE_NOTES; it returns the answer and the notes it was given. INSERT (when a note is open) puts an answer at the cursor; OPEN shows a source. EXCLUDED: the open note is in ai_exclude, so it isn't offered. */
  let { ask, note, noteLabel, insert, open, excluded = false }: { ask: (msgs: ChatMsg[], withNote: boolean, useNotes: boolean, onPiece: (piece: string) => void, onTool: (e: ToolEvent) => void) => Promise<{ answer: string; sources?: Source[] }>; note: string | null; noteLabel?: () => string; insert?: (text: string) => void; open: (path: string, line: number) => void; excluded?: boolean } = $props();
  let msgs = $state<ChatMsg[]>([]);
  let text = $state("");
  let withNote = $state(true);
  let useNotes = $state(false);
  let busy = $state(false);
  /** The answer so far, while it streams in (#210). */
  let partial = $state("");
  /** Its tool calls so far, as lines (#963). */
  let calls = $state<string[]>([]);
  const shown = $derived(cleanChat(partial));
  let error = $state("");
  let list = $state<HTMLElement>();
  let input = $state<HTMLTextAreaElement>();
  /** What the checkbox says is sent: the selection, a section or the note; refreshed on focus and tab change, as the cursor isn't reactive. */
  let label = $state("");
  const refresh = () => (label = note && noteLabel ? noteLabel() : (note ?? ""));
  $effect(() => { refresh(); });

  const scroll = async () => { await tick(); list?.scrollTo({ top: list.scrollHeight }); };

  export const focus = () => input?.focus();

  async function send() {
    const q = text.trim();
    if (!q || busy) return;
    msgs.push({ role: "user", content: q });
    [text, busy, error, partial, calls] = ["", true, "", "", []];
    await scroll();
    try {
      const { answer, sources } = await ask($state.snapshot(msgs), withNote && !!note && !excluded, useNotes, (p) => { partial += p; scroll(); }, (e) => { calls.push(toolLine(e)); scroll(); });
      msgs.push({ role: "assistant", content: answer, sources, tools: calls.length ? [...calls] : undefined });
    } catch (e) {
      // Cancelled or failed midway: keep what came. Unanswered: back into the box, so the history stays question, answer, …
      const kept = cleanChat(partial);
      if (kept) msgs.push({ role: "assistant", content: kept, tools: calls.length ? [...calls] : undefined });
      else { msgs.pop(); text = q; }
      error = String(e);
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
      {#each m.tools ?? [] as t}<div class="tool" title={t}>{t}</div>{/each}
      <p class={m.role}>{m.content}</p>
      {#if m.sources}
        <div class="srcs">
          {#each m.sources as s}{#if s.kind === "note"}<button title={s.text} onclick={() => open(s.path, s.line)}>{s.title}:{s.line + 1}</button>{:else}<span title={s.text}>{KINDS[s.kind as keyof typeof KINDS] ?? s.kind}</span>{/if}{:else}No note matched.{/each}
        </div>
      {/if}
      {#if m.role === "assistant"}
        <div class="acts">
          <button disabled={!insert} onclick={() => insert?.(m.content)}>Insert</button>
          <button onclick={() => navigator.clipboard.writeText(m.content).catch((e) => (error = String(e)))}>Copy</button>
        </div>
      {/if}
    {:else}
      <p class="dim">Ask the local model anything{note ? ", say about the open note" : ""}. Nothing leaves this machine, and the chat isn't saved.</p>
    {/each}
    {#if busy}{#each calls as t}<div class="tool" title={t}>{t}</div>{/each}<p class={shown ? "assistant" : "dim"}>{shown || "Thinking…"}</p>{/if}
    {#if error}<p class="err">⚠ {error}</p>{/if}
  </div>
  {#if note && excluded}<p class="dim">{note} is excluded from AI (AI chat settings).</p>{:else if note}<label><input type="checkbox" bind:checked={withNote} /> Include {label || note}</label>{/if}
  <label title="Send excerpts of the notes that match the question and of the notes linked to them, and your tasks, agenda or time log when the question asks about them"><input type="checkbox" bind:checked={useNotes} /> Use my notes</label>
  <textarea bind:this={input} bind:value={text} {onkeydown} onfocus={refresh} rows="3" placeholder="Message (Enter sends, Shift+Enter new line)"></textarea>
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
  .acts { display: flex; gap: var(--s2); margin: calc(-1 * var(--s1)) var(--s5) var(--s2) 0; justify-content: flex-end; }
  .acts button { background: none; border: 0; color: var(--link); font-size: var(--fs-sm); cursor: pointer; padding: 0; }
  .acts button:disabled { color: var(--dim); cursor: default; }
  .srcs { display: flex; flex-wrap: wrap; gap: var(--s1) var(--s2); margin: calc(-1 * var(--s1)) var(--s5) var(--s2) 0; color: var(--dim); font-size: var(--fs-sm); }
  .srcs button { background: none; border: 0; color: var(--link); font-size: var(--fs-sm); cursor: pointer; padding: 0; }
  .tool { margin: 0 var(--s5) var(--s1) 0; padding: 0 var(--s2); color: var(--dim); font-size: var(--fs-sm); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .dim { color: var(--dim); font-size: var(--fs-sm); }
  .err { color: var(--todo); font-size: var(--fs-sm); }
  label { color: var(--dim); font-size: var(--fs-sm); display: flex; gap: var(--s1); align-items: center; overflow: hidden; white-space: nowrap; text-overflow: ellipsis; }
  textarea { resize: none; background: var(--canvas); color: var(--fg); border: 1px solid var(--border); border-radius: var(--radius); padding: 6px var(--s2); font: var(--fs-md) var(--sans); }
  textarea:focus { outline: none; border-color: var(--accent); }
</style>
