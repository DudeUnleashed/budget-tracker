const CURRENCY_STORAGE_KEY = "budget-tracker:currency";

/** Curated list of common currencies for the settings dropdown - not the full ISO 4217 set.
 * `locale` picks the separator/symbol-placement convention each currency is normally shown
 * with (e.g. EUR as "1.234,56 €" rather than the US-style "€1,234.56") - it's about number
 * formatting only, not language or translation. */
export const CURRENCIES: { code: string; label: string; locale: string }[] = [
  { code: "USD", label: "USD - US Dollar ($)", locale: "en-US" },
  { code: "EUR", label: "EUR - Euro (€)", locale: "de-DE" },
  { code: "GBP", label: "GBP - British Pound (£)", locale: "en-GB" },
  { code: "PHP", label: "PHP - Philippine Peso (₱)", locale: "en-PH" },
  { code: "JPY", label: "JPY - Japanese Yen (¥)", locale: "ja-JP" },
  { code: "CNY", label: "CNY - Chinese Yuan (¥)", locale: "zh-CN" },
  { code: "INR", label: "INR - Indian Rupee (₹)", locale: "en-IN" },
  { code: "AUD", label: "AUD - Australian Dollar ($)", locale: "en-AU" },
  { code: "CAD", label: "CAD - Canadian Dollar ($)", locale: "en-CA" },
  { code: "NZD", label: "NZD - New Zealand Dollar ($)", locale: "en-NZ" },
  { code: "SGD", label: "SGD - Singapore Dollar ($)", locale: "en-SG" },
  { code: "HKD", label: "HKD - Hong Kong Dollar ($)", locale: "zh-HK" },
  { code: "CHF", label: "CHF - Swiss Franc", locale: "de-CH" },
  { code: "SEK", label: "SEK - Swedish Krona", locale: "sv-SE" },
  { code: "NOK", label: "NOK - Norwegian Krone", locale: "nb-NO" },
  { code: "DKK", label: "DKK - Danish Krone", locale: "da-DK" },
  { code: "KRW", label: "KRW - South Korean Won (₩)", locale: "ko-KR" },
  { code: "IDR", label: "IDR - Indonesian Rupiah (Rp)", locale: "id-ID" },
  { code: "MYR", label: "MYR - Malaysian Ringgit (RM)", locale: "ms-MY" },
  { code: "THB", label: "THB - Thai Baht (฿)", locale: "th-TH" },
  { code: "VND", label: "VND - Vietnamese Dong (₫)", locale: "vi-VN" },
  { code: "MXN", label: "MXN - Mexican Peso ($)", locale: "es-MX" },
  { code: "BRL", label: "BRL - Brazilian Real (R$)", locale: "pt-BR" },
  { code: "ZAR", label: "ZAR - South African Rand (R)", locale: "en-ZA" },
  { code: "AED", label: "AED - UAE Dirham", locale: "ar-AE" },
  { code: "PLN", label: "PLN - Polish Zloty", locale: "pl-PL" },
  { code: "TRY", label: "TRY - Turkish Lira (₺)", locale: "tr-TR" },
];

let currencyCode = localStorage.getItem(CURRENCY_STORAGE_KEY) || "USD";

export function getCurrencyCode(): string {
  return currencyCode;
}

/** Only changes how amounts are labeled ($ vs ₱ vs ...) - every stored value is still the same
 * number of cents, there's no unit conversion. Persisted per-install since it's a display
 * preference, not data that belongs in a CSV backup. */
export function setCurrencyCode(code: string): void {
  currencyCode = code;
  localStorage.setItem(CURRENCY_STORAGE_KEY, code);
}

export function formatCents(cents: number): string {
  const locale = CURRENCIES.find((c) => c.code === currencyCode)?.locale ?? "en-US";
  return (cents / 100).toLocaleString(locale, { style: "currency", currency: currencyCode });
}

const DATE_FORMAT_STORAGE_KEY = "budget-tracker:date-format";

export type DateFormat = "dmy" | "mdy" | "ymd" | "ydm";

export const DATE_FORMAT_OPTIONS: { value: DateFormat; label: string }[] = [
  { value: "dmy", label: "Day/Month/Year (31/12/2026)" },
  { value: "mdy", label: "Month/Day/Year (12/31/2026)" },
  { value: "ymd", label: "Year/Month/Day (2026/12/31)" },
  { value: "ydm", label: "Year/Day/Month (2026/31/12)" },
];

let dateFormat: DateFormat = (localStorage.getItem(DATE_FORMAT_STORAGE_KEY) as DateFormat | null) || "dmy";

export function getDateFormat(): DateFormat {
  return dateFormat;
}

/** Only changes how dates are displayed as text - every date is still stored (and compared,
 * sorted, sent to the backend) as an ISO YYYY-MM-DD string. Native date pickers show their own
 * OS/browser-locale format regardless and aren't affected either way. */
export function setDateFormat(format: DateFormat): void {
  dateFormat = format;
  localStorage.setItem(DATE_FORMAT_STORAGE_KEY, format);
}

/** Reformats a stored ISO (YYYY-MM-DD) date for display per the active date format setting.
 * Returns the input unchanged if it isn't a recognizable ISO date. */
export function formatDate(isoDate: string): string {
  const match = /^(\d{4})-(\d{2})-(\d{2})/.exec(isoDate);
  if (!match) return isoDate;
  const [, y, mo, d] = match;
  switch (dateFormat) {
    case "mdy":
      return `${mo}/${d}/${y}`;
    case "ymd":
      return `${y}/${mo}/${d}`;
    case "ydm":
      return `${y}/${d}/${mo}`;
    case "dmy":
    default:
      return `${d}/${mo}/${y}`;
  }
}

export const MONTH_NAMES = [
  "January", "February", "March", "April", "May", "June",
  "July", "August", "September", "October", "November", "December",
];

export function monthLabel(year: number, month: number): string {
  return `${MONTH_NAMES[month - 1]} ${year}`;
}

export function shiftMonth(year: number, month: number, delta: number): { year: number; month: number } {
  const total = year * 12 + (month - 1) + delta;
  return { year: Math.floor(total / 12), month: (total % 12) + 1 };
}

export function todayIso(): string {
  const d = new Date();
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${d.getFullYear()}-${m}-${day}`;
}

/** good < 70%, warning 70-99%, critical >= 100% - matches the dataviz skill's status palette. */
export function budgetStatus(percent: number): "good" | "warning" | "critical" {
  if (percent >= 100) return "critical";
  if (percent >= 70) return "warning";
  return "good";
}

export const STATUS_COLOR: Record<"good" | "warning" | "critical", string> = {
  good: "#0ca30c",
  warning: "#fab219",
  critical: "#d03b3b",
};

/** Orders tags so each tag is immediately followed by its own descendants (depth-first), for a
 * grouped/tree feel in flat lists (dropdowns, chip pickers) without a real tree widget. Works to
 * arbitrary depth, not just one level of children. */
export function orderTagsHierarchically<T extends { id: number; parent_id: number | null }>(tags: T[]): T[] {
  const ordered: T[] = [];
  const seen = new Set<number>();
  function addChildren(parentId: number) {
    for (const t of tags.filter((c) => c.parent_id === parentId)) {
      if (seen.has(t.id)) continue;
      seen.add(t.id);
      ordered.push(t);
      addChildren(t.id);
    }
  }
  for (const root of tags.filter((t) => t.parent_id === null)) {
    if (seen.has(root.id)) continue;
    seen.add(root.id);
    ordered.push(root);
    addChildren(root.id);
  }
  ordered.push(...tags.filter((t) => !seen.has(t.id)));
  return ordered;
}

/** How many parents deep a tag is nested - 0 for a top-level tag. */
export function tagDepth<T extends { id: number; parent_id: number | null }>(tag: T, tags: T[]): number {
  let depth = 0;
  let current: T | undefined = tag;
  while (current && current.parent_id !== null) {
    current = tags.find((t) => t.id === current!.parent_id);
    depth++;
  }
  return depth;
}

/** All descendant ids of a tag (children, grandchildren, ...), used to keep a tag from being
 * re-parented under one of its own descendants. */
export function descendantTagIds<T extends { id: number; parent_id: number | null }>(tagId: number, tags: T[]): Set<number> {
  const ids = new Set<number>();
  const stack = [tagId];
  while (stack.length > 0) {
    const id = stack.pop()!;
    for (const t of tags.filter((c) => c.parent_id === id)) {
      if (!ids.has(t.id)) {
        ids.add(t.id);
        stack.push(t.id);
      }
    }
  }
  return ids;
}

/** Converts a recurring amount on any cadence to an approximate monthly-equivalent, so cadences
 * that don't align to a calendar month (e.g. quarterly) can still be summed into a monthly budget. */
export function monthlyEquivalentCents(
  amountCents: number,
  intervalUnit: "day" | "week" | "month" | "year",
  intervalCount: number,
): number {
  const AVG_DAYS_PER_MONTH = 30.44;
  const monthsPerCycle =
    intervalUnit === "month"
      ? intervalCount
      : intervalUnit === "year"
        ? intervalCount * 12
        : intervalUnit === "week"
          ? (intervalCount * 7) / AVG_DAYS_PER_MONTH
          : intervalCount / AVG_DAYS_PER_MONTH;
  return Math.round(amountCents / monthsPerCycle);
}

/** Sum of a parent tag's children's recurring projections (monthly-equivalent), for pre-filling
 * a budget so it doesn't need to be kept in sync by hand. Returns null if no children have a
 * recurring definition. */
export function suggestedBudgetAmountCents(
  parentTagId: number,
  tags: { id: number; parent_id: number | null }[],
  recurring: {
    recurring: {
      tag_id: number;
      projected_amount_cents: number;
      interval_unit: "day" | "week" | "month" | "year";
      interval_count: number;
      active: boolean;
    };
  }[],
): { cents: number; count: number } | null {
  const children = tags.filter((t) => t.parent_id === parentTagId);
  if (children.length === 0) return null;
  let total = 0;
  let count = 0;
  for (const child of children) {
    const prog = recurring.find((r) => r.recurring.tag_id === child.id);
    if (prog && prog.recurring.active) {
      total += monthlyEquivalentCents(prog.recurring.projected_amount_cents, prog.recurring.interval_unit, prog.recurring.interval_count);
      count++;
    }
  }
  return count > 0 ? { cents: total, count } : null;
}

function addMonthsClamped(date: Date, months: number): Date {
  const totalMonths = date.getUTCFullYear() * 12 + date.getUTCMonth() + months;
  const year = Math.floor(totalMonths / 12);
  const month = ((totalMonths % 12) + 12) % 12;
  const lastDay = new Date(Date.UTC(year, month + 1, 0)).getUTCDate();
  return new Date(Date.UTC(year, month, Math.min(date.getUTCDate(), lastDay)));
}

function addInterval(date: Date, unit: "day" | "week" | "month" | "year", count: number): Date {
  if (unit === "day") return new Date(Date.UTC(date.getUTCFullYear(), date.getUTCMonth(), date.getUTCDate() + count));
  if (unit === "week") return new Date(Date.UTC(date.getUTCFullYear(), date.getUTCMonth(), date.getUTCDate() + count * 7));
  if (unit === "year") return addMonthsClamped(date, count * 12);
  return addMonthsClamped(date, count);
}

/** Whether a recurring item (anchored at `anchorDate`, stepping by interval_unit/count) has a
 * cycle that starts within the given calendar month - i.e. it actually renews that month, rather
 * than being amortized across every month. Mirrors the backend's cycle-stepping logic. */
export function recurringDueInMonth(
  anchorDate: string,
  intervalUnit: "day" | "week" | "month" | "year",
  intervalCount: number,
  year: number,
  month: number,
): boolean {
  const monthStart = new Date(Date.UTC(year, month - 1, 1));
  const monthEnd = new Date(Date.UTC(month === 12 ? year + 1 : year, month === 12 ? 0 : month, 1));
  let cursor = new Date(`${anchorDate}T00:00:00Z`);
  const MAX_STEPS = 2000;
  for (let i = 0; i < MAX_STEPS; i++) {
    if (cursor >= monthEnd) return false;
    if (cursor >= monthStart) return true;
    cursor = addInterval(cursor, intervalUnit, intervalCount);
  }
  return false;
}

/** At-a-glance color for a spend-vs-projected figure: green once paid at or under the
 * projected amount, red once it runs over, and undefined (default text color) until anything
 * has actually been paid or when there's nothing to compare against. */
export function spendStatusColor(spentCents: number, projectedCents: number | null): string | undefined {
  if (projectedCents === null || spentCents === 0) return undefined;
  return spentCents <= projectedCents ? STATUS_COLOR.good : STATUS_COLOR.critical;
}
