/** Whether the background feed fetch is due: LAST and NOW are unix seconds (LAST 0 = never), HOURS the
 *  `feeds_auto_hours` setting. Off for 0/NaN/negative; a clock gone backwards (LAST in the future) counts as due. */
export const feedsDue = (last: number, hours: number, now: number) =>
  hours > 0 && Number.isFinite(hours) && (last > now || now - last >= hours * 3600);
