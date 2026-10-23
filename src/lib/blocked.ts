/** Whether a task list shows IT (#718): blocked tasks (`:ORDERED:` / `:BLOCKER:`) are hidden unless SHOWALL,
 *  except ones dated TODAY or earlier (scheduled, due or overdue), which stay so a deadline isn't lost. */
export const visible = (it: { blocked?: boolean; date: string }, today: string, showAll: boolean) =>
  !it.blocked || showAll || (it.date.length === 10 && it.date <= today);
