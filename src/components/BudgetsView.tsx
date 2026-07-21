import { useEffect, useState } from "react";
import type { Account, BudgetProgress, RecurringProgress, Tag } from "../api";
import { deleteBudget, getBudgetProgress, updateBudget } from "../api";
import { monthLabel, orderTagsHierarchically } from "../utils";
import BudgetMeter from "./BudgetMeter";

interface Props {
  accounts: Account[];
  tags: Tag[];
  recurring: RecurringProgress[];
  year: number;
  month: number;
  dataVersion: number;
  bump: () => void;
}

export default function BudgetsView({ accounts, tags, recurring, year, month, dataVersion, bump }: Props) {
  const [progress, setProgress] = useState<BudgetProgress[]>([]);
  const [editingId, setEditingId] = useState<number | null>(null);
  const [accountId, setAccountId] = useState<number | "">("");
  const [tagId, setTagId] = useState<number | "">("");
  const [amount, setAmount] = useState("");
  const [showOnDashboard, setShowOnDashboard] = useState(true);
  const [submitting, setSubmitting] = useState(false);

  function refreshProgress() {
    getBudgetProgress(year, month).then(setProgress);
  }

  useEffect(refreshProgress, [year, month, dataVersion]);

  function resetForm() {
    setEditingId(null);
    setAccountId("");
    setTagId("");
    setAmount("");
    setShowOnDashboard(true);
  }

  function startEdit(p: BudgetProgress) {
    setEditingId(p.budget.id);
    setAccountId(p.budget.account_id ?? "");
    setTagId(p.budget.tag_id ?? "");
    setAmount((p.budget.amount_cents / 100).toString());
    setShowOnDashboard(p.budget.show_on_dashboard);
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (editingId === null) return;
    const cents = Math.round(parseFloat(amount || "0") * 100);
    if (!cents) return;
    setSubmitting(true);
    try {
      await updateBudget(editingId, accountId === "" ? null : Number(accountId), tagId === "" ? null : Number(tagId), cents, showOnDashboard);
      resetForm();
      bump();
    } finally {
      setSubmitting(false);
    }
  }

  async function handleDelete(p: BudgetProgress) {
    const label = p.budget.tag_id ? tags.find((t) => t.id === p.budget.tag_id)?.name ?? "this budget" : "this overall budget";
    if (!window.confirm(`Delete the budget for "${label}"? This can't be undone.`)) return;
    await deleteBudget(p.budget.id);
    if (editingId === p.budget.id) resetForm();
    bump();
  }

  const selectStyle = { borderColor: "var(--border)" };

  return (
    <div className="flex flex-col gap-6">
      <p className="text-sm" style={{ color: "var(--text-muted)" }}>
        New budgets are created from a category above - use "generate budget" on the one you want. This section is
        for reviewing and adjusting the ones you already have.
      </p>

      {editingId !== null && (
        <form
          onSubmit={handleSubmit}
          className="flex flex-wrap items-end gap-3 rounded-lg border p-4"
          style={{ borderColor: "var(--accent)", backgroundColor: "var(--surface-1)" }}
        >
          <div className="flex flex-col gap-1">
            <label className="text-xs" style={{ color: "var(--text-muted)" }}>Tag / category</label>
            <select
              value={tagId}
              onChange={(e) => setTagId(e.target.value ? Number(e.target.value) : "")}
              className="rounded border bg-transparent px-2 py-1 text-sm"
              style={selectStyle}
            >
              <option value="">Overall</option>
              {orderTagsHierarchically(tags).map((t) => (
                <option key={t.id} value={t.id} style={{ color: "black" }}>
                  {t.parent_id ? `— ${t.name}` : t.name}
                </option>
              ))}
            </select>
          </div>
          <div className="flex flex-col gap-1">
            <label className="text-xs" style={{ color: "var(--text-muted)" }}>Account (blank = any)</label>
            <select
              value={accountId}
              onChange={(e) => setAccountId(e.target.value ? Number(e.target.value) : "")}
              className="rounded border bg-transparent px-2 py-1 text-sm"
              style={selectStyle}
            >
              <option value="">Any account</option>
              {accounts.map((a) => (
                <option key={a.id} value={a.id} style={{ color: "black" }}>{a.name}</option>
              ))}
            </select>
          </div>
          <div className="flex flex-col gap-1">
            <label className="text-xs" style={{ color: "var(--text-muted)" }}>Monthly amount</label>
            <input
              type="number"
              step="0.01"
              value={amount}
              onChange={(e) => setAmount(e.target.value)}
              className="w-32 rounded border bg-transparent px-2 py-1 text-sm"
              style={selectStyle}
            />
          </div>
          <label className="flex items-center gap-2 pb-1.5 text-xs" style={{ color: "var(--text-secondary)" }}>
            <input type="checkbox" checked={showOnDashboard} onChange={(e) => setShowOnDashboard(e.target.checked)} />
            Show on dashboard
          </label>
          <button
            type="submit"
            disabled={submitting}
            className="rounded px-3 py-1.5 text-sm font-medium"
            style={{ backgroundColor: "var(--accent)", color: "white" }}
          >
            Save changes
          </button>
          <button type="button" onClick={resetForm} className="rounded border px-3 py-1.5 text-sm" style={selectStyle}>
            Cancel
          </button>
        </form>
      )}

      <div>
        <h3 className="mb-3 text-sm font-medium" style={{ color: "var(--text-muted)" }}>{monthLabel(year, month)}</h3>
        <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
          {progress.map((p) => (
            <BudgetMeter
              key={p.budget.id}
              progress={p}
              tags={tags}
              accounts={accounts}
              recurring={recurring}
              year={year}
              month={month}
              onEdit={() => startEdit(p)}
              onDelete={() => handleDelete(p)}
              onChanged={refreshProgress}
            />
          ))}
          {progress.length === 0 && (
            <p className="text-sm" style={{ color: "var(--text-muted)" }}>
              No budgets yet - use "generate budget" on a category above.
            </p>
          )}
        </div>
      </div>
    </div>
  );
}
