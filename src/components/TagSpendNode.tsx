import { useEffect, useState } from "react";
import type { ChildSpend, RecurringProgress, Transaction } from "../api";
import { getTagBreakdown, listTransactionsForMonth } from "../api";
import { formatCents, formatDate, recurringDueInMonth, spendStatusColor } from "../utils";

interface Props {
  tagId: number;
  tagName: string;
  tagColor: string | null;
  spentCents: number;
  accountId: number | null;
  year: number;
  month: number;
  recurring: RecurringProgress[];
  depth: number;
}

/** One row in a budget's expandable breakdown - shows this tag's own transactions plus its
 * children, each of which recurses through the same component so drill-down works to any depth. */
export default function TagSpendNode({ tagId, tagName, tagColor, spentCents, accountId, year, month, recurring, depth }: Props) {
  const [expanded, setExpanded] = useState(false);
  const [children, setChildren] = useState<ChildSpend[] | null>(null);
  const [ownTxns, setOwnTxns] = useState<Transaction[] | null>(null);

  useEffect(() => {
    setExpanded(false);
    setChildren(null);
    setOwnTxns(null);
  }, [tagId, accountId, year, month]);

  async function load() {
    const [kids, txns] = await Promise.all([
      getTagBreakdown(tagId, accountId, year, month),
      listTransactionsForMonth(accountId, tagId, year, month),
    ]);
    setChildren(kids);
    setOwnTxns(txns);
  }

  async function toggle() {
    if (expanded) {
      setExpanded(false);
      return;
    }
    setExpanded(true);
    if (children === null) await load();
  }

  const prog = recurring.find((r) => r.recurring.tag_id === tagId);
  const projected = prog && prog.recurring.active
    ? recurringDueInMonth(prog.recurring.anchor_date, prog.recurring.interval_unit, prog.recurring.interval_count, year, month)
      ? prog.recurring.projected_amount_cents
      : 0
    : null;
  const color = spendStatusColor(spentCents, projected);

  return (
    <div className="flex flex-col gap-1">
      <button onClick={toggle} className="flex w-full items-center justify-between text-sm">
        <span className="flex items-center gap-2" style={{ color: "var(--text-secondary)" }}>
          <span className="text-xs" style={{ color: "var(--text-muted)" }}>{expanded ? "▾" : "▸"}</span>
          <span className="inline-block h-2 w-2 rounded-full" style={{ backgroundColor: tagColor ?? "#3987e5" }} />
          {tagName}
        </span>
        <span style={{ fontVariantNumeric: "tabular-nums" }}>
          {projected !== null && <span style={{ color: "var(--text-muted)" }}>proj {formatCents(projected)} · </span>}
          <span style={{ color: color ?? "var(--text-secondary)" }}>{formatCents(spentCents)}</span>
        </span>
      </button>

      {expanded && (
        <div className="ml-4 flex flex-col gap-2 border-l pl-3" style={{ borderColor: "var(--gridline)" }}>
          {children === null && <p className="text-xs" style={{ color: "var(--text-muted)" }}>Loading…</p>}

          {ownTxns && ownTxns.length === 0 && (children === null || children.length === 0) && (
            <p className="text-xs" style={{ color: "var(--text-muted)" }}>No transactions this month.</p>
          )}

          {ownTxns && ownTxns.length > 0 && (
            <div className="flex flex-col gap-1">
              {ownTxns.map((t) => (
                <div key={t.id} className="flex items-center justify-between text-xs">
                  <span style={{ color: "var(--text-muted)" }}>{formatDate(t.date)} · {t.description}</span>
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
              accountId={accountId}
              year={year}
              month={month}
              recurring={recurring}
              depth={depth + 1}
            />
          ))}
        </div>
      )}
    </div>
  );
}
