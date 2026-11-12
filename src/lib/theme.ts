/** Colour theme (`theme` in config.toml): "light" or "dark" override the OS; anything else follows it. */
export const effectiveTheme = (setting: unknown, systemLight: boolean): "light" | "dark" =>
  setting === "light" || setting === "dark" ? setting : systemLight ? "light" : "dark";

/** Next in the cycle system -> light -> dark -> system (unknown counts as system); "" is system. */
export const nextTheme = (current: unknown): "light" | "dark" | "" =>
  current === "light" ? "dark" : current === "dark" ? "" : "light";

/** Set (or, for system, clear) the override on the root; CSS follows, `margin-theme` tells the diagrams. */
export function applyTheme(setting: unknown) {
  const el = document.documentElement;
  if (setting === "light" || setting === "dark") el.dataset.theme = setting; else delete el.dataset.theme;
  window.dispatchEvent(new Event("margin-theme"));
}

/** Set (or, when empty, clear) `data-look` on the root; any non-empty name is accepted so CSS can key off it. */
export function applyLook(name: unknown) {
  const el = document.documentElement;
  if (typeof name === "string" && name) el.dataset.look = name; else delete el.dataset.look;
}

/** Set (or, when empty, clear) `data-layout` on the root, so CSS can move the shell's regions. */
export function applyLayout(name: unknown) {
  const el = document.documentElement;
  if (typeof name === "string" && name) el.dataset.layout = name; else delete el.dataset.layout;
}

/** The layout a user look asks for in a leading `/* layout: NAME *\/` comment ("" for the default), or null without one. */
export const lookLayout = (css: string | null | undefined): string | null =>
  css?.match(/^\uFEFF?\s*\/\*\s*layout:\s*([\w-]*)\s*\*\//)?.[1] ?? null;

/** Put a user look's CSS in `<style id="user-look">` (created on first use, after the built-in styles), or remove it when CSS is null. */
export function setUserLook(css: string | null | undefined) {
  let el = document.getElementById("user-look");
  if (css == null) return el?.remove();
  if (!el) (el = document.createElement("style")), (el.id = "user-look"), document.head.append(el);
  el.textContent = css;
}
