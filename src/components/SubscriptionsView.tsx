import { useState } from "react";
import type { Account, IntervalUnit, Subscription, Tag } from "../api";
import {
  cancelSubscription,
  createSubscription,
  pauseSubscription,
  reactivateSubscription,
  updateSubscription,
} from "../api";
import { formatCents, todayIso } from "../utils";
import TagPicker from "./TagPicker";

interface Props {
  accounts: Account[];
  tags: Tag[];
  subscriptions: Subscription[];
  onTagCreated: (tag: Tag) => void;
  bump: () => void;
}

export default function SubscriptionsView({ accounts, tags, subscriptions, onTagCreated, bump }: Props) {
  const defaultAccountId = accounts.find((a) => a.is_default)?.id ?? "";
  const [editingId, setEditingId] = useState<number | null>(null);
  const [name, setName] = useState("");
  const [accountId, setAccountId] = useState<number | "">(defaultAccountId);
  const [amount, setAmount] = useState("");
  const [kind, setKind] = useState<"expense" | "income">("expense");
  const [intervalUnit, setIntervalUnit] = useState<IntervalUnit>("month");
  const [intervalCount, setIntervalCount] = useState("1");
  const [startDate, setStartDate] = useState(todayIso());
  const [tagIds, setTagIds] = useState<number[]>([]);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState("");
  const [reactivateDates, setReactivateDates] = useState<Record<number, string>>({});

  const accountName = (id: number) => accounts.find((a) => a.id === id)?.name ?? `#${id}`;

  function resetForm() {
    setEditingId(null);
    setName("");
    setAccountId(defaultAccountId);
    setAmount("");
    setKind("expense");
    setIntervalUnit("month");
    setIntervalCount("1");
    setStartDate(todayIso());
    setTagIds([]);
  }

  function startEdit(s: Subscription) {
    setEditingId(s.id);
    setName(s.name);
    setAccountId(s.account_id);
    setAmount((Math.abs(s.amount_cents) / 100).toString());
    setKind(s.type);
    setIntervalUnit(s.interval_unit);
    setIntervalCount(String(s.interval_count));
    setTagIds(s.tags.map((t) => t.id));
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setError("");
    const cents = Math.round(parseFloat(amount || "0") * 100);
    if (!name.trim() || !cents || !accountId) {
      setError("Name, amount, and account are required.");
      return;
    }
    setSubmitting(true);
    try {
      if (editingId !== null) {
        await updateSubscription({
          id: editingId,
          accountId: Number(accountId),
          name: name.trim(),
          amountCents: cents,
          intervalUnit,
          intervalCount: Number(intervalCount) || 1,
          tagIds,
        });
      } else {
        await createSubscription({
          accountId: Number(accountId),
          name: name.trim(),
          amountCents: cents,
          kind,
          intervalUnit,
          intervalCount: Number(intervalCount) || 1,
          startDate,
          tagIds,
        });
      }
      resetForm();
      bump();
    } catch (err) {
      setError(String(err));
    } finally {
      setSubmitting(false);
    }
  }

  const active = subscriptions.filter((s) => s.active);
  const inactive = subscriptions.filter((s) => !s.active);

  const selectStyle = { borderColor: "var(--border)" };

  return (
    <div className="flex flex-col gap-6">
      <form
        onSubmit={handleSubmit}
        className="flex flex-col gap-3 rounded-lg border p-4"
        style={{ borderColor: "var(--border)", backgroundColor: "var(--surface-1)" }}
      >
        <div className="flex flex-wrap items-end gap-3">
          <div className="flex flex-1 flex-col gap-1">
            <label className="text-xs" style={{ color: "var(--text-muted)" }}>Name</label>
            <input
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="Netflix"
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
              className="w-28 rounded border bg-transparent px-2 py-1 text-sm"
              style={selectStyle}
            />
          </div>
          <div className="flex flex-col gap-1">
            <label className="text-xs" style={{ color: "var(--text-muted)" }}>Type</label>
            <select
              value={kind}
              onChange={(e) => setKind(e.target.value as "expense" | "income")}
              disabled={editingId !== null}
              className="rounded border bg-transparent px-2 py-1 text-sm disabled:opacity-50"
              style={selectStyle}
            >
              <option value="expense" style={{ color: "black" }}>expense</option>
              <option value="income" style={{ color: "black" }}>income</option>
            </select>
          </div>
          <div className="flex flex-col gap-1">
            <label className="text-xs" style={{ color: "var(--text-muted)" }}>Account</label>
            <select value={accountId} onChange={(e) => setAccountId(e.target.value ? Number(e.target.value) : "")} className="rounded border bg-transparent px-2 py-1 text-sm" style={selectStyle}>
              <option value="">Select…</option>
              {accounts.map((a) => (
                <option key={a.id} value={a.id} style={{ color: "black" }}>{a.name}</option>
              ))}
            </select>
          </div>
        </div>

        <div className="flex flex-wrap items-end gap-3">
          <div className="flex flex-col gap-1">
            <label className="text-xs" style={{ color: "var(--text-muted)" }}>Every</label>
            <input
              type="number"
              min="1"
              value={intervalCount}
              onChange={(e) => setIntervalCount(e.target.value)}
              className="w-16 rounded border bg-transparent px-2 py-1 text-sm"
              style={selectStyle}
            />
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
          {editingId === null && (
            <div className="flex flex-col gap-1">
              <label className="text-xs" style={{ color: "var(--text-muted)" }}>Start date</label>
              <input
                type="date"
                value={startDate}
                onChange={(e) => setStartDate(e.target.value)}
                className="rounded border bg-transparent px-2 py-1 text-sm"
                style={selectStyle}
              />
            </div>
          )}
        </div>

        <TagPicker tags={tags} selected={tagIds} onChange={setTagIds} onTagCreated={onTagCreated} />

        {error && <p className="text-xs" style={{ color: "var(--status-critical)" }}>{error}</p>}

        <div className="flex gap-2">
          <button
            type="submit"
            disabled={submitting}
            className="self-start rounded px-4 py-1.5 text-sm font-medium"
            style={{ backgroundColor: "var(--accent)", color: "white" }}
          >
            {submitting ? "Saving…" : editingId !== null ? "Save changes" : "Add subscription"}
          </button>
          {editingId !== null && (
            <button
              type="button"
              onClick={resetForm}
              className="self-start rounded border px-4 py-1.5 text-sm"
              style={selectStyle}
            >
              Cancel
            </button>
          )}
        </div>
      </form>

      <div>
        <h3 className="mb-2 text-sm font-medium" style={{ color: "var(--text-muted)" }}>Active</h3>
        <div className="flex flex-col gap-2">
          {active.map((s) => (
            <div
              key={s.id}
              className="flex flex-wrap items-center justify-between gap-3 rounded-lg border p-3"
              style={{ borderColor: "var(--border)", backgroundColor: "var(--surface-1)" }}
            >
              <div>
                <p className="font-medium">{s.name}</p>
                <p className="text-xs" style={{ color: "var(--text-secondary)" }}>
                  {formatCents(Math.abs(s.amount_cents))} every {s.interval_count} {s.interval_unit}
                  {s.interval_count > 1 ? "s" : ""} · next {s.next_charge_date} · {accountName(s.account_id)}
                </p>
                <div className="mt-1 flex flex-wrap gap-1">
                  {s.tags.map((t) => (
                    <span key={t.id} className="rounded-full px-2 py-0.5 text-xs" style={{ backgroundColor: `${t.color ?? "#3987e5"}22`, color: "var(--text-secondary)" }}>
                      {t.name}
                    </span>
                  ))}
                </div>
              </div>
              <div className="flex gap-2">
                <button
                  onClick={() => startEdit(s)}
                  className="rounded border px-3 py-1 text-xs"
                  style={{ borderColor: "var(--border)" }}
                >
                  Edit
                </button>
                <button
                  onClick={async () => {
                    await pauseSubscription(s.id);
                    bump();
                  }}
                  className="rounded border px-3 py-1 text-xs"
                  style={{ borderColor: "var(--border)" }}
                >
                  Pause
                </button>
                <button
                  onClick={async () => {
                    await cancelSubscription(s.id);
                    bump();
                  }}
                  className="rounded border px-3 py-1 text-xs"
                  style={{ borderColor: "var(--status-critical)", color: "var(--status-critical)" }}
                >
                  Cancel
                </button>
              </div>
            </div>
          ))}
          {active.length === 0 && <p className="text-sm" style={{ color: "var(--text-muted)" }}>No active subscriptions.</p>}
        </div>
      </div>

      {inactive.length > 0 && (
        <div>
          <h3 className="mb-2 text-sm font-medium" style={{ color: "var(--text-muted)" }}>Paused / cancelled</h3>
          <div className="flex flex-col gap-2">
            {inactive.map((s) => (
              <div
                key={s.id}
                className="flex flex-wrap items-center justify-between gap-3 rounded-lg border p-3"
                style={{ borderColor: "var(--border)", backgroundColor: "var(--surface-1)" }}
              >
                <div>
                  <p className="font-medium">{s.name}</p>
                  <p className="text-xs" style={{ color: "var(--text-muted)" }}>
                    {s.paused_until ? `Paused - check in after ${s.paused_until}` : "Cancelled"} · {accountName(s.account_id)}
                  </p>
                </div>
                <div className="flex items-center gap-2">
                  <input
                    type="date"
                    value={reactivateDates[s.id] ?? ""}
                    onChange={(e) => setReactivateDates((prev) => ({ ...prev, [s.id]: e.target.value }))}
                    className="rounded border bg-transparent px-2 py-1 text-xs"
                    style={{ borderColor: "var(--border)" }}
                    title="Optional: new next charge date"
                  />
                  <button
                    onClick={async () => {
                      await reactivateSubscription(s.id, reactivateDates[s.id] || null);
                      bump();
                    }}
                    className="rounded px-3 py-1 text-xs font-medium"
                    style={{ backgroundColor: "var(--status-good)", color: "white" }}
                  >
                    Resume
                  </button>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
