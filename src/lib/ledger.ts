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
