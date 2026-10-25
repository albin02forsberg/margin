// `#+TRANSCLUDE: [[link]]` (org-transclusion's keyword, #784): the target heading's body shows read-only and
// dimmed under the line, as a block widget, so line numbers and vim motions are untouched. The file keeps only
// the directive. The backend (`transclude_text`) resolves the link and expands nested transclusions.
import { StateEffect, StateField, type EditorState, type Extension, type Range } from "@codemirror/state";
import { Decoration, EditorView, ViewPlugin, WidgetType, type DecorationSet } from "@codemirror/view";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

const DIRECTIVE = /^\s*#\+transclude:\s*\[\[([^\]\n]+)\](?:\[[^\]]*\])?\]/i;
/** The link target of a `#+TRANSCLUDE:` line, or null. */
export const transcludeTarget = (line: string) => DIRECTIVE.exec(line)?.[1].trim() || null;
/** The directive line for link TARGET. */
export const directive = (target: string) => `#+TRANSCLUDE: [[${target}]]`;

type Res = { path: string; line: number; text: string } | { error: string };
/** Results by `from\0link`, shared by every view (a tab switch shows the last one while it refetches). */
const cache = new Map<string, Res>();
const loaded = StateEffect.define<null>();
/** Live fetchers; each refetches everything on an `fs-changed` event (edits on disk or saved from another tab). */
const live = new Set<() => void>();
let listening = false;

class Block extends WidgetType {
  link: string; res: Res | undefined; follow: (t: string) => void;
  constructor(link: string, res: Res | undefined, follow: (t: string) => void) { super(); this.link = link; this.res = res; this.follow = follow; }
  eq(o: Block) { return o.link === this.link && JSON.stringify(o.res) === JSON.stringify(this.res); }
  toDOM() {
    const d = document.createElement("div"), r = this.res;
    d.className = `cm-transclude${r && "error" in r ? " cm-transclude-missing" : ""}`;
    d.textContent = !r ? "…" : "error" in r ? `⚠ ${r.error}` : r.text || "(empty)";
    d.title = "Click to open the source";
    // An exact jump to the resolved heading (also for id: links); a missing target lets the app say why.
    d.onmousedown = (e) => (e.preventDefault(), this.follow(r && !("error" in r) ? `file:${r.path}::${r.line + 1}` : this.link));
    return d;
  }
  ignoreEvent() { return true; }
}

const targets = (s: EditorState) => {
  const out: { to: number; link: string }[] = [];
  for (let i = 1; i <= s.doc.lines; i++) {
    const l = s.doc.line(i), link = l.text.includes("#+") ? transcludeTarget(l.text) : null;
    if (link) out.push({ to: l.to, link });
  }
  return out;
};

/** Transclusions for the org note at FROM; FOLLOW opens a link target (the app's link following). */
export function transclusion(from: string | undefined, follow: (t: string) => void): Extension {
  if (!from) return [];
  const key = (link: string) => `${from}\0${link}`;
  const build = (s: EditorState): DecorationSet =>
    Decoration.set(targets(s).map(({ to, link }): Range<Decoration> => Decoration.widget({ widget: new Block(link, cache.get(key(link)), follow), block: true, side: 1 }).range(to)));
  const field = StateField.define<DecorationSet>({
    create: build,
    update: (d, tr) => (tr.docChanged || tr.effects.some((e) => e.is(loaded)) ? build(tr.state) : d),
    provide: (f) => EditorView.decorations.from(f),
  });
  const fetcher = ViewPlugin.define((v) => {
    let alive = true;
    const pending = new Set<string>();
    const run = (all: boolean) => {
      for (const { link } of targets(v.state)) {
        const k = key(link);
        if (pending.has(k) || (!all && cache.has(k))) continue;
        pending.add(k);
        invoke<Res>("transclude_text", { link, from }).catch((e) => ({ error: String(e) })).then((r) => {
          pending.delete(k);
          cache.set(k, r);
          if (alive) v.dispatch({ effects: loaded.of(null) });
        });
      }
    };
    const refresh = () => run(true);
    live.add(refresh);
    if (!listening) (listening = true), listen("fs-changed", () => live.forEach((f) => f())).catch(() => (listening = false));
    queueMicrotask(refresh); // a fresh state (tab switch, reload): refetch, showing the cached result meanwhile
    return {
      update: (u) => void (u.docChanged && run(false)),
      destroy: () => ((alive = false), live.delete(refresh)),
    };
  });
  return [field, fetcher, theme];
}

const theme = EditorView.baseTheme({
  ".cm-transclude": {
    color: "var(--dim)", background: "var(--panel)", borderLeft: "3px solid var(--border)", padding: "0.2em 0.8em", margin: "0.1em 0 0.3em",
    whiteSpace: "pre-wrap", fontSize: "0.92em", cursor: "pointer", userSelect: "none", borderRadius: "0 var(--radius-sm) var(--radius-sm) 0",
  },
  ".cm-transclude:hover": { borderLeftColor: "var(--accent)" },
  ".cm-transclude-missing": { fontStyle: "italic" },
});
