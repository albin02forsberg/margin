<script lang="ts" module>
  export type FormulaTable = { lines: string[]; fm: string; row: number; col: number };
</script>

<script lang="ts">
  // "Table formulas": list, edit and delete the table's #+TBLFM formulas and add new ones
  // by target + function + expression, with a live preview and the error inline.
  import { tick } from "svelte";
  import * as T from "./orgtable";

  type Kind = T.Target["kind"] | "other";
  let active = $state(false);
  let table = $state<FormulaTable>({ lines: [], fm: "", row: 1, col: 1 });
  let fm = $state("");
  let kind = $state<Kind>("col");
  let col = $state(1), row = $state(1);
  let lhsText = $state("");
  let fn = $state("");
  let expr = $state("");
  let fmt = $state("");
  let editing = $state<string | null>(null);
  let prefill = "";
  let exprEl = $state<HTMLInputElement>();
  let prevFocus: HTMLElement | null = null;
  let closed: (fm: string | null) => void = () => {};

  export const isOpen = () => active;

  /** Resolves with the table's new TBLFM ("" for none), or null when cancelled. */
  export async function open(t: FormulaTable): Promise<string | null> {
    prevFocus = document.activeElement as HTMLElement | null;
    table = t;
    fm = t.fm;
    reset();
    active = true;
    await tick();
    exprEl?.focus();
    return new Promise((r) => (closed = r));
  }

  function close(out: string | null) {
    active = false;
    prevFocus?.focus();
    closed(out);
  }

  function reset() {
    editing = null;
    kind = "col"; col = table.col; row = table.row; lhsText = ""; fn = ""; expr = ""; fmt = ""; prefill = "";
  }

  const hasSep = $derived(table.lines.some(T.isSep));
  const isRange = (f: string) => T.FUNCTIONS.some(([n, , r]) => n === f && r);
  /** A starting expression for FN: the rows above (a cell) or the columns left (a column). */
  function suggest(f: string): string {
    const left = Math.max(1, col - 1);
    if (!f) return "";
    if (isRange(f)) return kind === "col" ? `$1..$${left}` : `@${hasSep ? "I" : "<"}..@-1`;
    return f === "round" ? `$${left}, 2` : `$${left}`;
  }
  function pickFn() {
    if (!expr.trim() || expr === prefill) expr = prefill = suggest(fn);
    exprEl?.focus();
  }

  const lhs = $derived(kind === "other" ? lhsText.trim() : T.buildFormula({ kind, col, row }, "", "").lhs);
  const rhs = $derived(T.buildFormula({ kind: "col", col: 1 }, fn, expr).rhs + (fmt.trim() ? `;${fmt.trim()}` : ""));
  const draft = $derived(expr.trim() ? `${lhs}=${rhs}` : "");
  const error = $derived(draft ? T.formulaError(table.lines, lhs, rhs) : null);
  /** FM with the formula being written in place of the one being edited. */
  const next = $derived(draft ? T.mergeTblfm(editing ? T.removeTblfm(fm, editing) : fm, [draft]) : fm);
  const preview = $derived(T.align(next ? T.applyFormulas(table.lines, next) : table.lines));
  const list = $derived(T.parseTblfm(fm).map((f) => ({ ...f, error: T.formulaError(table.lines, f.lhs, f.rhs) })));

  /** Load an existing formula into the form. */
  function edit(f: { lhs: string; rhs: string }) {
    editing = f.lhs;
    const c = /^\$(\d+)$/.exec(f.lhs), last = /^@>\$(\d+)$/.exec(f.lhs), cell = /^@(\d+)\$(\d+)$/.exec(f.lhs);
    if (c) { kind = "col"; col = +c[1]; }
    else if (last) { kind = "last"; col = +last[1]; }
    else if (cell) { kind = "cell"; row = +cell[1]; col = +cell[2]; }
    else { kind = "other"; lhsText = f.lhs; }
    const semi = f.rhs.indexOf(";");
    const e = semi < 0 ? f.rhs : f.rhs.slice(0, semi);
    fmt = semi < 0 ? "" : f.rhs.slice(semi + 1);
    const m = /^(\w+)\((.*)\)$/.exec(e);
    // Only a call that wraps the whole expression, e.g. not vsum($1)+vsum($2).
    let depth = 0;
    const whole = m && T.FUNCTIONS.some(([n]) => n === m[1]) && [...m[2]].every((ch) => (depth += ch === "(" ? 1 : ch === ")" ? -1 : 0) >= 0);
    fn = whole ? m[1] : "";
    expr = whole ? m[2] : e;
    prefill = "";
    exprEl?.focus();
  }

  function commit() {
    if (!draft || error) return;
    fm = next;
    reset();
    exprEl?.focus();
  }

  function key(e: KeyboardEvent) {
    e.stopPropagation();
    if (e.key === "Escape") close(null);
    else if (e.key === "Enter" && !(e.target instanceof HTMLButtonElement) && !(e.target instanceof HTMLSelectElement)) {
      if (draft) commit();
      else close(fm);
    } else return;
    e.preventDefault();
  }
</script>

{#if active}
  <div class="backdrop" onmousedown={() => close(null)} role="presentation"></div>
  <div class="dialog" onkeydown={key} role="dialog" aria-label="Table formulas" tabindex="-1">
    <h2>Table formulas</h2>
    {#if list.length}
      <ul class="list">
        {#each list as f (f.lhs)}
          <li class:editing={editing === f.lhs}>
            <code>{f.lhs}={f.rhs}</code>
            {#if f.error}<span class="err">{f.error}</span>{/if}
            <button type="button" onclick={() => edit(f)}>Edit</button>
            <button type="button" aria-label="Delete {f.lhs}" onclick={() => { fm = T.removeTblfm(fm, f.lhs); if (editing === f.lhs) reset(); }}>Delete</button>
          </li>
        {/each}
      </ul>
    {:else}
      <p class="none">No formulas yet. Add one below, or type <code>=expr</code> in a cell.</p>
    {/if}

    <div class="form">
      <label>Target
        <select name="target" bind:value={kind}>
          <option value="col">Column (every row)</option>
          <option value="cell">One cell</option>
          <option value="last">Last row (a total)</option>
          <option value="other">Other (type it)</option>
        </select>
      </label>
      {#if kind === "other"}
        <label>Field <input class="lhs" placeholder="@<$2, @II$3" bind:value={lhsText} /></label>
      {:else}
        {#if kind === "cell"}<label>Row <input name="row" type="number" min="1" bind:value={row} /></label>{/if}
        <label>Column <input name="column" type="number" min="1" bind:value={col} /></label>
      {/if}
      <label>Function
        <select name="fn" bind:value={fn} onchange={(e) => { fn = e.currentTarget.value; pickFn(); }}>
          <option value="">none (expression)</option>
          {#each T.FUNCTIONS as [name, what]}<option value={name}>{name} — {what}</option>{/each}
        </select>
      </label>
      <label class="wide">Expression
        <span class="expr">{#if fn}<code>{fn}(</code>{/if}<input name="expr" bind:this={exprEl} bind:value={expr} placeholder={fn ? "@I..@-1" : "$2*$3"} spellcheck="false" />{#if fn}<code>)</code>{/if}</span>
      </label>
      <label>Format <input class="fmt" placeholder="%.2f" bind:value={fmt} spellcheck="false" /></label>
      <button type="button" class="add" disabled={!draft || !!error} onclick={commit}>{editing ? "Update" : "Add"}</button>
    </div>
    <p class="hint"><code>$3</code> column 3 · <code>@2</code> row 2 (separators don't count) · <code>@2$3</code> one field · <code>@&gt;</code> / <code>$&gt;</code> last · <code>@-1</code> / <code>$+1</code> relative · <code>@I</code> first separator · <code>@2..@-1</code> range · <code>+ - * / ^ ( )</code></p>
    {#if draft}<p class="status" class:err={!!error}><code>{draft}</code> {error ? `— ${error}` : "— ok"}</p>{/if}
    <pre class="preview" aria-label="Preview">{preview.join("\n")}</pre>

    <footer>
      <span>↵ add · ↵ on an empty expression saves · Esc cancel</span>
      <button type="button" onclick={() => close(null)}>Cancel</button>
      <button type="button" class="primary" disabled={!!error} onclick={() => close(next)}>Save &amp; recalculate</button>
    </footer>
  </div>
{/if}

<style>
  .backdrop { position: fixed; inset: 0; z-index: 499; background: var(--scrim); }
  .dialog {
    position: fixed; top: 6%; left: 50%; transform: translateX(-50%); z-index: 500; max-height: 88vh; overflow: auto;
    width: min(640px, calc(100vw - 32px)); background: var(--card); border: 1px solid var(--border);
    border-radius: var(--r); box-shadow: var(--pshadow); padding: 18px 20px; outline: none; box-sizing: border-box;
  }
  h2 { margin: 0 0 var(--s3); font-size: var(--fs-md); color: var(--dim); font-weight: 600; text-transform: uppercase; letter-spacing: 0.05em; }
  code, pre, .lhs, .fmt, .expr input { font-family: var(--mono); }
  .list { list-style: none; margin: 0 0 var(--s3); padding: 0; display: flex; flex-direction: column; gap: 4px; }
  .list li { display: flex; gap: var(--s2); align-items: center; padding: 3px 6px; border-radius: var(--radius-sm); }
  .list li.editing { background: color-mix(in srgb, var(--accent) 15%, transparent); }
  .list code { flex: 1; overflow-wrap: anywhere; }
  .none, .hint { color: var(--dim); font-size: var(--fs-xs); margin: 0 0 var(--s3); }
  .err { color: var(--todo); font-size: var(--fs-xs); }
  .form { display: flex; flex-wrap: wrap; gap: var(--s2) var(--s3); align-items: flex-end; margin-bottom: var(--s2); }
  label { color: var(--dim); font-size: var(--fs-sm); display: flex; flex-direction: column; gap: 3px; }
  label.wide { flex: 1; min-width: 200px; }
  .expr { display: flex; align-items: center; gap: 2px; color: var(--fg); }
  .expr input { flex: 1; min-width: 0; }
  input, select { background: var(--bg); border: 1px solid var(--border); border-radius: var(--radius); color: var(--fg); font-size: var(--fs-base); padding: 5px 7px; box-sizing: border-box; }
  input[type="number"] { width: 64px; }
  .fmt { width: 72px; }
  input:focus, select:focus { outline: 2px solid var(--accent); outline-offset: -1px; }
  .status { margin: 0 0 var(--s2); font-size: var(--fs-sm); color: var(--dim); }
  .status.err code { color: inherit; }
  .preview { margin: 0; padding: 8px 10px; background: var(--bg); border: 1px solid var(--border); border-radius: var(--radius); font-size: var(--fs-sm); overflow: auto; max-height: 30vh; }
  button { background: var(--bg); border: 1px solid var(--border); color: var(--fg); border-radius: var(--radius); padding: 5px 12px; font: var(--fs-sm) var(--sans); cursor: pointer; }
  button:disabled { opacity: 0.5; cursor: default; }
  .list button { padding: 1px 8px; }
  footer { display: flex; gap: var(--s2); align-items: center; margin-top: 18px; }
  footer span { flex: 1; color: var(--dim); font-size: var(--fs-xs); }
  footer button { padding: 7px 14px; font-size: var(--fs-md); }
  footer .primary { background: var(--accent); border-color: var(--accent); color: var(--bg); font-weight: 600; }
</style>
