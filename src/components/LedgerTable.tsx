import type { Account, Transaction } from "../api";
import { formatCents, formatDate } from "../utils";

interface Props {
  transactions: Transaction[];
  accounts: Account[];
  onEdit: (t: Transaction) => void;
  onDelete: (id: number) => void;
  emptyMessage: string;
}

const COLUMN_WIDTHS = {
  date: "116px",
  title: "auto",
  tags: "200px",
  account: "160px",
  amount: "120px",
  actions: "64px",
};

export default function LedgerTable({ transactions, accounts, onEdit, onDelete, emptyMessage }: Props) {
  const accountName = (id: number) => accounts.find((a) => a.id === id)?.name ?? `#${id}`;

  return (
    <div className="overflow-x-auto rounded-lg border" style={{ borderColor: "var(--border)" }}>
      <table className="w-full table-fixed text-sm">
        <colgroup>
          <col style={{ width: COLUMN_WIDTHS.date }} />
          <col style={{ width: COLUMN_WIDTHS.title }} />
          <col style={{ width: COLUMN_WIDTHS.tags }} />
          <col style={{ width: COLUMN_WIDTHS.account }} />
          <col style={{ width: COLUMN_WIDTHS.amount }} />
          <col style={{ width: COLUMN_WIDTHS.actions }} />
        </colgroup>
        <thead>
          <tr className="border-b" style={{ color: "var(--text-muted)", borderColor: "var(--gridline)" }}>
            <th className="px-3 py-2 text-left font-normal">Date</th>
            <th className="px-3 py-2 text-left font-normal">Title</th>
            <th className="px-3 py-2 text-left font-normal">Tags</th>
            <th className="px-3 py-2 text-left font-normal">Account</th>
            <th className="px-3 py-2 text-right font-normal">Amount</th>
            <th className="px-3 py-2" />
          </tr>
        </thead>
        <tbody>
          {transactions.map((t) => (
            <tr key={t.id} className="border-b align-top" style={{ borderColor: "var(--gridline)" }}>
              <td className="truncate px-3 py-2" style={{ color: "var(--text-secondary)", fontVariantNumeric: "tabular-nums" }}>
                {formatDate(t.date)}
              </td>
              <td className="truncate px-3 py-2">
                {t.description}
                {t.notes && (
                  <span className="ml-2 text-xs" style={{ color: "var(--text-muted)" }}>
                    — {t.notes}
                  </span>
                )}
              </td>
              <td className="px-3 py-2">
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
              </td>
              <td className="truncate px-3 py-2" style={{ color: "var(--text-secondary)" }}>
                {accountName(t.account_id)} · {t.type}
              </td>
              <td
                className="px-3 py-2 text-right"
                style={{
                  fontVariantNumeric: "tabular-nums",
                  color: t.amount_cents < 0 ? "var(--status-critical)" : "var(--status-good)",
                }}
              >
                {formatCents(t.amount_cents)}
              </td>
              <td className="px-3 py-2 text-right">
                <div className="flex justify-end gap-2">
                  <button onClick={() => onEdit(t)} title="Edit" aria-label={`Edit ${t.description}`} style={{ color: "var(--text-muted)" }}>
                    ✎
                  </button>
                  <button onClick={() => onDelete(t.id)} title="Delete" aria-label={`Delete ${t.description}`} style={{ color: "var(--status-critical)" }}>
                    ✕
                  </button>
                </div>
              </td>
            </tr>
          ))}
          {transactions.length === 0 && (
            <tr>
              <td colSpan={6} className="px-3 py-6 text-center" style={{ color: "var(--text-muted)" }}>
                {emptyMessage}
              </td>
            </tr>
          )}
        </tbody>
      </table>
    </div>
  );
}
