/** Habits overview helpers (#802). */

const BLOCKS = "▁▂▃▄▅▆▇█";

/** A habit BAR ('x' done, one char per day, oldest first) as a sparkline of completions per 7 days,
 *  chunked from today backwards (an older partial week is scaled to its length). */
export function spark(bar: string): string {
  let out = "";
  for (let end = bar.length; end > 0; end -= 7) {
    const w = bar.slice(Math.max(0, end - 7), end);
    out = BLOCKS[Math.round(([...w].filter((c) => c === "x").length / w.length) * (BLOCKS.length - 1))] + out;
  }
  return out;
}
