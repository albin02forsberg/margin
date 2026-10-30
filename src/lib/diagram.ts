// Mermaid diagrams (#931): a `#+begin_src mermaid` block shows its diagram under the `#+end_src` line, as a block
// widget (like transclusions), so line numbers and vim motions are untouched. mermaid is large, so it loads on the
// first diagram; it is bundled (no CDN) and runs with securityLevel strict. Folding the block keeps the diagram.
import { StateEffect, StateField, type EditorState, type Extension } from "@codemirror/state";
import { Decoration, EditorView, ViewPlugin, WidgetType, type DecorationSet } from "@codemirror/view";
import { diagramKey, srcBlocks } from "./babel";

/** Characters of diagram source rendered at most. */
const MAX = 20_000;
type Out = { svg: string } | { error: string };
/** Rendered diagrams by `theme\0code`, shared by every view, the oldest dropped past 64. */
const cache = new Map<string, Out>();
/** mermaid.render isn't reentrant: one at a time. */
let queue: Promise<unknown> = Promise.resolve();
let seq = 0;
const LIGHT = "(prefers-color-scheme: light)";
const retheme = StateEffect.define<null>();

function render(code: string, theme: string): Promise<Out> {
  const k = `${theme}\0${code}`, hit = cache.get(k);
  if (hit) return Promise.resolve(hit);
  const p = queue.then(async (): Promise<Out> => {
    const { default: m } = await import("mermaid");
    m.initialize({ startOnLoad: false, securityLevel: "strict", theme: theme as "dark" | "default", suppressErrorRendering: true });
    try { return { svg: (await m.render(`margin-mermaid-${++seq}`, code)).svg }; }
    catch (e) { return { error: String((e as Error)?.message ?? e).trim() }; }
  }).catch((e): Out => ({ error: `mermaid didn't load: ${e}` }));
  queue = p;
  return p.then((r) => {
    cache.set(k, r);
    if (cache.size > 64) cache.delete(cache.keys().next().value!);
    return r;
  });
}

/** TEXT's mermaid diagrams as light-theme SVGs by `diagramKey`, for HTML/PDF export (#942); failed ones are left out. */
export async function svgs(text: string): Promise<Record<string, string>> {
  const out: Record<string, string> = {};
  if (!/#\+begin_src[ \t]+mermaid\b/i.test(text)) return out;
  for (const b of srcBlocks(text)) {
    if (b.lang.toLowerCase() !== "mermaid" || !b.code.trim() || b.code.length > MAX) continue;
    const r = await render(b.code, "default");
    if ("svg" in r) out[diagramKey(b.code)] = r.svg;
  }
  return out;
}

class Diagram extends WidgetType {
  code: string; theme: string;
  constructor(code: string, theme: string) { super(); this.code = code; this.theme = theme; }
  eq(o: Diagram) { return o.code === this.code && o.theme === this.theme; }
  toDOM(view: EditorView) {
    const d = document.createElement("div");
    d.className = "cm-diagram";
    const fill = (r: Out) => {
      if ("svg" in r) d.innerHTML = r.svg;
      else (d.className = "cm-diagram cm-diagram-error"), (d.textContent = `⚠ ${r.error.split("\n")[0]}`), (d.title = r.error);
      view.requestMeasure();
    };
    const hit = cache.get(`${this.theme}\0${this.code}`);
    if (hit) fill(hit);
    else if (this.code.length > MAX) fill({ error: `Diagram too large to render (over ${MAX} characters)` });
    else {
      d.textContent = "…";
      // A pause first, so typing in the block renders only once it stops (a replaced widget is detached by then).
      setTimeout(() => d.isConnected && render(this.code, this.theme).then(fill), 250);
    }
    return d;
  }
  ignoreEvent() { return true; }
}

function build(s: EditorState): DecorationSet {
  const t = s.doc.toString();
  if (!/#\+begin_src[ \t]+mermaid\b/i.test(t)) return Decoration.none;
  const theme = matchMedia(LIGHT).matches ? "default" : "dark";
  return Decoration.set(srcBlocks(t).filter((b) => b.lang.toLowerCase() === "mermaid" && b.code.trim())
    .map((b) => Decoration.widget({ widget: new Diagram(b.code, theme), block: true, side: 1 }).range(b.end)));
}

const field = StateField.define<DecorationSet>({
  create: build,
  update: (d, tr) => (tr.docChanged || tr.effects.some((e) => e.is(retheme)) ? build(tr.state) : d),
  provide: (f) => EditorView.decorations.from(f),
});

/** Re-render in the other palette when the system switches light/dark. */
const watch = ViewPlugin.define((v) => {
  const q = matchMedia(LIGHT), f = () => v.dispatch({ effects: retheme.of(null) });
  q.addEventListener("change", f);
  return { destroy: () => q.removeEventListener("change", f) };
});

const theme = EditorView.baseTheme({
  ".cm-diagram": { padding: "0.6em 36px", margin: "0.1em 0 0.3em", textAlign: "center", overflowX: "auto", cursor: "default", color: "var(--dim)" },
  ".cm-diagram svg": { maxWidth: "100%", height: "auto" },
  ".cm-diagram-error": { textAlign: "left", fontSize: "0.85em", fontFamily: "var(--mono)", color: "var(--todo)", whiteSpace: "pre-wrap" },
});

/** Mermaid diagrams under `#+begin_src mermaid` blocks of an org note. */
export const diagrams: Extension = [field, watch, theme];
