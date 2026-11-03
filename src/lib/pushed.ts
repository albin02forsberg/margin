/** Reschedules after which a task counts as stuck (chip, nudge, review). */
export const PUSHED_MIN = 3;

/** Whether a task was rescheduled often enough to flag. */
export const isPushed = (n?: number | null): boolean => (n ?? 0) >= PUSHED_MIN;
