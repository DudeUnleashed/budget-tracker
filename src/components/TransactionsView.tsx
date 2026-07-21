import { useEffect, useState } from "react";
import type { Account, Tag, Transaction } from "../api";
import { deleteTransaction, searchTransactions } from "../api";
import AddEntryForm from "./AddEntryForm";
import EditTransactionForm from "./EditTransactionForm";
import LedgerTable from "./LedgerTable";

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
  const [editingTransaction, setEditingTransaction] = useState<Transaction | null>(null);

  useEffect(() => {
    let cancelled = false;
    searchTransactions(query || null, accountFilter === "" ? null : Number(accountFilter)).then((r) => {
      if (!cancelled) setTransactions(r);
    });
    return () => {
      cancelled = true;
    };
  }, [query, accountFilter, dataVersion]);

  async function handleDelete(id: number) {
    const t = transactions.find((tx) => tx.id === id);
    if (!window.confirm(`Delete "${t?.description ?? "this transaction"}"? This can't be undone.`)) return;
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

      <LedgerTable
        transactions={transactions}
        accounts={accounts}
        onEdit={setEditingTransaction}
        onDelete={handleDelete}
        emptyMessage={query ? "No transactions match your search." : "No transactions yet."}
      />
    </div>
  );
}
