import type { MonthTotal } from "../api";
import { formatCents, MONTH_NAMES } from "../utils";

interface Props {
  /** Oldest first. */
  points: MonthTotal[];
}

export default function TrendChart({ points }: Props) {
  const maxValue = Math.max(1, ...points.flatMap((p) => [p.total_income_cents, p.total_expense_cents]));

  return (
    <div className="rounded-lg border p-4" style={{ borderColor: "var(--border)", backgroundColor: "var(--surface-1)" }}>
      <div className="mb-3 flex items-center justify-between">
        <h3 className="text-sm font-medium" style={{ color: "var(--text-muted)" }}>Last {points.length} months</h3>
        <div className="flex items-center gap-3 text-xs" style={{ color: "var(--text-muted)" }}>
          <span className="flex items-center gap-1">
            <span className="inline-block h-2 w-2 rounded-full" style={{ backgroundColor: "var(--status-good)" }} /> Income
          </span>
          <span className="flex items-center gap-1">
            <span className="inline-block h-2 w-2 rounded-full" style={{ backgroundColor: "var(--status-critical)" }} /> Expenses
          </span>
        </div>
      </div>
      <div className="flex items-end gap-3 overflow-x-auto" style={{ height: "140px" }}>
        {points.map((p) => (
          <div key={`${p.year}-${p.month}`} className="flex flex-1 flex-col items-center gap-1" style={{ minWidth: "2.5rem" }}>
            <div className="flex h-full w-full items-end justify-center gap-1">
              <div
                title={`Income ${formatCents(p.total_income_cents)}`}
                className="w-3 rounded-t"
                style={{
                  height: `${Math.max(1, (p.total_income_cents / maxValue) * 100)}%`,
                  backgroundColor: "var(--status-good)",
                  opacity: p.total_income_cents > 0 ? 1 : 0.15,
                }}
              />
              <div
                title={`Expenses ${formatCents(p.total_expense_cents)}`}
                className="w-3 rounded-t"
                style={{
                  height: `${Math.max(1, (p.total_expense_cents / maxValue) * 100)}%`,
                  backgroundColor: "var(--status-critical)",
                  opacity: p.total_expense_cents > 0 ? 1 : 0.15,
                }}
              />
            </div>
            <span className="text-xs" style={{ color: "var(--text-muted)" }}>{MONTH_NAMES[p.month - 1].slice(0, 3)}</span>
            <span
              className="text-xs"
              style={{
                fontVariantNumeric: "tabular-nums",
                color: p.net_cents >= 0 ? "var(--status-good)" : "var(--status-critical)",
              }}
            >
              {formatCents(p.net_cents)}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}
