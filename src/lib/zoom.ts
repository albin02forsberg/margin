/** UI zoom level: 0.1 steps between MIN and MAX (the app owns it instead of Tauri's hotkey script, so it can be remembered). */
const MIN = 0.5, MAX = 3;
const clamp = (z: number) => Math.min(MAX, Math.max(MIN, Math.round(z * 10) / 10));

export const nextZoom = (z: number, dir: "in" | "out" | "reset") => (dir === "reset" ? 1 : clamp(z + (dir === "in" ? 0.1 : -0.1)));

/** A stored zoom level; anything unreadable is 1. */
export function parseZoom(raw: string | null) {
  const z = Number(raw);
  return raw && Number.isFinite(z) && z > 0 ? clamp(z) : 1;
}
