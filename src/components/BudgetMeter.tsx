import type { BudgetProgress, Tag } from "../api";
import { budgetStatus, formatCents, STATUS_COLOR } from "../utils";

interface Props {
  progress: BudgetProgress;
  tags: Tag[];
  onEdit?: () => void;
  onDelete?: () => void;
}

export default function BudgetMeter({ progress: p, tags, onEdit, onDelete }: Props) {
  const tagName = p.budget.tag_id ? tags.find((t) => t.id === p.budget.tag_id)?.name ?? "Tag" : "Overall";
  const status = budgetStatus(p.percent);
  const color = STATUS_COLOR[status];
  const pct = Math.min(p.percent, 100);

  return (
    <div
      className="rounded-lg border p-4"
      style={{ borderColor: "var(--border)", backgroundColor: "var(--surface-1)" }}
    >
      <div className="flex items-center justify-between gap-2">
        <span className="font-medium">
          {tagName}
          {p.budget.ledger ? ` · ${p.budget.ledger}` : ""}
        </span>
        <div className="flex items-center gap-2">
          <span className="flex items-center gap-1 text-xs" style={{ color: "var(--text-secondary)" }}>
            <span className="inline-block h-2 w-2 rounded-full" style={{ backgroundColor: color }} />
            {p.percent.toFixed(0)}%
          </span>
          {onEdit && (
            <button onClick={onEdit} className="text-xs" style={{ color: "var(--text-muted)" }}>
              edit
            </button>
          )}
          {onDelete && (
            <button onClick={onDelete} className="text-xs" style={{ color: "var(--text-muted)" }}>
              delete
            </button>
          )}
        </div>
      </div>
      <p className="mt-2 text-sm" style={{ color: "var(--text-secondary)" }}>
        {formatCents(p.spent_cents)} <span style={{ color: "var(--text-muted)" }}>of</span> {formatCents(p.budget.amount_cents)}
      </p>
      <div className="mt-2 h-2 w-full overflow-hidden rounded-full" style={{ backgroundColor: "var(--gridline)" }}>
        <div className="h-full rounded-full transition-all" style={{ width: `${pct}%`, backgroundColor: color }} />
      </div>
    </div>
  );
}
