import { useState } from "react";
import type { Account, Tag, Transaction } from "../api";
import { updateTransaction } from "../api";
import TagPicker from "./TagPicker";

interface Props {
  transaction: Transaction;
  accounts: Account[];
  tags: Tag[];
  onTagCreated: (tag: Tag) => void;
  onSaved: () => void;
  onCancel: () => void;
}

export default function EditTransactionForm({ transaction, accounts, tags, onTagCreated, onSaved, onCancel }: Props) {
  const [accountId, setAccountId] = useState(transaction.account_id);
  const [date, setDate] = useState(transaction.date);
  const [amount, setAmount] = useState((Math.abs(transaction.amount_cents) / 100).toString());
  const [kind, setKind] = useState(transaction.type);
  const [description, setDescription] = useState(transaction.description);
  const [notes, setNotes] = useState(transaction.notes ?? "");
  const [tagIds, setTagIds] = useState<number[]>(transaction.tags.map((t) => t.id));
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState("");

  const selectStyle = { borderColor: "var(--border)" };
  const isLinked = transaction.linked_group_id !== null;

  // Title auto-fills from a newly-picked tag only while it's still blank.
  function handleTagChange(newIds: number[]) {
    const addedId = newIds.find((id) => !tagIds.includes(id));
    if (addedId && description.trim() === "") {
      const tag = tags.find((t) => t.id === addedId);
      if (tag) setDescription(tag.name);
    }
    setTagIds(newIds);
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setError("");
    const cents = Math.round(parseFloat(amount || "0") * 100);
    if (!cents || !description.trim()) {
      setError("Amount and description are required.");
      return;
    }
    setSubmitting(true);
    try {
      await updateTransaction({
        id: transaction.id,
        accountId,
        date,
        amountCents: cents,
        kind,
        description: description.trim(),
        notes: notes.trim() || null,
        tagIds,
      });
      onSaved();
    } catch (err) {
      setError(String(err));
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <form
      onSubmit={handleSubmit}
      className="flex flex-col gap-3 rounded-lg border p-4"
      style={{ borderColor: "var(--accent)", backgroundColor: "var(--surface-1)" }}
    >
      <div className="flex items-center justify-between">
        <p className="text-sm font-medium">Edit entry</p>
        {isLinked && (
          <p className="text-xs" style={{ color: "var(--text-muted)" }}>
            Part of a linked pair - only this side will change.
          </p>
        )}
      </div>
      <div className="flex flex-wrap items-end gap-3">
        <div className="flex flex-col gap-1">
          <label className="text-xs" style={{ color: "var(--text-muted)" }}>Date</label>
          <input type="date" value={date} onChange={(e) => setDate(e.target.value)} className="rounded border bg-transparent px-2 py-1 text-sm" style={selectStyle} />
        </div>
        <div className="flex flex-col gap-1">
          <label className="text-xs" style={{ color: "var(--text-muted)" }}>Amount</label>
          <input type="number" step="0.01" value={amount} onChange={(e) => setAmount(e.target.value)} className="w-28 rounded border bg-transparent px-2 py-1 text-sm" style={selectStyle} />
        </div>
        <div className="flex flex-1 flex-col gap-1">
          <label className="text-xs" style={{ color: "var(--text-muted)" }}>Title</label>
          <input value={description} onChange={(e) => setDescription(e.target.value)} className="w-full rounded border bg-transparent px-2 py-1 text-sm" style={selectStyle} />
        </div>
      </div>
      <div className="flex flex-col gap-1">
        <label className="text-xs" style={{ color: "var(--text-muted)" }}>Notes (optional)</label>
        <textarea
          value={notes}
          onChange={(e) => setNotes(e.target.value)}
          rows={2}
          className="w-full rounded border bg-transparent px-2 py-1 text-sm"
          style={selectStyle}
        />
      </div>
      <div className="flex flex-wrap items-end gap-3">
        <div className="flex flex-col gap-1">
          <label className="text-xs" style={{ color: "var(--text-muted)" }}>Account</label>
          <select value={accountId} onChange={(e) => setAccountId(Number(e.target.value))} className="rounded border bg-transparent px-2 py-1 text-sm" style={selectStyle}>
            {accounts.map((a) => (
              <option key={a.id} value={a.id} style={{ color: "black" }}>{a.name}</option>
            ))}
          </select>
        </div>
        {!isLinked && (
          <div className="flex flex-col gap-1">
            <label className="text-xs" style={{ color: "var(--text-muted)" }}>Type</label>
            <select value={kind} onChange={(e) => setKind(e.target.value as "expense" | "income")} className="rounded border bg-transparent px-2 py-1 text-sm" style={selectStyle}>
              <option value="expense" style={{ color: "black" }}>expense</option>
              <option value="income" style={{ color: "black" }}>income</option>
            </select>
          </div>
        )}
      </div>
      <TagPicker tags={tags} selected={tagIds} onChange={handleTagChange} onTagCreated={onTagCreated} />
      {error && <p className="text-xs" style={{ color: "var(--status-critical)" }}>{error}</p>}
      <div className="flex gap-2">
        <button type="submit" disabled={submitting} className="rounded px-4 py-1.5 text-sm font-medium" style={{ backgroundColor: "var(--accent)", color: "white" }}>
          {submitting ? "Saving…" : "Save changes"}
        </button>
        <button type="button" onClick={onCancel} className="rounded border px-4 py-1.5 text-sm" style={selectStyle}>
          Cancel
        </button>
      </div>
    </form>
  );
}
