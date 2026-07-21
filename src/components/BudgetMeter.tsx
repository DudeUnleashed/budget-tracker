import { useEffect, useState } from "react";
import type { Account, BudgetProgress, ChildSpend, RecurringProgress, Tag, Transaction } from "../api";
import { clearBudgetMonthOverride, getBudgetBreakdown, listTransactionsForMonth, setBudgetMonthOverride } from "../api";
import { budgetStatus, formatCents, formatDate, STATUS_COLOR } from "../utils";
import TagSpendNode from "./TagSpendNode";

interface Props {
  progress: BudgetProgress;
  tags: Tag[];
  accounts: Account[];
  recurring: RecurringProgress[];
  year: number;
  month: number;
  onEdit?: () => void;
  onDelete?: () => void;
  onChanged?: () => void;
}

export default function BudgetMeter({ progress: p, tags, accounts, recurring, year, month, onEdit, onDelete, onChanged }: Props) {
  const tagName = p.budget.tag_id ? tags.find((t) => t.id === p.budget.tag_id)?.name ?? "Tag" : "Overall";
  const accountName = p.budget.account_id !== null ? accounts.find((a) => a.id === p.budget.account_id)?.name ?? "Account" : null;
  const status = budgetStatus(p.percent);
  const color = STATUS_COLOR[status];
  const pct = Math.min(p.percent, 100);

  const [expanded, setExpanded] = useState(false);
  const [children, setChildren] = useState<ChildSpend[] | null>(null);
  const [ownTxns, setOwnTxns] = useState<Transaction[] | null>(null);
  const [overriding, setOverriding] = useState(false);
  const [overrideInput, setOverrideInput] = useState("");

  // The month can change (or a different budget can be shown) while this stays expanded -
  // cached breakdown data would otherwise go stale silently.
  useEffect(() => {
    setChildren(null);
    setOwnTxns(null);
    setOverriding(false);
    if (expanded) {
      void loadBreakdown();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [year, month, p.budget.id]);

  async function loadBreakdown() {
    if (p.budget.tag_id === null) return;
    const [kids, txns] = await Promise.all([
      getBudgetBreakdown(p.budget.id, year, month),
      listTransactionsForMonth(p.budget.account_id, p.budget.tag_id, year, month),
    ]);
    setChildren(kids);
    setOwnTxns(txns);
  }

  async function toggleExpand() {
    if (expanded) {
      setExpanded(false);
      return;
    }
    setExpanded(true);
    if (children === null) {
      await loadBreakdown();
    }
  }

  function startOverride() {
    setOverriding(true);
    setOverrideInput((p.amount_cents / 100).toString());
  }

  async function saveOverride() {
    const cents = Math.round(parseFloat(overrideInput || "0") * 100);
    if (cents > 0) {
      await setBudgetMonthOverride(p.budget.id, year, month, cents);
      onChanged?.();
    }
    setOverriding(false);
  }

  async function resetOverride() {
    await clearBudgetMonthOverride(p.budget.id, year, month);
    onChanged?.();
  }

  const canExpand = p.budget.tag_id !== null;

  return (
    <div
      className="rounded-lg border p-4"
      style={{ borderColor: "var(--border)", backgroundColor: "var(--surface-1)" }}
    >
      <div className="flex items-center justify-between gap-2">
        <button
          onClick={canExpand ? toggleExpand : undefined}
          className="flex items-center gap-1 font-medium"
          style={{ cursor: canExpand ? "pointer" : "default" }}
        >
          {canExpand && <span className="text-xs" style={{ color: "var(--text-muted)" }}>{expanded ? "▾" : "▸"}</span>}
          {tagName}
          {accountName ? ` · ${accountName}` : ""}
        </button>
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
        {formatCents(p.spent_cents)} <span style={{ color: "var(--text-muted)" }}>of</span> {formatCents(p.amount_cents)}
      </p>
      <div className="mt-2 h-2 w-full overflow-hidden rounded-full" style={{ backgroundColor: "var(--gridline)" }}>
        <div className="h-full rounded-full transition-all" style={{ width: `${pct}%`, backgroundColor: color }} />
      </div>

      {overriding ? (
        <div className="mt-2 flex items-center gap-2">
          <input
            type="number"
            step="0.01"
            autoFocus
            value={overrideInput}
            onChange={(e) => setOverrideInput(e.target.value)}
            className="w-24 rounded border bg-transparent px-2 py-1 text-xs"
            style={{ borderColor: "var(--border)" }}
          />
          <button onClick={saveOverride} className="text-xs" style={{ color: "var(--accent)" }}>save</button>
          <button onClick={() => setOverriding(false)} className="text-xs" style={{ color: "var(--text-muted)" }}>cancel</button>
        </div>
      ) : (
        <div className="mt-2 flex items-center gap-2 text-xs">
          {p.is_override ? (
            <>
              <span style={{ color: "var(--status-warning)" }}>customized for {formatCents(p.amount_cents)} this month</span>
              <button onClick={resetOverride} style={{ color: "var(--text-muted)" }}>reset to default</button>
            </>
          ) : (
            <button onClick={startOverride} style={{ color: "var(--text-muted)" }}>override this month</button>
          )}
        </div>
      )}

      {expanded && (
        <div className="mt-3 flex flex-col gap-2 border-t pt-3" style={{ borderColor: "var(--gridline)" }}>
          {children === null && <p className="text-xs" style={{ color: "var(--text-muted)" }}>Loading…</p>}

          {ownTxns && ownTxns.length === 0 && (children === null || children.length === 0) && (
            <p className="text-xs" style={{ color: "var(--text-muted)" }}>No transactions this month.</p>
          )}

          {ownTxns && ownTxns.length > 0 && (
            <div className="flex flex-col gap-1">
              {ownTxns.map((t) => (
                <div key={t.id} className="flex items-center justify-between text-sm">
                  <span style={{ color: "var(--text-secondary)" }}>{formatDate(t.date)} · {t.description}</span>
                  <span style={{ fontVariantNumeric: "tabular-nums", color: t.amount_cents < 0 ? "var(--status-critical)" : "var(--status-good)" }}>
                    {formatCents(t.amount_cents)}
                  </span>
                </div>
              ))}
            </div>
          )}

          {children?.map((c) => (
            <TagSpendNode
              key={c.tag.id}
              tagId={c.tag.id}
              tagName={c.tag.name}
              tagColor={c.tag.color}
              spentCents={c.spent_cents}
              accountId={p.budget.account_id}
              year={year}
              month={month}
              recurring={recurring}
              depth={0}
            />
          ))}
        </div>
      )}
    </div>
  );
}
