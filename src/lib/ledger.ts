/** Ledger month view helpers (#789). */

/** CENTS (signed minor units) as `−12.50 EUR` / `+1200.00 EUR`; SIGN false drops the + of positive amounts. */
export function money(cents: number, cur: string, sign = true) {
  const a = Math.abs(cents);
  return `${cents < 0 ? "−" : sign && cents > 0 ? "+" : ""}${Math.floor(a / 100)}.${String(a % 100).padStart(2, "0")} ${cur}`;
}

/** MONTH (`YYYY-MM`) moved by N months. */
export function shiftMonth(month: string, n: number) {
  const m = +month.slice(0, 4) * 12 + +month.slice(5, 7) - 1 + n;
  return `${Math.floor(m / 12)}-${String((m % 12) + 1).padStart(2, "0")}`;
}

/** MONTH (`YYYY-MM`) as "October 2026". */
export const monthName = (month: string) => new Date(+month.slice(0, 4), +month.slice(5, 7) - 1, 1).toLocaleDateString(undefined, { month: "long", year: "numeric" });

/** A project card's ledger line for one currency (#790): "Ledger · cost 340.00 EUR · income 900.00 EUR", parts left out when 0. */
export function costLine(t: { currency: string; income: number; expense: number }) {
  const parts = [t.expense ? `cost ${money(-t.expense, t.currency, false)}` : "", t.income ? `income ${money(t.income, t.currency, false)}` : ""];
  return ["Ledger", ...parts.filter(Boolean)].join(" · ");
}

/** A budget bar's fill (0-100, refunds clamp at 0) and whether USED went over BUDGET (cents, #1181). */
export const budgetBar = (used: number, budget: number) => ({ pct: Math.max(0, Math.min(100, Math.round((used / budget) * 100))), over: used > budget });

type FRow = { cents: number; currency: string; text: string; category: string | null; payee: string | null };

/** ROWS matching filter Q (#1195): `#cat` / `@payee` match exactly (`#-` = uncategorised), else a substring of text, category or payee; all case-insensitive. */
export function filterRows<T extends FRow>(rows: T[], q: string): T[] {
  q = q.trim().toLowerCase();
  if (!q) return rows;
  const v = q.slice(1);
  if (q[0] === "#" && v) return rows.filter((r) => (v === "-" ? !r.category : r.category?.toLowerCase() === v));
  if (q[0] === "@" && v) return rows.filter((r) => r.payee?.toLowerCase() === v);
  return rows.filter((r) => [r.text, r.category, r.payee].some((s) => s?.toLowerCase().includes(q)));
}

/** Signed cents summed per currency, in code order. */
export function sumByCurrency(rows: { cents: number; currency: string }[]): [string, number][] {
  const m = new Map<string, number>();
  for (const r of rows) m.set(r.currency, (m.get(r.currency) ?? 0) + r.cents);
  return [...m].sort((a, b) => (a[0] < b[0] ? -1 : 1));
}

/** The share of INCOME kept (NET / INCOME, cents) as a whole percent, or null without income (#1196). */
export const savingsRate = (income: number, net: number) => (income > 0 ? Math.round((net / income) * 100) : null);
