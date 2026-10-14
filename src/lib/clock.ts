/** A day of `tc_dashboard`'s week: hours worked and expected (time off already taken out). */
export type WeekDay = { date: string; hours: number; expected: number; leave: string; leave_hours: number };

/** "Worked today" for TODAY (yyyy-mm-dd): hours, the day's target, progress 0–1 and hours left. */
export function worked(week: WeekDay[], today: string) {
  const d = week.find((w) => w.date === today);
  const hours = d?.hours ?? 0, expected = d?.expected ?? 0;
  return { hours, expected, progress: expected > 0 ? Math.min(1, hours / expected) : hours > 0 ? 1 : 0, left: Math.max(0, expected - hours) };
}
