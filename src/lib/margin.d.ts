// Types for the `margin` object Margin hands to init scripts (init/*.ts). Written to init/margin.d.ts by the palette
// command "Write init script types"; keep in step with `Margin` in commands.ts (margin.types-check.ts fails on drift).
// A global script (no import/export), so `margin` is in scope in every script.
type MarginHook = "save" | "open" | "startup";
type MarginCommand = { label: string; keys?: string[]; leader?: string; ctx?: "editor" | "normal"; run: () => unknown };
type MarginBuffer = {
  /** Path of the focused note, if it is a file. */
  readonly path?: string;
  /** Throws when no note is focused. */
  text?: () => string;
  selection?: () => { from: number; to: number; text: string };
  /** Replaces the selection. */
  insert?: (text: string) => void;
  replace?: (from: number, to: number, text: string) => void;
};
type MarginApi = {
  command: (c: MarginCommand) => void;
  register: (c: MarginCommand) => void;
  /** Bind KEYS ("Ctrl+Alt+H" or a list) to the command labelled LABEL. */
  bind: (keys: string | string[], label: string) => void;
  on: (hook: MarginHook, fn: (...a: any[]) => unknown) => void;
  /** Session-only, like Emacs `setq`; config.toml is never written. */
  config: { get: (key: string) => unknown; set: (key: string, value: unknown) => void };
  buffer: MarginBuffer;
  reload: () => unknown;
};
declare const margin: MarginApi;
