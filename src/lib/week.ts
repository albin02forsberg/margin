/** Week start (config `week_start`, #1091): "monday" (the default and anything unknown), "sunday" or "saturday". */
const first = (ws?: string) => (ws === "sunday" ? 0 : ws === "saturday" ? 6 : 1);

/** How many days into its week (starting on WS) a day with JS weekday DOW (0 = Sunday) is. */
export const intoWeek = (dow: number, ws?: string) => (dow - first(ws) + 7) % 7;

/** Short weekday names in column order for WS. */
export const weekDays = (ws?: string) => Array.from({ length: 7 }, (_, i) => ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"][(first(ws) + i) % 7]);

/** The first day of the week (starting on WS) containing local date D, at midnight. */
export const weekStartOf = (d: Date, ws?: string) => new Date(d.getFullYear(), d.getMonth(), d.getDate() - intoWeek(d.getDay(), ws));
