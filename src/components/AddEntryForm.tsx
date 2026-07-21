import { useState } from "react";
import type { Account, Tag } from "../api";
import { createAccountTransfer, createTransaction } from "../api";
import { todayIso } from "../utils";
import TagPicker from "./TagPicker";

type Mode = "transaction" | "transfer";

interface Props {
  accounts: Account[];
  tags: Tag[];
  onTagCreated: (tag: Tag) => void;
  onCreated: () => void;
}

export default function AddEntryForm({ accounts, tags, onTagCreated, onCreated }: Props) {
  const defaultAccountId = accounts.find((a) => a.is_default)?.id ?? "";
  const [mode, setMode] = useState<Mode>("transaction");
  const [date, setDate] = useState(todayIso());
  const [amount, setAmount] = useState("");
  const [description, setDescription] = useState("");
  const [notes, setNotes] = useState("");
  const [accountId, setAccountId] = useState<number | "">(defaultAccountId);
  const [fromAccountId, setFromAccountId] = useState<number | "">(defaultAccountId);
  const [toAccountId, setToAccountId] = useState<number | "">("");
  const [kind, setKind] = useState<"expense" | "income">("expense");
  const [tagIds, setTagIds] = useState<number[]>([]);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState("");

  function reset() {
    setAmount("");
    setDescription("");
    setNotes("");
    setTagIds([]);
  }

  // Title auto-fills from a newly-picked tag only while it's still blank, so a
  // hand-typed title (or one already set) is never clobbered.
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
      if (mode === "transaction") {
        if (!accountId) return setError("Pick an account.");
        await createTransaction({
          accountId: Number(accountId),
          date,
          amountCents: cents,
          kind,
          description: description.trim(),
          notes: notes.trim() || null,
          tagIds,
        });
      } else {
        if (!fromAccountId || !toAccountId) return setError("Pick both accounts.");
        if (fromAccountId === toAccountId) return setError("Pick two different accounts.");
        await createAccountTransfer({
          fromAccountId: Number(fromAccountId),
          toAccountId: Number(toAccountId),
          date,
          amountCents: cents,
          description: description.trim(),
          tagIds,
        });
      }
      reset();
      onCreated();
    } catch (err) {
      setError(String(err));
    } finally {
      setSubmitting(false);
    }
  }

  const selectStyle = { borderColor: "var(--border)" };

  return (
    <form
      onSubmit={handleSubmit}
      className="flex flex-col gap-3 rounded-lg border p-4"
      style={{ borderColor: "var(--border)", backgroundColor: "var(--surface-1)" }}
    >
      <div className="flex gap-2">
        {(["transaction", "transfer"] as Mode[]).map((m) => (
          <button
            type="button"
            key={m}
            onClick={() => setMode(m)}
            className="rounded px-3 py-1 text-xs font-medium"
            style={{
              backgroundColor: mode === m ? "var(--accent)" : "transparent",
              color: mode === m ? "white" : "var(--text-secondary)",
              border: mode === m ? "none" : "1px solid var(--border)",
            }}
          >
            {m === "transaction" ? "One-off" : "Account transfer"}
          </button>
        ))}
      </div>

      <div className="flex flex-wrap items-end gap-3">
        <div className="flex flex-col gap-1">
          <label className="text-xs" style={{ color: "var(--text-muted)" }}>Date</label>
          <input
            type="date"
            value={date}
            onChange={(e) => setDate(e.target.value)}
            className="rounded border bg-transparent px-2 py-1 text-sm"
            style={selectStyle}
          />
        </div>
        <div className="flex flex-col gap-1">
          <label className="text-xs" style={{ color: "var(--text-muted)" }}>Amount</label>
          <input
            type="number"
            step="0.01"
            value={amount}
            onChange={(e) => setAmount(e.target.value)}
            placeholder="0.00"
            className="w-28 rounded border bg-transparent px-2 py-1 text-sm"
            style={selectStyle}
          />
        </div>
        <div className="flex flex-1 flex-col gap-1">
          <label className="text-xs" style={{ color: "var(--text-muted)" }}>Title</label>
          <input
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            placeholder="Pick a tag to auto-fill, or type your own"
            className="w-full rounded border bg-transparent px-2 py-1 text-sm"
            style={selectStyle}
          />
        </div>
      </div>

      {mode === "transaction" && (
        <div className="flex flex-col gap-1">
          <label className="text-xs" style={{ color: "var(--text-muted)" }}>Notes (optional)</label>
          <textarea
            value={notes}
            onChange={(e) => setNotes(e.target.value)}
            placeholder="Anything more specific - e.g. what the gift was for"
            rows={2}
            className="w-full rounded border bg-transparent px-2 py-1 text-sm"
            style={selectStyle}
          />
        </div>
      )}

      {mode === "transaction" ? (
        <div className="flex flex-wrap items-end gap-3">
          <div className="flex flex-col gap-1">
            <label className="text-xs" style={{ color: "var(--text-muted)" }}>Account</label>
            <select
              value={accountId}
              onChange={(e) => setAccountId(e.target.value ? Number(e.target.value) : "")}
              className="rounded border bg-transparent px-2 py-1 text-sm"
              style={selectStyle}
            >
              <option value="">Select…</option>
              {accounts.map((a) => (
                <option key={a.id} value={a.id} style={{ color: "black" }}>{a.name}</option>
              ))}
            </select>
          </div>
          <div className="flex flex-col gap-1">
            <label className="text-xs" style={{ color: "var(--text-muted)" }}>Type</label>
            <select
              value={kind}
              onChange={(e) => setKind(e.target.value as "expense" | "income")}
              className="rounded border bg-transparent px-2 py-1 text-sm"
              style={selectStyle}
            >
              <option value="expense" style={{ color: "black" }}>expense</option>
              <option value="income" style={{ color: "black" }}>income</option>
            </select>
          </div>
        </div>
      ) : (
        <div className="flex flex-wrap items-end gap-3">
          <div className="flex flex-col gap-1">
            <label className="text-xs" style={{ color: "var(--text-muted)" }}>From account</label>
            <select
              value={fromAccountId}
              onChange={(e) => setFromAccountId(e.target.value ? Number(e.target.value) : "")}
              className="rounded border bg-transparent px-2 py-1 text-sm"
              style={selectStyle}
            >
              <option value="">Select…</option>
              {accounts.map((a) => (
                <option key={a.id} value={a.id} style={{ color: "black" }}>{a.name}</option>
              ))}
            </select>
          </div>
          <div className="flex flex-col gap-1">
            <label className="text-xs" style={{ color: "var(--text-muted)" }}>To account</label>
            <select
              value={toAccountId}
              onChange={(e) => setToAccountId(e.target.value ? Number(e.target.value) : "")}
              className="rounded border bg-transparent px-2 py-1 text-sm"
              style={selectStyle}
            >
              <option value="">Select…</option>
              {accounts.map((a) => (
                <option key={a.id} value={a.id} style={{ color: "black" }}>{a.name}</option>
              ))}
            </select>
          </div>
          <p className="text-xs" style={{ color: "var(--text-muted)" }}>
            Same ledger on both sides → a transfer (not counted). Different ledgers → counted as income/expense on each side.
          </p>
        </div>
      )}

      <TagPicker tags={tags} selected={tagIds} onChange={handleTagChange} onTagCreated={onTagCreated} />

      {error && <p className="text-xs" style={{ color: "var(--status-critical)" }}>{error}</p>}

      <button
        type="submit"
        disabled={submitting}
        className="self-start rounded px-4 py-1.5 text-sm font-medium"
        style={{ backgroundColor: "var(--accent)", color: "white" }}
      >
        {submitting ? "Saving…" : "Add"}
      </button>
    </form>
  );
}
