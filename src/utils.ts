export function formatCents(cents: number): string {
  return (cents / 100).toLocaleString("en-US", { style: "currency", currency: "USD" });
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
