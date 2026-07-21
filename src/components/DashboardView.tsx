import { useEffect, useState } from "react";
import type { Account, BudgetProgress, MonthSummary, MonthTotal, RecurringProgress, Tag, Transaction } from "../api";
import { deleteTransaction, getAccountBalance, getBudgetProgress, getMonthlyTrend, getMonthSummary } from "../api";
import { formatCents, monthLabel, shiftMonth } from "../utils";
import AddEntryForm from "./AddEntryForm";
import BudgetMeter from "./BudgetMeter";
import EditTransactionForm from "./EditTransactionForm";
import LedgerTable from "./LedgerTable";
import TrendChart from "./TrendChart";

const TREND_MONTHS = 6;

interface Props {
  accounts: Account[];
  tags: Tag[];
  recurring: RecurringProgress[];
  year: number;
  month: number;
  onMonthChange: (year: number, month: number) => void;
  onTagCreated: (tag: Tag) => void;
  dataVersion: number;
  bump: () => void;
}

export default function DashboardView({
  accounts,
  tags,
  recurring,
  year,
  month,
  onMonthChange,
  onTagCreated,
  dataVersion,
  bump,
}: Props) {
  const [summary, setSummary] = useState<MonthSummary | null>(null);
  const [trend, setTrend] = useState<MonthTotal[]>([]);
  const [budgetProgress, setBudgetProgress] = useState<BudgetProgress[]>([]);
  const [accountFilter, setAccountFilter] = useState<number | "">("");
  const [showForm, setShowForm] = useState(false);
  const [editingTransaction, setEditingTransaction] = useState<Transaction | null>(null);
  const [defaultBalance, setDefaultBalance] = useState<number | null>(null);

  const defaultAccount = accounts.find((a) => a.is_default) ?? null;

  useEffect(() => {
    if (!defaultAccount) {
      setDefaultBalance(null);
      return;
    }
    let cancelled = false;
    getAccountBalance(defaultAccount.id).then((b) => {
      if (!cancelled) setDefaultBalance(b);
    });
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [defaultAccount?.id, dataVersion]);

  useEffect(() => {
    let cancelled = false;
    getMonthSummary(year, month, accountFilter === "" ? null : Number(accountFilter)).then((s) => {
      if (!cancelled) setSummary(s);
    });
    return () => {
      cancelled = true;
    };
  }, [year, month, accountFilter, dataVersion]);

  useEffect(() => {
    let cancelled = false;
    getMonthlyTrend(year, month, TREND_MONTHS, accountFilter === "" ? null : Number(accountFilter)).then((t) => {
      if (!cancelled) setTrend(t);
    });
    return () => {
      cancelled = true;
    };
  }, [year, month, accountFilter, dataVersion]);

  function refreshBudgetProgress() {
    getBudgetProgress(year, month).then(setBudgetProgress);
  }

  useEffect(refreshBudgetProgress, [year, month, dataVersion]);

  async function handleDeleteTransaction(id: number) {
    const t = summary?.transactions.find((tx) => tx.id === id);
    if (!window.confirm(`Delete "${t?.description ?? "this transaction"}"? This can't be undone.`)) return;
    await deleteTransaction(id);
    bump();
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
            aria-label="Previous month"
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
            aria-label="Next month"
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
        <div className="grid grid-cols-4 gap-3">
          <StatTile label="Income" value={formatCents(summary.total_income_cents)} />
          <StatTile label="Expenses" value={formatCents(summary.total_expense_cents)} />
          <StatTile
            label="Net"
            value={formatCents(summary.net_cents)}
            color={summary.net_cents >= 0 ? "var(--status-good)" : "var(--status-critical)"}
          />
          <StatTile
            label={defaultAccount ? `${defaultAccount.name} balance` : "Balance"}
            value={defaultAccount ? (defaultBalance !== null ? formatCents(defaultBalance) : "…") : "No default account"}
          />
        </div>
      )}

      {trend.length > 0 && <TrendChart points={trend} />}

      {budgetProgress.some((p) => p.budget.show_on_dashboard) && (
        <div>
          <h3 className="mb-2 text-sm font-medium" style={{ color: "var(--text-muted)" }}>Budgets</h3>
          <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
            {budgetProgress
              .filter((p) => p.budget.show_on_dashboard)
              .map((p) => (
                <BudgetMeter
                  key={p.budget.id}
                  progress={p}
                  tags={tags}
                  accounts={accounts}
                  recurring={recurring}
                  year={year}
                  month={month}
                  onChanged={refreshBudgetProgress}
                />
              ))}
          </div>
        </div>
      )}

      <LedgerTable
        transactions={summary?.transactions ?? []}
        accounts={accounts}
        onEdit={setEditingTransaction}
        onDelete={handleDeleteTransaction}
        emptyMessage="Nothing this month yet."
      />
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
