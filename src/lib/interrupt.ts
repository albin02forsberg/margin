/** A clock stacked by Space t ! (#1293), offered back once its interruption stops. */
export type Clock = { project: string; task: string };
type Status = { profile?: string; project?: string | null };

/** The clock to offer back when the status went from PREV to NOW: the top of STACK (popped) when the
 * running project went set -> none. A profile change drops the stack (resuming into another profile is wrong). */
export function resumeOffer(stack: Clock[], prev: Status, now: Status): Clock | null {
  if (prev.profile !== now.profile) stack.length = 0;
  return prev.project != null && now.project == null ? (stack.pop() ?? null) : null;
}
