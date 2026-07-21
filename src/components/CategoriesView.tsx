import { useState } from "react";
import type { Account, Budget, IntervalUnit, RecurringProgress, Tag, Transaction } from "../api";
import {
  createBudget,
  createRecurring,
  createTag,
  deleteRecurring,
  deleteTag,
  listRecurringCycleTransactions,
  setRecurringActive,
  updateRecurring,
  updateTag,
} from "../api";
import { descendantTagIds, formatCents, formatDate, orderTagsHierarchically, suggestedBudgetAmountCents, tagDepth, todayIso } from "../utils";
import BudgetsView from "./BudgetsView";

interface Props {
  accounts: Account[];
  tags: Tag[];
  recurring: RecurringProgress[];
  budgets: Budget[];
  year: number;
  month: number;
  dataVersion: number;
  bump: () => void;
}

type TagMode = "existing" | "new";

export default function CategoriesView({ accounts, tags, recurring, budgets, year, month, dataVersion, bump }: Props) {
  const topLevel = orderTagsHierarchically(tags).filter((t) => t.parent_id === null);
  const childrenOf = (id: number) => tags.filter((t) => t.parent_id === id);
  const recurringByTagId = new Map(recurring.map((r) => [r.recurring.tag_id, r]));
  const tagsWithoutRecurring = tags.filter((t) => !recurringByTagId.has(t.id));
  const budgetByTagId = new Map(budgets.filter((b) => b.tag_id !== null).map((b) => [b.tag_id as number, b]));
  const defaultAccountId = accounts.find((a) => a.is_default)?.id ?? "";

  // Add-category form
  const [catName, setCatName] = useState("");
  const [catColor, setCatColor] = useState("#3987e5");
  const [catParentId, setCatParentId] = useState<number | "">("");
  const [catSubmitting, setCatSubmitting] = useState(false);
  const [catError, setCatError] = useState("");

  // Tag inline edit
  const [editingTagId, setEditingTagId] = useState<number | null>(null);
  const [editTagName, setEditTagName] = useState("");
  const [editTagColor, setEditTagColor] = useState("#3987e5");
  const [editTagParentId, setEditTagParentId] = useState<number | "">("");
  const [editTagError, setEditTagError] = useState("");

  // Add/edit recurring form
  const [showRecurringForm, setShowRecurringForm] = useState(false);
  const [editingRecurringId, setEditingRecurringId] = useState<number | null>(null);
  const [tagMode, setTagMode] = useState<TagMode>("existing");
  const [recurTagId, setRecurTagId] = useState<number | "">("");
  const [newTagName, setNewTagName] = useState("");
  const [newTagColor, setNewTagColor] = useState("#3987e5");
  const [newTagParentId, setNewTagParentId] = useState<number | "">("");
  const [recurAccountId, setRecurAccountId] = useState<number | "">(defaultAccountId);
  const [recurKind, setRecurKind] = useState<"expense" | "income">("expense");
  const [intervalUnit, setIntervalUnit] = useState<IntervalUnit>("month");
  const [intervalCount, setIntervalCount] = useState("1");
  const [anchorDate, setAnchorDate] = useState(todayIso());
  const [projectedAmount, setProjectedAmount] = useState("");
  const [recurNotes, setRecurNotes] = useState("");
  const [recurSubmitting, setRecurSubmitting] = useState(false);
  const [recurError, setRecurError] = useState("");

  // Drill-down
  const [expandedId, setExpandedId] = useState<number | null>(null);
  const [cycleTxns, setCycleTxns] = useState<Transaction[]>([]);

  // Generate-budget form
  const [budgetFormTagId, setBudgetFormTagId] = useState<number | null>(null);
  const [budgetAccountId, setBudgetAccountId] = useState<number | "">(defaultAccountId);
  const [budgetAmount, setBudgetAmount] = useState("");
  const [budgetShowOnDashboard, setBudgetShowOnDashboard] = useState(true);
  const [budgetSuggestionCount, setBudgetSuggestionCount] = useState(0);
  const [budgetSubmitting, setBudgetSubmitting] = useState(false);

  const selectStyle = { borderColor: "var(--border)" };
  const accountName = (id: number | null) => (id === null ? "Any account" : accounts.find((a) => a.id === id)?.name ?? `#${id}`);

  // Tag names only have to be unique among siblings now, so the one collision left to explain
  // nicely is "another category at this same level already has this name".
  function friendlyTagError(err: unknown): string {
    const message = String(err);
    if (message.includes("UNIQUE constraint failed")) {
      return "A category with this name already exists at this level - pick a different name or a different parent.";
    }
    return message;
  }

  async function handleCreateCategory(e: React.FormEvent) {
    e.preventDefault();
    if (!catName.trim()) return;
    setCatError("");
    setCatSubmitting(true);
    try {
      await createTag(catName.trim(), catColor, catParentId === "" ? null : Number(catParentId));
      setCatName("");
      bump();
    } catch (err) {
      setCatError(friendlyTagError(err));
    } finally {
      setCatSubmitting(false);
    }
  }

  function startEditTag(t: Tag) {
    setEditingTagId(t.id);
    setEditTagName(t.name);
    setEditTagColor(t.color ?? "#3987e5");
    setEditTagParentId(t.parent_id ?? "");
    setEditTagError("");
  }

  async function handleSaveTagEdit(t: Tag) {
    setEditTagError("");
    try {
      await updateTag(t.id, editTagName.trim(), editTagColor, editTagParentId === "" ? null : Number(editTagParentId));
      setEditingTagId(null);
      bump();
    } catch (err) {
      setEditTagError(friendlyTagError(err));
    }
  }

  async function handleDeleteTag(tag: Tag) {
    if (!window.confirm(`Delete "${tag.name}"? It'll be removed from any transactions that have it.`)) return;
    try {
      await deleteTag(tag.id);
      bump();
    } catch (err) {
      window.alert(String(err));
    }
  }

  function resetRecurringForm() {
    setShowRecurringForm(false);
    setEditingRecurringId(null);
    setTagMode("existing");
    setRecurTagId("");
    setNewTagName("");
    setNewTagParentId("");
    setRecurAccountId(defaultAccountId);
    setRecurKind("expense");
    setIntervalUnit("month");
    setIntervalCount("1");
    setAnchorDate(todayIso());
    setProjectedAmount("");
    setRecurNotes("");
    setRecurError("");
  }

  function startAddRecurringFor(tagId: number) {
    resetRecurringForm();
    setShowRecurringForm(true);
    setTagMode("existing");
    setRecurTagId(tagId);
  }

  function startEditRecurring(p: RecurringProgress) {
    setShowRecurringForm(true);
    setEditingRecurringId(p.recurring.id);
    setTagMode("existing");
    setRecurTagId(p.recurring.tag_id);
    setRecurAccountId(p.recurring.account_id ?? "");
    setRecurKind(p.recurring.type);
    setIntervalUnit(p.recurring.interval_unit);
    setIntervalCount(String(p.recurring.interval_count));
    setAnchorDate(p.recurring.anchor_date);
    setProjectedAmount((p.recurring.projected_amount_cents / 100).toString());
    setRecurNotes(p.recurring.notes ?? "");
  }

  async function handleSubmitRecurring(e: React.FormEvent) {
    e.preventDefault();
    setRecurError("");
    const cents = Math.round(parseFloat(projectedAmount || "0") * 100);
    if (!cents) {
      setRecurError("Projected amount is required.");
      return;
    }
    if (tagMode === "existing" && !recurTagId) {
      setRecurError("Pick a tag.");
      return;
    }
    if (tagMode === "new" && !newTagName.trim()) {
      setRecurError("Name the new tag.");
      return;
    }
    setRecurSubmitting(true);
    try {
      let tagId = recurTagId === "" ? null : Number(recurTagId);
      if (tagMode === "new") {
        const created = await createTag(newTagName.trim(), newTagColor, newTagParentId === "" ? null : Number(newTagParentId));
        tagId = created.id;
      }
      const payload = {
        tagId: tagId as number,
        accountId: recurAccountId === "" ? null : Number(recurAccountId),
        kind: recurKind,
        intervalUnit,
        intervalCount: Number(intervalCount) || 1,
        anchorDate,
        projectedAmountCents: cents,
        notes: recurNotes.trim() || null,
      };
      if (editingRecurringId !== null) {
        await updateRecurring({ id: editingRecurringId, ...payload });
      } else {
        await createRecurring(payload);
      }
      resetRecurringForm();
      bump();
    } catch (err) {
      setRecurError(String(err));
    } finally {
      setRecurSubmitting(false);
    }
  }

  async function handleDeleteRecurring(prog: RecurringProgress) {
    if (!window.confirm(`Remove the recurring item for "${prog.tag.name}"? The tag and its transaction history stay.`)) return;
    await deleteRecurring(prog.recurring.id);
    if (editingRecurringId === prog.recurring.id) resetRecurringForm();
    bump();
  }

  async function handleToggleActive(prog: RecurringProgress) {
    await setRecurringActive(prog.recurring.id, !prog.recurring.active);
    bump();
  }

  function startGenerateBudget(tagId: number) {
    setBudgetFormTagId(tagId);
    setBudgetAccountId(defaultAccountId);
    setBudgetShowOnDashboard(true);
    const suggestion = suggestedBudgetAmountCents(tagId, tags, recurring);
    setBudgetAmount(suggestion ? (suggestion.cents / 100).toString() : "");
    setBudgetSuggestionCount(suggestion?.count ?? 0);
  }

  function resetBudgetForm() {
    setBudgetFormTagId(null);
    setBudgetAccountId(defaultAccountId);
    setBudgetAmount("");
    setBudgetShowOnDashboard(true);
    setBudgetSuggestionCount(0);
  }

  async function handleSubmitBudget(e: React.FormEvent) {
    e.preventDefault();
    if (budgetFormTagId === null) return;
    const cents = Math.round(parseFloat(budgetAmount || "0") * 100);
    if (!cents) return;
    setBudgetSubmitting(true);
    try {
      await createBudget(budgetAccountId === "" ? null : Number(budgetAccountId), budgetFormTagId, cents, budgetShowOnDashboard);
      resetBudgetForm();
      bump();
    } finally {
      setBudgetSubmitting(false);
    }
  }

  async function toggleExpand(p: RecurringProgress) {
    if (expandedId === p.recurring.id) {
      setExpandedId(null);
      return;
    }
    setExpandedId(p.recurring.id);
    setCycleTxns(await listRecurringCycleTransactions(p.recurring.id));
  }

  function renderRow(tag: Tag, depth: number) {
    const prog = recurringByTagId.get(tag.id);

    if (editingTagId === tag.id) {
      return (
        <div
          key={tag.id}
          className="flex flex-wrap items-center gap-2 rounded-lg border p-2"
          style={{ borderColor: "var(--accent)", backgroundColor: "var(--surface-1)", marginLeft: `${depth * 1.5}rem` }}
        >
          <input value={editTagName} onChange={(e) => setEditTagName(e.target.value)} className="rounded border bg-transparent px-2 py-1 text-sm" style={selectStyle} />
          <input type="color" value={editTagColor} onChange={(e) => setEditTagColor(e.target.value)} className="h-8 w-10 rounded border bg-transparent" style={selectStyle} />
          <select
            value={editTagParentId}
            onChange={(e) => setEditTagParentId(e.target.value ? Number(e.target.value) : "")}
            className="rounded border bg-transparent px-2 py-1 text-sm"
            style={selectStyle}
          >
            <option value="">No parent (top-level)</option>
            {orderTagsHierarchically(tags)
              .filter((t) => t.id !== tag.id && !descendantTagIds(tag.id, tags).has(t.id))
              .map((t) => (
                <option key={t.id} value={t.id} style={{ color: "black" }}>{"— ".repeat(tagDepth(t, tags))}{t.name}</option>
              ))}
          </select>
          <button onClick={() => handleSaveTagEdit(tag)} className="rounded px-3 py-1 text-xs font-medium" style={{ backgroundColor: "var(--accent)", color: "white" }}>
            Save
          </button>
          <button onClick={() => setEditingTagId(null)} className="rounded border px-3 py-1 text-xs" style={selectStyle}>
            Cancel
          </button>
          {editTagError && <p className="w-full text-xs" style={{ color: "var(--status-critical)" }}>{editTagError}</p>}
        </div>
      );
    }

    const expanded = prog && expandedId === prog.recurring.id;
    const diffColor = prog && prog.difference_cents > 0 ? "var(--status-critical)" : "var(--status-good)";
    const isPaused = prog ? !prog.recurring.active : false;

    // At-a-glance payment status: green once paid, red if the cycle's due date has arrived
    // with nothing logged yet (a likely missed payment), white/default while still pending.
    // A paused item skips all that and just goes muted - it's not due, so there's no status to show.
    const nameColor = !prog
      ? "var(--text-primary)"
      : isPaused
        ? "var(--text-muted)"
        : prog.paid_cents !== 0
          ? "var(--status-good)"
          : todayIso() > prog.cycle_start
            ? "var(--status-critical)"
            : "var(--text-primary)";

    return (
      <div key={tag.id} className="rounded-lg border" style={{ borderColor: "var(--border)", backgroundColor: "var(--surface-1)", marginLeft: `${depth * 1.5}rem` }}>
        <div className="flex flex-wrap items-center gap-3 px-3 py-2">
          {prog ? (
            <button
              onClick={() => toggleExpand(prog)}
              className="text-xs"
              aria-label={expanded ? `Collapse ${tag.name}` : `Expand ${tag.name}`}
              style={{ color: "var(--text-muted)" }}
            >
              {expanded ? "▾" : "▸"}
            </button>
          ) : (
            <span className="w-3" />
          )}
          <span className="flex items-center gap-2 font-medium" style={{ color: nameColor }}>
            <span className="inline-block h-3 w-3 rounded-full" style={{ backgroundColor: tag.color ?? "#3987e5" }} />
            {tag.name}
            {isPaused && <span className="text-xs font-normal">(paused)</span>}
          </span>
          {prog && (
            <span className="text-xs" style={{ color: "var(--text-muted)" }}>
              {accountName(prog.recurring.account_id)} · every {prog.recurring.interval_count} {prog.recurring.interval_unit}
              {prog.recurring.interval_count > 1 ? "s" : ""} · cycle {formatDate(prog.cycle_start)} → {formatDate(prog.cycle_end)}
            </span>
          )}
          <span className="ml-auto flex items-center gap-3 text-sm" style={{ fontVariantNumeric: "tabular-nums" }}>
            {prog && (
              <>
                <span style={{ color: "var(--text-secondary)" }}>proj {formatCents(prog.recurring.projected_amount_cents)}</span>
                <span style={{ color: "var(--text-secondary)" }}>paid {formatCents(prog.paid_cents)}</span>
                {prog.paid_cents !== 0 && (
                  <span style={{ color: diffColor }}>diff {prog.difference_cents >= 0 ? "+" : ""}{formatCents(prog.difference_cents)}</span>
                )}
              </>
            )}
            {budgetByTagId.has(tag.id) ? (
              <span style={{ color: "var(--text-muted)" }}>budget {formatCents(budgetByTagId.get(tag.id)!.amount_cents)}</span>
            ) : (
              <button
                onClick={() => startGenerateBudget(tag.id)}
                title="Generate budget for this category"
                aria-label={`Generate budget for ${tag.name}`}
                style={{ color: "var(--accent)" }}
              >
                ▦
              </button>
            )}
            <button onClick={() => startEditTag(tag)} title="Edit tag" aria-label={`Edit ${tag.name}`} style={{ color: "var(--text-muted)" }}>✎</button>
            {prog ? (
              <button
                onClick={() => startEditRecurring(prog)}
                title="Edit recurring"
                aria-label={`Edit recurring for ${tag.name}`}
                style={{ color: "var(--text-muted)" }}
              >
                ⚙
              </button>
            ) : (
              <button
                onClick={() => startAddRecurringFor(tag.id)}
                title="Add recurring"
                aria-label={`Add recurring for ${tag.name}`}
                style={{ color: "var(--accent)" }}
              >
                +
              </button>
            )}
            {prog && (
              <button
                onClick={() => handleToggleActive(prog)}
                title={isPaused ? "Resume recurring" : "Pause recurring (excludes it from budget projections)"}
                aria-label={isPaused ? `Resume recurring for ${tag.name}` : `Pause recurring for ${tag.name}`}
                style={{ color: isPaused ? "var(--status-good)" : "var(--text-muted)" }}
              >
                {isPaused ? "▶" : "⏸"}
              </button>
            )}
            {prog && (
              <button
                onClick={() => handleDeleteRecurring(prog)}
                title="Remove recurring (keeps the tag)"
                aria-label={`Remove recurring for ${tag.name}`}
                style={{ color: "var(--status-warning)" }}
              >
                ⊘
              </button>
            )}
            <button onClick={() => handleDeleteTag(tag)} title="Delete tag" aria-label={`Delete ${tag.name}`} style={{ color: "var(--status-critical)" }}>✕</button>
          </span>
        </div>
        {expanded && prog && (
          <div className="border-t px-3 py-2" style={{ borderColor: "var(--gridline)" }}>
            {prog.recurring.notes && <p className="mb-2 text-sm" style={{ color: "var(--text-secondary)" }}>{prog.recurring.notes}</p>}
            <p className="mb-1 text-xs" style={{ color: "var(--text-muted)" }}>Transactions in this cycle:</p>
            {cycleTxns.length === 0 && <p className="text-sm" style={{ color: "var(--text-muted)" }}>None yet.</p>}
            <div className="flex flex-col gap-1">
              {cycleTxns.map((t) => (
                <div key={t.id} className="flex items-center justify-between text-sm">
                  <span style={{ color: "var(--text-secondary)" }}>{formatDate(t.date)} · {t.description}</span>
                  <span style={{ fontVariantNumeric: "tabular-nums", color: t.amount_cents < 0 ? "var(--status-critical)" : "var(--status-good)" }}>
                    {formatCents(t.amount_cents)}
                  </span>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>
    );
  }

  function renderTree(tag: Tag, depth: number) {
    return (
      <div key={tag.id} className="flex flex-col gap-2">
        {renderRow(tag, depth)}
        {childrenOf(tag.id).map((child) => renderTree(child, depth + 1))}
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-6">
      <form onSubmit={handleCreateCategory} className="flex flex-wrap items-end gap-3 rounded-lg border p-4" style={{ borderColor: "var(--border)", backgroundColor: "var(--surface-1)" }}>
        <div className="flex flex-col gap-1">
          <label className="text-xs" style={{ color: "var(--text-muted)" }}>New category name</label>
          <input value={catName} onChange={(e) => setCatName(e.target.value)} placeholder="Groceries" className="rounded border bg-transparent px-2 py-1 text-sm" style={selectStyle} />
        </div>
        <div className="flex flex-col gap-1">
          <label className="text-xs" style={{ color: "var(--text-muted)" }}>Color</label>
          <input type="color" value={catColor} onChange={(e) => setCatColor(e.target.value)} className="h-8 w-10 rounded border bg-transparent" style={selectStyle} />
        </div>
        <div className="flex flex-col gap-1">
          <label className="text-xs" style={{ color: "var(--text-muted)" }}>Parent (optional)</label>
          <select value={catParentId} onChange={(e) => setCatParentId(e.target.value ? Number(e.target.value) : "")} className="rounded border bg-transparent px-2 py-1 text-sm" style={selectStyle}>
            <option value="">No parent (top-level)</option>
            {orderTagsHierarchically(tags).map((t) => (
              <option key={t.id} value={t.id} style={{ color: "black" }}>{"— ".repeat(tagDepth(t, tags))}{t.name}</option>
            ))}
          </select>
        </div>
        <button type="submit" disabled={catSubmitting} className="rounded px-3 py-1.5 text-sm font-medium" style={{ backgroundColor: "var(--accent)", color: "white" }}>
          + Add category
        </button>
        {!showRecurringForm && (
          <button type="button" onClick={() => setShowRecurringForm(true)} className="rounded border px-3 py-1.5 text-sm font-medium" style={selectStyle}>
            + Add recurring
          </button>
        )}
        {catError && <p className="w-full text-xs" style={{ color: "var(--status-critical)" }}>{catError}</p>}
      </form>

      {showRecurringForm && (
        <form onSubmit={handleSubmitRecurring} className="flex flex-col gap-3 rounded-lg border p-4" style={{ borderColor: "var(--accent)", backgroundColor: "var(--surface-1)" }}>
          <div className="flex gap-2">
            <button
              type="button"
              onClick={() => setTagMode("existing")}
              className="rounded px-3 py-1 text-xs font-medium"
              style={{ backgroundColor: tagMode === "existing" ? "var(--accent)" : "transparent", color: tagMode === "existing" ? "white" : "var(--text-secondary)", border: tagMode === "existing" ? "none" : "1px solid var(--border)" }}
            >
              Existing tag
            </button>
            <button
              type="button"
              onClick={() => setTagMode("new")}
              className="rounded px-3 py-1 text-xs font-medium"
              style={{ backgroundColor: tagMode === "new" ? "var(--accent)" : "transparent", color: tagMode === "new" ? "white" : "var(--text-secondary)", border: tagMode === "new" ? "none" : "1px solid var(--border)" }}
            >
              New tag
            </button>
          </div>

          {tagMode === "existing" ? (
            <div className="flex flex-col gap-1">
              <label className="text-xs" style={{ color: "var(--text-muted)" }}>Tag</label>
              <select value={recurTagId} onChange={(e) => setRecurTagId(e.target.value ? Number(e.target.value) : "")} className="w-64 rounded border bg-transparent px-2 py-1 text-sm" style={selectStyle}>
                <option value="">Select…</option>
                {orderTagsHierarchically(editingRecurringId !== null ? tags : tagsWithoutRecurring).map((t) => (
                  <option key={t.id} value={t.id} style={{ color: "black" }}>{t.parent_id ? `— ${t.name}` : t.name}</option>
                ))}
              </select>
            </div>
          ) : (
            <div className="flex flex-wrap items-end gap-3">
              <div className="flex flex-col gap-1">
                <label className="text-xs" style={{ color: "var(--text-muted)" }}>New tag name</label>
                <input value={newTagName} onChange={(e) => setNewTagName(e.target.value)} placeholder="Water" className="rounded border bg-transparent px-2 py-1 text-sm" style={selectStyle} />
              </div>
              <div className="flex flex-col gap-1">
                <label className="text-xs" style={{ color: "var(--text-muted)" }}>Color</label>
                <input type="color" value={newTagColor} onChange={(e) => setNewTagColor(e.target.value)} className="h-8 w-10 rounded border bg-transparent" style={selectStyle} />
              </div>
              <div className="flex flex-col gap-1">
                <label className="text-xs" style={{ color: "var(--text-muted)" }}>Parent (optional)</label>
                <select value={newTagParentId} onChange={(e) => setNewTagParentId(e.target.value ? Number(e.target.value) : "")} className="rounded border bg-transparent px-2 py-1 text-sm" style={selectStyle}>
                  <option value="">No parent (top-level)</option>
                  {orderTagsHierarchically(tags).map((t) => (
                    <option key={t.id} value={t.id} style={{ color: "black" }}>{"— ".repeat(tagDepth(t, tags))}{t.name}</option>
                  ))}
                </select>
              </div>
            </div>
          )}

          <div className="flex flex-wrap items-end gap-3">
            <div className="flex flex-col gap-1">
              <label className="text-xs" style={{ color: "var(--text-muted)" }}>Account</label>
              <select value={recurAccountId} onChange={(e) => setRecurAccountId(e.target.value ? Number(e.target.value) : "")} className="rounded border bg-transparent px-2 py-1 text-sm" style={selectStyle}>
                <option value="">Any account</option>
                {accounts.map((a) => (
                  <option key={a.id} value={a.id} style={{ color: "black" }}>{a.name}</option>
                ))}
              </select>
            </div>
            <div className="flex flex-col gap-1">
              <label className="text-xs" style={{ color: "var(--text-muted)" }}>Type</label>
              <select value={recurKind} onChange={(e) => setRecurKind(e.target.value as "expense" | "income")} className="rounded border bg-transparent px-2 py-1 text-sm" style={selectStyle}>
                <option value="expense" style={{ color: "black" }}>expense</option>
                <option value="income" style={{ color: "black" }}>income</option>
              </select>
            </div>
            <div className="flex flex-col gap-1">
              <label className="text-xs" style={{ color: "var(--text-muted)" }}>Projected amount</label>
              <input type="number" step="0.01" value={projectedAmount} onChange={(e) => setProjectedAmount(e.target.value)} className="w-32 rounded border bg-transparent px-2 py-1 text-sm" style={selectStyle} />
            </div>
          </div>

          <div className="flex flex-wrap items-end gap-3">
            <div className="flex flex-col gap-1">
              <label className="text-xs" style={{ color: "var(--text-muted)" }}>Every</label>
              <input type="number" min="1" value={intervalCount} onChange={(e) => setIntervalCount(e.target.value)} className="w-16 rounded border bg-transparent px-2 py-1 text-sm" style={selectStyle} />
            </div>
            <div className="flex flex-col gap-1">
              <label className="text-xs" style={{ color: "var(--text-muted)" }}>Unit</label>
              <select value={intervalUnit} onChange={(e) => setIntervalUnit(e.target.value as IntervalUnit)} className="rounded border bg-transparent px-2 py-1 text-sm" style={selectStyle}>
                <option value="day" style={{ color: "black" }}>day(s)</option>
                <option value="week" style={{ color: "black" }}>week(s)</option>
                <option value="month" style={{ color: "black" }}>month(s)</option>
                <option value="year" style={{ color: "black" }}>year(s)</option>
              </select>
            </div>
            <div className="flex flex-col gap-1">
              <label className="text-xs" style={{ color: "var(--text-muted)" }}>Date of payment</label>
              <input type="date" value={anchorDate} onChange={(e) => setAnchorDate(e.target.value)} className="rounded border bg-transparent px-2 py-1 text-sm" style={selectStyle} />
            </div>
            <div className="flex flex-1 flex-col gap-1">
              <label className="text-xs" style={{ color: "var(--text-muted)" }}>Notes (optional)</label>
              <input value={recurNotes} onChange={(e) => setRecurNotes(e.target.value)} className="w-full rounded border bg-transparent px-2 py-1 text-sm" style={selectStyle} />
            </div>
          </div>

          {recurError && <p className="text-xs" style={{ color: "var(--status-critical)" }}>{recurError}</p>}

          <div className="flex gap-2">
            <button type="submit" disabled={recurSubmitting} className="self-start rounded px-4 py-1.5 text-sm font-medium" style={{ backgroundColor: "var(--accent)", color: "white" }}>
              {recurSubmitting ? "Saving…" : editingRecurringId !== null ? "Save changes" : "Add recurring"}
            </button>
            <button type="button" onClick={resetRecurringForm} className="self-start rounded border px-4 py-1.5 text-sm" style={selectStyle}>
              Cancel
            </button>
          </div>
        </form>
      )}

      {budgetFormTagId !== null && (
        <form
          onSubmit={handleSubmitBudget}
          className="flex flex-wrap items-end gap-3 rounded-lg border p-4"
          style={{ borderColor: "var(--accent)", backgroundColor: "var(--surface-1)" }}
        >
          <p className="w-full text-sm font-medium">
            Generate budget for {tags.find((t) => t.id === budgetFormTagId)?.name}
          </p>
          <div className="flex flex-col gap-1">
            <label className="text-xs" style={{ color: "var(--text-muted)" }}>Account (blank = any)</label>
            <select
              value={budgetAccountId}
              onChange={(e) => setBudgetAccountId(e.target.value ? Number(e.target.value) : "")}
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
              value={budgetAmount}
              onChange={(e) => {
                setBudgetAmount(e.target.value);
                setBudgetSuggestionCount(0);
              }}
              className="w-32 rounded border bg-transparent px-2 py-1 text-sm"
              style={selectStyle}
            />
          </div>
          <label className="flex items-center gap-2 pb-1.5 text-xs" style={{ color: "var(--text-secondary)" }}>
            <input type="checkbox" checked={budgetShowOnDashboard} onChange={(e) => setBudgetShowOnDashboard(e.target.checked)} />
            Show on dashboard
          </label>
          <button type="submit" disabled={budgetSubmitting} className="rounded px-3 py-1.5 text-sm font-medium" style={{ backgroundColor: "var(--accent)", color: "white" }}>
            Create budget
          </button>
          <button type="button" onClick={resetBudgetForm} className="rounded border px-3 py-1.5 text-sm" style={selectStyle}>
            Cancel
          </button>
          {budgetSuggestionCount > 0 && (
            <p className="w-full text-xs" style={{ color: "var(--text-muted)" }}>
              Amount auto-filled from {budgetSuggestionCount} recurring item{budgetSuggestionCount > 1 ? "s" : ""} under this category - adjust if you want buffer.
            </p>
          )}
        </form>
      )}

      <div className="flex flex-col gap-2">
        {topLevel.map((parent) => renderTree(parent, 0))}
        {tags.length === 0 && <p className="text-sm" style={{ color: "var(--text-muted)" }}>No categories yet.</p>}
      </div>

      <div className="border-t pt-8" style={{ borderColor: "var(--gridline)" }}>
        <h2 className="mb-4 text-lg font-semibold">Budgets</h2>
        <BudgetsView accounts={accounts} tags={tags} recurring={recurring} year={year} month={month} dataVersion={dataVersion} bump={bump} />
      </div>
    </div>
  );
}
