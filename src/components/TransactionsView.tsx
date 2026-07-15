import { useEffect, useState } from "react";
import type { Account, Tag, Transaction } from "../api";
import { deleteTransaction, searchTransactions } from "../api";
import { formatCents } from "../utils";
import AddEntryForm from "./AddEntryForm";
import EditTransactionForm from "./EditTransactionForm";

interface Props {
  accounts: Account[];
  tags: Tag[];
  onTagCreated: (tag: Tag) => void;
  dataVersion: number;
  bump: () => void;
}

export default function TransactionsView({ accounts, tags, onTagCreated, dataVersion, bump }: Props) {
  const [query, setQuery] = useState("");
  const [accountFilter, setAccountFilter] = useState<number | "">("");
  const [transactions, setTransactions] = useState<Transaction[]>([]);
  const [showAddForm, setShowAddForm] = useState(false);
  const [expandedIds, setExpandedIds] = useState<Set<number>>(new Set());
  const [editingId, setEditingId] = useState<number | null>(null);

  useEffect(() => {
    let cancelled = false;
    searchTransactions(query || null, accountFilter === "" ? null : Number(accountFilter)).then((r) => {
      if (!cancelled) setTransactions(r);
    });
    return () => {
      cancelled = true;
    };
  }, [query, accountFilter, dataVersion]);

  const accountName = (id: number) => accounts.find((a) => a.id === id)?.name ?? `#${id}`;

  function toggleExpanded(id: number) {
    setExpandedIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) {
        next.delete(id);
      } else {
        next.add(id);
      }
      return next;
    });
  }

  async function handleDelete(id: number) {
    await deleteTransaction(id);
    bump();
  }

  const selectStyle = { borderColor: "var(--border)" };

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-wrap items-center gap-3">
        <input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Search by title, notes, or tag…"
          className="min-w-64 flex-1 rounded border bg-transparent px-3 py-1.5 text-sm"
          style={selectStyle}
        />
        <select
          value={accountFilter}
          onChange={(e) => setAccountFilter(e.target.value ? Number(e.target.value) : "")}
          className="rounded border bg-transparent px-2 py-1.5 text-sm"
          style={selectStyle}
        >
          <option value="">All accounts</option>
          {accounts.map((a) => (
            <option key={a.id} value={a.id} style={{ color: "black" }}>{a.name}</option>
          ))}
        </select>
        <button
          onClick={() => setShowAddForm((v) => !v)}
          className="rounded px-3 py-1.5 text-sm font-medium"
          style={{ backgroundColor: "var(--accent)", color: "white" }}
        >
          {showAddForm ? "Close" : "+ Add"}
        </button>
      </div>

      {showAddForm && (
        <AddEntryForm
          accounts={accounts}
          tags={tags}
          onTagCreated={onTagCreated}
          onCreated={() => {
            bump();
            setShowAddForm(false);
          }}
        />
      )}

      <div className="flex flex-col gap-2">
        {transactions.map((t) => {
          const expanded = expandedIds.has(t.id);
          return (
            <div key={t.id} className="rounded-lg border" style={{ borderColor: "var(--border)", backgroundColor: "var(--surface-1)" }}>
              <button
                onClick={() => toggleExpanded(t.id)}
                className="flex w-full flex-wrap items-center gap-3 px-4 py-3 text-left"
              >
                <span className="w-4 text-xs" style={{ color: "var(--text-muted)" }}>{expanded ? "▾" : "▸"}</span>
                <span
                  className="w-24 shrink-0 text-xs"
                  style={{ color: "var(--text-secondary)", fontVariantNumeric: "tabular-nums" }}
                >
                  {t.date}
                </span>
                <span className="flex-1 font-medium">{t.description}</span>
                <div className="flex flex-wrap gap-1">
                  {t.tags.map((tag) => (
                    <span
                      key={tag.id}
                      className="rounded-full px-2 py-0.5 text-xs"
                      style={{ backgroundColor: `${tag.color ?? "#3987e5"}22`, color: "var(--text-secondary)" }}
                    >
                      {tag.name}
                    </span>
                  ))}
                </div>
                <span className="w-32 shrink-0 text-xs" style={{ color: "var(--text-secondary)" }}>
                  {accountName(t.account_id)}
                </span>
                <span
                  className="w-24 shrink-0 text-right"
                  style={{
                    fontVariantNumeric: "tabular-nums",
                    color: t.amount_cents < 0 ? "var(--status-critical)" : "var(--status-good)",
                  }}
                >
                  {formatCents(t.amount_cents)}
                </span>
              </button>

              {expanded && (
                <div className="border-t px-4 py-3" style={{ borderColor: "var(--gridline)" }}>
                  {editingId === t.id ? (
                    <EditTransactionForm
                      transaction={t}
                      accounts={accounts}
                      tags={tags}
                      onTagCreated={onTagCreated}
                      onSaved={() => {
                        bump();
                        setEditingId(null);
                      }}
                      onCancel={() => setEditingId(null)}
                    />
                  ) : (
                    <>
                      <p className="text-sm" style={{ color: t.notes ? "var(--text-secondary)" : "var(--text-muted)" }}>
                        {t.notes || "No notes."}
                      </p>
                      <div className="mt-3 flex gap-2">
                        <button
                          onClick={() => setEditingId(t.id)}
                          className="rounded border px-3 py-1 text-xs"
                          style={selectStyle}
                        >
                          Edit
                        </button>
                        <button
                          onClick={() => handleDelete(t.id)}
                          className="rounded border px-3 py-1 text-xs"
                          style={{ borderColor: "var(--status-critical)", color: "var(--status-critical)" }}
                        >
                          Delete
                        </button>
                      </div>
                    </>
                  )}
                </div>
              )}
            </div>
          );
        })}
        {transactions.length === 0 && (
          <p className="px-1 py-6 text-center text-sm" style={{ color: "var(--text-muted)" }}>
            {query ? "No transactions match your search." : "No transactions yet."}
          </p>
        )}
      </div>
    </div>
  );
}
