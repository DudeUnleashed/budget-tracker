import { useEffect, useState } from "react";
import type { Account } from "../api";
import { createAccount, deleteAccount, getAccountBalance, setDefaultAccount, updateAccount } from "../api";
import { formatCents } from "../utils";

interface Props {
  accounts: Account[];
  onAccountCreated: (account: Account) => void;
  bump: () => void;
}

export default function AccountsView({ accounts, onAccountCreated, bump }: Props) {
  const [name, setName] = useState("");
  const [ledger, setLedger] = useState("personal");
  const [startingBalance, setStartingBalance] = useState("0");
  const [balances, setBalances] = useState<Record<number, number>>({});
  const [submitting, setSubmitting] = useState(false);
  const [editingId, setEditingId] = useState<number | null>(null);
  const [editName, setEditName] = useState("");
  const [editLedger, setEditLedger] = useState("personal");
  const [editBalance, setEditBalance] = useState("0");
  const [deleteErrors, setDeleteErrors] = useState<Record<number, string>>({});

  useEffect(() => {
    let cancelled = false;
    (async () => {
      const entries = await Promise.all(
        accounts.map(async (a) => [a.id, await getAccountBalance(a.id)] as const),
      );
      if (!cancelled) setBalances(Object.fromEntries(entries));
    })();
    return () => {
      cancelled = true;
    };
  }, [accounts]);

  async function handleCreate(e: React.FormEvent) {
    e.preventDefault();
    if (!name.trim()) return;
    setSubmitting(true);
    try {
      const cents = Math.round(parseFloat(startingBalance || "0") * 100);
      const account = await createAccount(name.trim(), ledger, cents);
      onAccountCreated(account);
      setName("");
      setStartingBalance("0");
    } finally {
      setSubmitting(false);
    }
  }

  function startEdit(a: Account) {
    setEditingId(a.id);
    setEditName(a.name);
    setEditLedger(a.ledger);
    setEditBalance(((balances[a.id] ?? a.starting_balance_cents) / 100).toString());
  }

  async function handleSaveEdit(a: Account) {
    // Editing "current balance" adjusts the starting-balance anchor by the same delta,
    // so activity recorded since account creation is preserved.
    const currentComputed = balances[a.id] ?? a.starting_balance_cents;
    const desiredCurrent = Math.round(parseFloat(editBalance || "0") * 100);
    const delta = desiredCurrent - currentComputed;
    const newStartingBalance = a.starting_balance_cents + delta;
    await updateAccount(a.id, editName.trim(), editLedger, newStartingBalance);
    setEditingId(null);
    bump();
  }

  async function handleSetDefault(id: number) {
    await setDefaultAccount(id);
    bump();
  }

  async function handleDelete(id: number, name: string) {
    if (!window.confirm(`Delete "${name}"? This can't be undone.`)) return;
    try {
      await deleteAccount(id);
      setDeleteErrors((prev) => {
        const next = { ...prev };
        delete next[id];
        return next;
      });
      bump();
    } catch (err) {
      setDeleteErrors((prev) => ({ ...prev, [id]: String(err) }));
    }
  }

  const inputStyle = { borderColor: "var(--border)" };

  return (
    <div className="flex flex-col gap-6">
      <form
        onSubmit={handleCreate}
        className="flex flex-wrap items-end gap-3 rounded-lg border p-4"
        style={{ borderColor: "var(--border)", backgroundColor: "var(--surface-1)" }}
      >
        <div className="flex flex-col gap-1">
          <label className="text-xs" style={{ color: "var(--text-muted)" }}>Account name</label>
          <input
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="Personal Checking"
            className="rounded border bg-transparent px-2 py-1 text-sm"
            style={inputStyle}
          />
        </div>
        <div className="flex flex-col gap-1">
          <label className="text-xs" style={{ color: "var(--text-muted)" }}>Ledger</label>
          <select
            value={ledger}
            onChange={(e) => setLedger(e.target.value)}
            className="rounded border bg-transparent px-2 py-1 text-sm"
            style={inputStyle}
          >
            <option value="personal" style={{ color: "black" }}>personal</option>
            <option value="business" style={{ color: "black" }}>business</option>
          </select>
        </div>
        <div className="flex flex-col gap-1">
          <label className="text-xs" style={{ color: "var(--text-muted)" }}>Starting balance</label>
          <input
            value={startingBalance}
            onChange={(e) => setStartingBalance(e.target.value)}
            type="number"
            step="0.01"
            className="w-32 rounded border bg-transparent px-2 py-1 text-sm"
            style={inputStyle}
          />
        </div>
        <button
          type="submit"
          disabled={submitting}
          className="rounded px-3 py-1.5 text-sm font-medium"
          style={{ backgroundColor: "var(--accent)", color: "white" }}
        >
          Add account
        </button>
      </form>

      <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
        {accounts.map((a) => (
          <div
            key={a.id}
            className="rounded-lg border p-4"
            style={{ borderColor: "var(--border)", backgroundColor: "var(--surface-1)" }}
          >
            {editingId === a.id ? (
              <div className="flex flex-col gap-2">
                <input
                  value={editName}
                  onChange={(e) => setEditName(e.target.value)}
                  className="rounded border bg-transparent px-2 py-1 text-sm"
                  style={inputStyle}
                />
                <select
                  value={editLedger}
                  onChange={(e) => setEditLedger(e.target.value)}
                  className="rounded border bg-transparent px-2 py-1 text-sm"
                  style={inputStyle}
                >
                  <option value="personal" style={{ color: "black" }}>personal</option>
                  <option value="business" style={{ color: "black" }}>business</option>
                </select>
                <label className="text-xs" style={{ color: "var(--text-muted)" }}>Current balance</label>
                <input
                  value={editBalance}
                  onChange={(e) => setEditBalance(e.target.value)}
                  type="number"
                  step="0.01"
                  className="rounded border bg-transparent px-2 py-1 text-sm"
                  style={inputStyle}
                />
                <div className="flex gap-2">
                  <button
                    onClick={() => handleSaveEdit(a)}
                    className="rounded px-3 py-1 text-xs font-medium"
                    style={{ backgroundColor: "var(--accent)", color: "white" }}
                  >
                    Save
                  </button>
                  <button
                    onClick={() => setEditingId(null)}
                    className="rounded border px-3 py-1 text-xs"
                    style={inputStyle}
                  >
                    Cancel
                  </button>
                </div>
              </div>
            ) : (
              <>
                <div className="flex items-center justify-between">
                  <span className="flex items-center gap-1.5 font-medium">
                    {a.name}
                    {a.is_default && (
                      <span title="Default account" style={{ color: "var(--status-warning)" }}>★</span>
                    )}
                  </span>
                  <div className="flex items-center gap-2">
                    <span
                      className="rounded-full px-2 py-0.5 text-xs uppercase"
                      style={{ color: "var(--text-muted)", border: "1px solid var(--border)" }}
                    >
                      {a.ledger}
                    </span>
                    <button
                      onClick={() => startEdit(a)}
                      className="text-xs"
                      style={{ color: "var(--text-muted)" }}
                    >
                      edit
                    </button>
                    <button
                      onClick={() => handleDelete(a.id, a.name)}
                      className="text-xs"
                      style={{ color: "var(--status-critical)" }}
                    >
                      delete
                    </button>
                  </div>
                </div>
                <p className="mt-2 text-2xl font-semibold">
                  {balances[a.id] !== undefined ? formatCents(balances[a.id]) : "…"}
                </p>
                {!a.is_default && (
                  <button
                    onClick={() => handleSetDefault(a.id)}
                    className="mt-2 text-xs"
                    style={{ color: "var(--text-muted)" }}
                  >
                    Set as default
                  </button>
                )}
                {deleteErrors[a.id] && (
                  <p className="mt-2 text-xs" style={{ color: "var(--status-critical)" }}>
                    {deleteErrors[a.id]}
                  </p>
                )}
              </>
            )}
          </div>
        ))}
        {accounts.length === 0 && (
          <p className="text-sm" style={{ color: "var(--text-muted)" }}>
            No accounts yet - add one above to get started.
          </p>
        )}
      </div>
    </div>
  );
}
