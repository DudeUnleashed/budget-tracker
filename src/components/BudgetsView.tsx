import { useEffect, useState } from "react";
import type { BudgetProgress, Tag } from "../api";
import { createBudget, createTag, deleteBudget, getBudgetProgress, updateBudget } from "../api";
import { monthLabel } from "../utils";
import BudgetMeter from "./BudgetMeter";

interface Props {
  tags: Tag[];
  year: number;
  month: number;
  dataVersion: number;
  bump: () => void;
  onTagCreated: (tag: Tag) => void;
}

export default function BudgetsView({ tags, year, month, dataVersion, bump, onTagCreated }: Props) {
  const [progress, setProgress] = useState<BudgetProgress[]>([]);
  const [editingId, setEditingId] = useState<number | null>(null);
  const [ledger, setLedger] = useState<string>("");
  const [tagId, setTagId] = useState<number | "">("");
  const [amount, setAmount] = useState("");
  const [showOnDashboard, setShowOnDashboard] = useState(true);
  const [newTagName, setNewTagName] = useState("");
  const [submitting, setSubmitting] = useState(false);

  useEffect(() => {
    getBudgetProgress(year, month).then(setProgress);
  }, [year, month, dataVersion]);

  function resetForm() {
    setEditingId(null);
    setLedger("");
    setTagId("");
    setAmount("");
    setShowOnDashboard(true);
  }

  function startEdit(p: BudgetProgress) {
    setEditingId(p.budget.id);
    setLedger(p.budget.ledger ?? "");
    setTagId(p.budget.tag_id ?? "");
    setAmount((p.budget.amount_cents / 100).toString());
    setShowOnDashboard(p.budget.show_on_dashboard);
  }

  async function handleAddTag() {
    const name = newTagName.trim();
    if (!name) return;
    const tag = await createTag(name, null);
    onTagCreated(tag);
    setTagId(tag.id);
    setNewTagName("");
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    const cents = Math.round(parseFloat(amount || "0") * 100);
    if (!cents) return;
    setSubmitting(true);
    try {
      const resolvedTagId = tagId === "" ? null : Number(tagId);
      if (editingId !== null) {
        await updateBudget(editingId, ledger || null, resolvedTagId, cents, showOnDashboard);
      } else {
        await createBudget(ledger || null, resolvedTagId, cents, showOnDashboard);
      }
      resetForm();
      bump();
    } finally {
      setSubmitting(false);
    }
  }

  async function handleDelete(id: number) {
    await deleteBudget(id);
    if (editingId === id) resetForm();
    bump();
  }

  const selectStyle = { borderColor: "var(--border)" };

  return (
    <div className="flex flex-col gap-6">
      <form
        onSubmit={handleSubmit}
        className="flex flex-col gap-3 rounded-lg border p-4"
        style={{ borderColor: "var(--border)", backgroundColor: "var(--surface-1)" }}
      >
        <div className="flex flex-wrap items-end gap-3">
          <div className="flex flex-col gap-1">
            <label className="text-xs" style={{ color: "var(--text-muted)" }}>Tag / category (blank = overall)</label>
            <select
              value={tagId}
              onChange={(e) => setTagId(e.target.value ? Number(e.target.value) : "")}
              className="rounded border bg-transparent px-2 py-1 text-sm"
              style={selectStyle}
            >
              <option value="">Overall</option>
              {tags.map((t) => (
                <option key={t.id} value={t.id} style={{ color: "black" }}>{t.name}</option>
              ))}
            </select>
          </div>
          <div className="flex items-end gap-1">
            <div className="flex flex-col gap-1">
              <label className="text-xs" style={{ color: "var(--text-muted)" }}>Or add new category</label>
              <input
                value={newTagName}
                onChange={(e) => setNewTagName(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") {
                    e.preventDefault();
                    handleAddTag();
                  }
                }}
                placeholder="Groceries"
                className="w-32 rounded border border-dashed bg-transparent px-2 py-1 text-sm"
                style={selectStyle}
              />
            </div>
            <button type="button" onClick={handleAddTag} className="rounded border px-2 py-1 text-xs" style={selectStyle}>
              + add
            </button>
          </div>
          <div className="flex flex-col gap-1">
            <label className="text-xs" style={{ color: "var(--text-muted)" }}>Ledger (blank = all)</label>
            <select value={ledger} onChange={(e) => setLedger(e.target.value)} className="rounded border bg-transparent px-2 py-1 text-sm" style={selectStyle}>
              <option value="">All ledgers</option>
              <option value="personal" style={{ color: "black" }}>personal</option>
              <option value="business" style={{ color: "black" }}>business</option>
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
            <input
              type="checkbox"
              checked={showOnDashboard}
              onChange={(e) => setShowOnDashboard(e.target.checked)}
            />
            Show on dashboard
          </label>
          <button
            type="submit"
            disabled={submitting}
            className="rounded px-3 py-1.5 text-sm font-medium"
            style={{ backgroundColor: "var(--accent)", color: "white" }}
          >
            {editingId !== null ? "Save changes" : "Add budget"}
          </button>
          {editingId !== null && (
            <button type="button" onClick={resetForm} className="rounded border px-3 py-1.5 text-sm" style={selectStyle}>
              Cancel
            </button>
          )}
        </div>
      </form>

      <div>
        <h3 className="mb-3 text-sm font-medium" style={{ color: "var(--text-muted)" }}>{monthLabel(year, month)}</h3>
        <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
          {progress.map((p) => (
            <BudgetMeter
              key={p.budget.id}
              progress={p}
              tags={tags}
              onEdit={() => startEdit(p)}
              onDelete={() => handleDelete(p.budget.id)}
            />
          ))}
          {progress.length === 0 && (
            <p className="text-sm" style={{ color: "var(--text-muted)" }}>No budgets set yet.</p>
          )}
        </div>
      </div>
    </div>
  );
}
