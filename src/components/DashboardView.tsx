import { useEffect, useState } from "react";
import type { Account, BudgetProgress, MonthSummary, Subscription, Tag, Transaction } from "../api";
import { deleteTransaction, getBudgetProgress, getMonthSummary, updateOccurrence } from "../api";
import { formatCents, monthLabel, shiftMonth } from "../utils";
import AddEntryForm from "./AddEntryForm";
import BudgetMeter from "./BudgetMeter";
import EditTransactionForm from "./EditTransactionForm";

interface Props {
  accounts: Account[];
  tags: Tag[];
  subscriptions: Subscription[];
  year: number;
  month: number;
  onMonthChange: (year: number, month: number) => void;
  onTagCreated: (tag: Tag) => void;
  dataVersion: number;
  bump: () => void;
}

const COLUMN_WIDTHS = {
  date: "96px",
  description: "auto",
  tags: "200px",
  account: "140px",
  status: "110px",
  amount: "120px",
  actions: "90px",
};

export default function DashboardView({
  accounts,
  tags,
  subscriptions,
  year,
  month,
  onMonthChange,
  onTagCreated,
  dataVersion,
  bump,
}: Props) {
  const [summary, setSummary] = useState<MonthSummary | null>(null);
  const [budgetProgress, setBudgetProgress] = useState<BudgetProgress[]>([]);
  const [accountFilter, setAccountFilter] = useState<number | "">("");
  const [showForm, setShowForm] = useState(false);
  const [editingTransaction, setEditingTransaction] = useState<Transaction | null>(null);
  const [editingOccurrenceId, setEditingOccurrenceId] = useState<number | null>(null);
  const [occurrenceAmountDraft, setOccurrenceAmountDraft] = useState("");
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    getMonthSummary(year, month, accountFilter === "" ? null : Number(accountFilter))
      .then((s) => {
        if (!cancelled) setSummary(s);
      })
      .finally(() => !cancelled && setLoading(false));
    return () => {
      cancelled = true;
    };
  }, [year, month, accountFilter, dataVersion]);

  useEffect(() => {
    getBudgetProgress(year, month).then(setBudgetProgress);
  }, [year, month, dataVersion]);

  const accountName = (id: number) => accounts.find((a) => a.id === id)?.name ?? `#${id}`;
  const subscriptionById = new Map(subscriptions.map((s) => [s.id, s]));

  type Row = {
    key: string;
    date: string;
    label: string;
    amountCents: number;
    tags: Tag[];
    meta: string;
    occurrenceId?: number;
    status?: string;
    transaction?: Transaction;
  };

  const rows: Row[] = [];
  if (summary) {
    for (const t of summary.transactions) {
      rows.push({
        key: `t-${t.id}`,
        date: t.date,
        label: t.description,
        amountCents: t.amount_cents,
        tags: t.tags,
        meta: `${accountName(t.account_id)} · ${t.type}`,
        transaction: t,
      });
    }
    for (const o of summary.occurrences) {
      const sub = subscriptionById.get(o.subscription_id);
      rows.push({
        key: `o-${o.id}`,
        date: o.due_date,
        label: sub?.name ?? `Subscription #${o.subscription_id}`,
        amountCents: o.amount_cents,
        tags: sub?.tags ?? [],
        meta: sub ? accountName(sub.account_id) : "",
        occurrenceId: o.id,
        status: o.status,
      });
    }
    rows.sort((a, b) => a.date.localeCompare(b.date));
  }

  async function handleDeleteTransaction(id: number) {
    await deleteTransaction(id);
    bump();
  }

  async function handleStatusChange(occurrenceId: number, status: string) {
    await updateOccurrence({ id: occurrenceId, status });
    bump();
  }

  async function handleSaveOccurrenceAmount(occurrenceId: number) {
    const cents = Math.round(parseFloat(occurrenceAmountDraft || "0") * 100);
    if (cents > 0) {
      await updateOccurrence({ id: occurrenceId, amountCents: cents });
      bump();
    }
    setEditingOccurrenceId(null);
  }

  return (
    <div className="flex flex-col gap-6">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-3">
          <button
            onClick={() => {
              const prev = shiftMonth(year, month, -1);
              onMonthChange(prev.year, prev.month);
            }}
            className="rounded border px-2 py-1 text-sm"
            style={{ borderColor: "var(--border)" }}
          >
            ←
          </button>
          <h2 className="min-w-40 text-center text-lg font-semibold">{monthLabel(year, month)}</h2>
          <button
            onClick={() => {
              const next = shiftMonth(year, month, 1);
              onMonthChange(next.year, next.month);
            }}
            className="rounded border px-2 py-1 text-sm"
            style={{ borderColor: "var(--border)" }}
          >
            →
          </button>
        </div>
        <div className="flex items-center gap-3">
          <select
            value={accountFilter}
            onChange={(e) => setAccountFilter(e.target.value ? Number(e.target.value) : "")}
            className="rounded border bg-transparent px-2 py-1 text-sm"
            style={{ borderColor: "var(--border)" }}
          >
            <option value="">All accounts</option>
            {accounts.map((a) => (
              <option key={a.id} value={a.id} style={{ color: "black" }}>{a.name}</option>
            ))}
          </select>
          <button
            onClick={() => setShowForm((v) => !v)}
            className="rounded px-3 py-1.5 text-sm font-medium"
            style={{ backgroundColor: "var(--accent)", color: "white" }}
          >
            {showForm ? "Close" : "+ Add"}
          </button>
        </div>
      </div>

      {showForm && (
        <AddEntryForm
          accounts={accounts}
          tags={tags}
          onTagCreated={onTagCreated}
          onCreated={() => {
            bump();
            setShowForm(false);
          }}
        />
      )}

      {editingTransaction && (
        <EditTransactionForm
          transaction={editingTransaction}
          accounts={accounts}
          tags={tags}
          onTagCreated={onTagCreated}
          onSaved={() => {
            bump();
            setEditingTransaction(null);
          }}
          onCancel={() => setEditingTransaction(null)}
        />
      )}

      {summary && (
        <div className="grid grid-cols-3 gap-3">
          <StatTile label="Income" value={formatCents(summary.total_income_cents)} />
          <StatTile label="Expenses" value={formatCents(summary.total_expense_cents)} />
          <StatTile
            label="Net"
            value={formatCents(summary.net_cents)}
            color={summary.net_cents >= 0 ? "var(--status-good)" : "var(--status-critical)"}
          />
        </div>
      )}

      {budgetProgress.some((p) => p.budget.show_on_dashboard) && (
        <div>
          <h3 className="mb-2 text-sm font-medium" style={{ color: "var(--text-muted)" }}>Budgets</h3>
          <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
            {budgetProgress
              .filter((p) => p.budget.show_on_dashboard)
              .map((p) => (
                <BudgetMeter key={p.budget.id} progress={p} tags={tags} />
              ))}
          </div>
        </div>
      )}

      <div className="overflow-x-auto rounded-lg border" style={{ borderColor: "var(--border)" }}>
        <table className="w-full table-fixed text-sm">
          <colgroup>
            <col style={{ width: COLUMN_WIDTHS.date }} />
            <col style={{ width: COLUMN_WIDTHS.description }} />
            <col style={{ width: COLUMN_WIDTHS.tags }} />
            <col style={{ width: COLUMN_WIDTHS.account }} />
            <col style={{ width: COLUMN_WIDTHS.status }} />
            <col style={{ width: COLUMN_WIDTHS.amount }} />
            <col style={{ width: COLUMN_WIDTHS.actions }} />
          </colgroup>
          <thead>
            <tr className="border-b" style={{ color: "var(--text-muted)", borderColor: "var(--gridline)" }}>
              <th className="px-3 py-2 text-left font-normal">Date</th>
              <th className="px-3 py-2 text-left font-normal">Title</th>
              <th className="px-3 py-2 text-left font-normal">Tags</th>
              <th className="px-3 py-2 text-left font-normal">Account</th>
              <th className="px-3 py-2 text-left font-normal">Status</th>
              <th className="px-3 py-2 text-right font-normal">Amount</th>
              <th className="px-3 py-2" />
            </tr>
          </thead>
          <tbody>
            {rows.map((r) => (
              <tr key={r.key} className="border-b align-top" style={{ borderColor: "var(--gridline)" }}>
                <td className="truncate px-3 py-2" style={{ color: "var(--text-secondary)", fontVariantNumeric: "tabular-nums" }}>
                  {r.date}
                </td>
                <td className="truncate px-3 py-2">{r.label}</td>
                <td className="px-3 py-2">
                  <div className="flex flex-wrap gap-1">
                    {r.tags.map((t) => (
                      <span
                        key={t.id}
                        className="rounded-full px-2 py-0.5 text-xs"
                        style={{ backgroundColor: `${t.color ?? "#3987e5"}22`, color: "var(--text-secondary)" }}
                      >
                        {t.name}
                      </span>
                    ))}
                  </div>
                </td>
                <td className="truncate px-3 py-2" style={{ color: "var(--text-secondary)" }}>{r.meta}</td>
                <td className="px-3 py-2">
                  {r.occurrenceId !== undefined ? (
                    <select
                      value={r.status}
                      onChange={(e) => handleStatusChange(r.occurrenceId!, e.target.value)}
                      className="w-full rounded border bg-transparent px-1 py-0.5 text-xs"
                      style={{ borderColor: "var(--border)" }}
                    >
                      <option value="projected" style={{ color: "black" }}>projected</option>
                      <option value="confirmed" style={{ color: "black" }}>confirmed</option>
                      <option value="skipped" style={{ color: "black" }}>skipped</option>
                    </select>
                  ) : (
                    <span style={{ color: "var(--text-muted)" }}>—</span>
                  )}
                </td>
                <td
                  className="px-3 py-2 text-right"
                  style={{
                    fontVariantNumeric: "tabular-nums",
                    color: r.amountCents < 0 ? "var(--status-critical)" : "var(--status-good)",
                  }}
                >
                  {r.occurrenceId !== undefined && editingOccurrenceId === r.occurrenceId ? (
                    <input
                      autoFocus
                      type="number"
                      step="0.01"
                      value={occurrenceAmountDraft}
                      onChange={(e) => setOccurrenceAmountDraft(e.target.value)}
                      onBlur={() => handleSaveOccurrenceAmount(r.occurrenceId!)}
                      onKeyDown={(e) => {
                        if (e.key === "Enter") handleSaveOccurrenceAmount(r.occurrenceId!);
                        if (e.key === "Escape") setEditingOccurrenceId(null);
                      }}
                      className="w-24 rounded border bg-transparent px-1 py-0.5 text-right text-sm"
                      style={{ borderColor: "var(--border)", color: "var(--text-primary)" }}
                    />
                  ) : r.occurrenceId !== undefined ? (
                    <button
                      onClick={() => {
                        setEditingOccurrenceId(r.occurrenceId!);
                        setOccurrenceAmountDraft((Math.abs(r.amountCents) / 100).toString());
                      }}
                      title="Click to edit this cycle's amount"
                    >
                      {formatCents(r.amountCents)}
                    </button>
                  ) : (
                    formatCents(r.amountCents)
                  )}
                </td>
                <td className="px-3 py-2 text-right">
                  {r.transaction && (
                    <div className="flex justify-end gap-2">
                      <button
                        onClick={() => setEditingTransaction(r.transaction!)}
                        className="text-xs"
                        style={{ color: "var(--text-muted)" }}
                      >
                        edit
                      </button>
                      <button
                        onClick={() => handleDeleteTransaction(r.transaction!.id)}
                        className="text-xs"
                        style={{ color: "var(--text-muted)" }}
                      >
                        delete
                      </button>
                    </div>
                  )}
                </td>
              </tr>
            ))}
            {!loading && rows.length === 0 && (
              <tr>
                <td colSpan={7} className="px-3 py-6 text-center" style={{ color: "var(--text-muted)" }}>
                  Nothing this month yet.
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
}

function StatTile({ label, value, color }: { label: string; value: string; color?: string }) {
  return (
    <div className="rounded-lg border p-4" style={{ borderColor: "var(--border)", backgroundColor: "var(--surface-1)" }}>
      <p className="text-xs" style={{ color: "var(--text-muted)" }}>{label}</p>
      <p className="mt-1 text-2xl font-semibold" style={{ color: color ?? "var(--text-primary)" }}>{value}</p>
    </div>
  );
}
