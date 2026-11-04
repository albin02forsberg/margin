/** Colour theme (`theme` in config.toml): "light" or "dark" override the OS; anything else follows it. */
export const effectiveTheme = (setting: unknown, systemLight: boolean): "light" | "dark" =>
  setting === "light" || setting === "dark" ? setting : systemLight ? "light" : "dark";

/** Set (or, for system, clear) the override on the root; CSS follows, `margin-theme` tells the diagrams. */
export function applyTheme(setting: unknown) {
  const el = document.documentElement;
  if (setting === "light" || setting === "dark") el.dataset.theme = setting; else delete el.dataset.theme;
  window.dispatchEvent(new Event("margin-theme"));
}
