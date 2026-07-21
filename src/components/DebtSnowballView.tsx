import { useEffect, useState } from "react";
import type { DebtInput, DebtPayoffResult, DebtProgress } from "../api";
import { createDebt, deleteDebt, previewDebtPayoff, updateDebt } from "../api";
import { formatCents, formatDate, todayIso } from "../utils";

interface Props {
  debts: DebtProgress[];
  bump: () => void;
}

function centsToInput(cents: number): string {
  return (cents / 100).toString();
}
function inputToCents(v: string): number {
  return Math.round(parseFloat(v || "0") * 100);
}
function bpsToInput(bps: number): string {
  return (bps / 100).toString();
}
function inputToBps(v: string): number {
  return Math.round(parseFloat(v || "0") * 100);
}

function monthsLabel(months: number): string {
  const years = Math.floor(months / 12);
  const rem = months % 12;
  const parts: string[] = [];
  if (years > 0) parts.push(`${years} year${years > 1 ? "s" : ""}`);
  if (rem > 0 || years === 0) parts.push(`${rem} month${rem !== 1 ? "s" : ""}`);
  return parts.join(", ");
}

function PayoffSummary({ result }: { result: DebtPayoffResult }) {
  if (result.never_pays_off) {
    return (
      <p style={{ color: "var(--status-critical)" }}>
        This payment never gets ahead of the interest - the balance won't reach zero.
      </p>
    );
  }
  return (
    <p style={{ color: "var(--text-secondary)" }}>
      Paid off <strong>{result.payoff_date ? formatDate(result.payoff_date) : ""}</strong> ({monthsLabel(result.months)}) · total cost{" "}
      <strong>{formatCents(result.total_paid_cents)}</strong> · {formatCents(result.total_interest_cents)} in interest
    </p>
  );
}

function DebtForm({
  initial,
  onCancel,
  onSubmit,
  submitLabel,
}: {
  initial: DebtInput;
  onCancel?: () => void;
  onSubmit: (input: DebtInput) => Promise<void>;
  submitLabel: string;
}) {
  const [name, setName] = useState(initial.name);
  const [startDate, setStartDate] = useState(initial.startDate);
  const [principal, setPrincipal] = useState(centsToInput(initial.principalCents));
  const [payment, setPayment] = useState(centsToInput(initial.monthlyPaymentCents));
  const [rate, setRate] = useState(bpsToInput(initial.interestRateBps));
  const [preview, setPreview] = useState<DebtPayoffResult | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState("");

  const principalCents = inputToCents(principal);
  const monthlyPaymentCents = inputToCents(payment);
  const interestRateBps = inputToBps(rate);

  // Live "what if" preview as the pay amount or rate changes, before anything is saved - a plain
  // local IPC call is fast enough to feel instant, and keeps the payoff math in one place (Rust)
  // instead of duplicating it in TypeScript.
  useEffect(() => {
    if (!startDate || principalCents <= 0 || monthlyPaymentCents <= 0) {
      setPreview(null);
      return;
    }
    let cancelled = false;
    previewDebtPayoff({ name, startDate, principalCents, monthlyPaymentCents, interestRateBps }).then((r) => {
      if (!cancelled) setPreview(r);
    });
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [startDate, principalCents, monthlyPaymentCents, interestRateBps]);

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!name.trim() || principalCents <= 0 || monthlyPaymentCents <= 0) {
      setError("Name, amount owed, and monthly payment are required.");
      return;
    }
    setError("");
    setSubmitting(true);
    try {
      await onSubmit({ name: name.trim(), startDate, principalCents, monthlyPaymentCents, interestRateBps });
    } catch (err) {
      setError(String(err));
    } finally {
      setSubmitting(false);
    }
  }

  const selectStyle = { borderColor: "var(--border)" };

  return (
    <form onSubmit={handleSubmit} className="flex flex-col gap-3">
      <div className="flex flex-wrap items-end gap-3">
        <div className="flex flex-1 flex-col gap-1">
          <label className="text-xs" style={{ color: "var(--text-muted)" }}>Name</label>
          <input
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="Car loan"
            className="w-full rounded border bg-transparent px-2 py-1 text-sm"
            style={selectStyle}
          />
        </div>
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
      </div>
      <div className="flex flex-wrap items-end gap-3">
        <div className="flex flex-col gap-1">
          <label className="text-xs" style={{ color: "var(--text-muted)" }}>Amount owed</label>
          <input
            type="number"
            step="0.01"
            value={principal}
            onChange={(e) => setPrincipal(e.target.value)}
            className="w-32 rounded border bg-transparent px-2 py-1 text-sm"
            style={selectStyle}
          />
        </div>
        <div className="flex flex-col gap-1">
          <label className="text-xs" style={{ color: "var(--text-muted)" }}>Monthly payment</label>
          <input
            type="number"
            step="0.01"
            value={payment}
            onChange={(e) => setPayment(e.target.value)}
            className="w-32 rounded border bg-transparent px-2 py-1 text-sm"
            style={selectStyle}
          />
        </div>
        <div className="flex flex-col gap-1">
          <label className="text-xs" style={{ color: "var(--text-muted)" }}>Interest rate (APR %)</label>
          <input
            type="number"
            step="0.01"
            value={rate}
            onChange={(e) => setRate(e.target.value)}
            className="w-28 rounded border bg-transparent px-2 py-1 text-sm"
            style={selectStyle}
          />
        </div>
      </div>

      {preview && (
        <div className="rounded border p-2 text-sm" style={{ borderColor: "var(--border)", backgroundColor: "var(--page-plane)" }}>
          <PayoffSummary result={preview} />
        </div>
      )}

      {error && <p className="text-xs" style={{ color: "var(--status-critical)" }}>{error}</p>}

      <div className="flex gap-2">
        <button
          type="submit"
          disabled={submitting}
          className="self-start rounded px-4 py-1.5 text-sm font-medium"
          style={{ backgroundColor: "var(--accent)", color: "white" }}
        >
          {submitting ? "Saving…" : submitLabel}
        </button>
        {onCancel && (
          <button type="button" onClick={onCancel} className="self-start rounded border px-4 py-1.5 text-sm" style={selectStyle}>
            Cancel
          </button>
        )}
      </div>
    </form>
  );
}

export default function DebtSnowballView({ debts, bump }: Props) {
  const [showAddForm, setShowAddForm] = useState(false);
  const [editingId, setEditingId] = useState<number | null>(null);

  const blankInput: DebtInput = {
    name: "",
    startDate: todayIso(),
    principalCents: 0,
    monthlyPaymentCents: 0,
    interestRateBps: 0,
  };

  async function handleCreate(input: DebtInput) {
    await createDebt(input);
    setShowAddForm(false);
    bump();
  }

  async function handleUpdate(id: number, input: DebtInput) {
    await updateDebt(id, input);
    setEditingId(null);
    bump();
  }

  async function handleDelete(id: number, name: string) {
    if (!window.confirm(`Delete "${name}"? This can't be undone.`)) return;
    await deleteDebt(id);
    bump();
  }

  const cardStyle = { borderColor: "var(--border)", backgroundColor: "var(--surface-1)" };

  return (
    <div className="flex flex-col gap-6">
      <div className="flex items-center justify-between">
        <p className="text-sm" style={{ color: "var(--text-secondary)" }}>
          Standalone payoff calculators - no tags or accounts involved, just the numbers.
        </p>
        {!showAddForm && (
          <button
            onClick={() => setShowAddForm(true)}
            className="rounded px-3 py-1.5 text-sm font-medium"
            style={{ backgroundColor: "var(--accent)", color: "white" }}
          >
            + Add debt
          </button>
        )}
      </div>

      {showAddForm && (
        <div className="rounded-lg border p-4" style={cardStyle}>
          <DebtForm initial={blankInput} onSubmit={handleCreate} onCancel={() => setShowAddForm(false)} submitLabel="Add debt" />
        </div>
      )}

      <div className="flex flex-col gap-3">
        {debts.map((p) => (
          <div key={p.debt.id} className="rounded-lg border p-4" style={cardStyle}>
            {editingId === p.debt.id ? (
              <DebtForm
                initial={{
                  name: p.debt.name,
                  startDate: p.debt.start_date,
                  principalCents: p.debt.principal_cents,
                  monthlyPaymentCents: p.debt.monthly_payment_cents,
                  interestRateBps: p.debt.interest_rate_bps,
                }}
                onSubmit={(input) => handleUpdate(p.debt.id, input)}
                onCancel={() => setEditingId(null)}
                submitLabel="Save changes"
              />
            ) : (
              <>
                <div className="flex items-center justify-between gap-2">
                  <span className="font-medium">{p.debt.name}</span>
                  <div className="flex items-center gap-3 text-xs">
                    <button onClick={() => setEditingId(p.debt.id)} title="Edit" aria-label={`Edit ${p.debt.name}`} style={{ color: "var(--text-muted)" }}>✎</button>
                    <button onClick={() => handleDelete(p.debt.id, p.debt.name)} title="Delete" aria-label={`Delete ${p.debt.name}`} style={{ color: "var(--status-critical)" }}>✕</button>
                  </div>
                </div>
                <p className="mt-1 text-xs" style={{ color: "var(--text-muted)" }}>
                  {formatCents(p.debt.principal_cents)} owed from {formatDate(p.debt.start_date)} · {formatCents(p.debt.monthly_payment_cents)}/mo ·{" "}
                  {(p.debt.interest_rate_bps / 100).toFixed(2)}% APR
                </p>
                <div className="mt-2 text-sm">
                  <PayoffSummary
                    result={{
                      payoff_date: p.payoff_date,
                      months: p.months,
                      total_paid_cents: p.total_paid_cents,
                      total_interest_cents: p.total_interest_cents,
                      never_pays_off: p.never_pays_off,
                    }}
                  />
                </div>
              </>
            )}
          </div>
        ))}
        {debts.length === 0 && !showAddForm && (
          <p className="text-sm" style={{ color: "var(--text-muted)" }}>No debts yet - add one to project a payoff date.</p>
        )}
      </div>
    </div>
  );
}
