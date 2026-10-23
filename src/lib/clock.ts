/** HOURS as "1h 05m" (or "45m"). */
export const hm = (h: number) => {
  const m = Math.round(h * 60);
  return m >= 60 ? `${Math.floor(m / 60)}h ${String(m % 60).padStart(2, "0")}m` : `${m}m`;
};

/** A day of `tc_dashboard`'s week: hours worked and expected (time off already taken out). */
export type WeekDay = { date: string; hours: number; expected: number; leave: string; leave_hours: number };

/** "Worked today" for TODAY (yyyy-mm-dd): hours, the day's target, progress 0–1 and hours left. */
export function worked(week: WeekDay[], today: string) {
  const d = week.find((w) => w.date === today);
  const hours = d?.hours ?? 0, expected = d?.expected ?? 0;
  return { hours, expected, progress: expected > 0 ? Math.min(1, hours / expected) : hours > 0 ? 1 : 0, left: Math.max(0, expected - hours) };
}

/** Top-bar countdown to the focus break: "break in 12m" from SINCE (naive ISO, the open clock-in) and
 *  FOCUS minutes at NOW, "break due" once reached; "" when off or not clocked in. */
export function focusLeft(since: string | null | undefined, focus: number, now: Date): string {
  if (!since || !(focus > 0)) return "";
  const left = Math.ceil(focus - (now.getTime() - new Date(since).getTime()) / 60000);
  return left > 0 ? `break in ${hm(left / 60)}` : "break due";
}

const p2 =(n: number) => String(n).padStart(2, "0");

/** Org inactive timestamp "[2026-10-12 Mon 09:05]" for D, local time; English weekday regardless of locale. */
export const inactiveStamp = (d: Date) =>
  `[${d.getFullYear()}-${p2(d.getMonth() + 1)}-${p2(d.getDate())} ${["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"][d.getDay()]} ${p2(d.getHours())}:${p2(d.getMinutes())}]`;

/** Local naive "YYYY-MM-DDTHH:MM:SS" (what the backend's NaiveDateTime parses; toISOString is UTC with a Z). */
export const naive = (d: Date) =>
  `${d.getFullYear()}-${p2(d.getMonth() + 1)}-${p2(d.getDate())}T${p2(d.getHours())}:${p2(d.getMinutes())}:${p2(d.getSeconds())}`;

/** "25" = minutes ago, "14:05" = that time today (yesterday if still ahead of NOW); null if empty, invalid or future. */
export function parseWhen(input: string, now: Date): Date | null {
  const s = input.trim();
  let t: Date;
  const hm = /^(\d{1,2}):(\d{2})$/.exec(s);
  if (hm) {
    const [h, m] = [+hm[1], +hm[2]];
    if (h > 23 || m > 59) return null;
    t = new Date(now.getFullYear(), now.getMonth(), now.getDate(), h, m);
    if (t > now) t.setDate(t.getDate() - 1);
  } else if (/^\d+$/.test(s)) {
    t = new Date(now.getTime() - +s * 60000);
  } else return null;
  return t > now ? null : t;
}

/** "HH:MM" or "HHMM" on DAY (yyyy-mm-dd) as a naive ISO datetime; null when it isn't a time of day. */
export function dayTime(day: string, v: string): string | null {
  const m = v.trim().match(/^(\d{1,2}):?(\d\d)$/);
  return m && +m[1] < 24 && +m[2] < 60 ? `${day}T${m[1].padStart(2, "0")}:${m[2]}:00` : null;
}
