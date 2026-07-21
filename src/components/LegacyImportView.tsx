import { open } from "@tauri-apps/plugin-dialog";
import { useState } from "react";
import type { LegacyImportSummary, LegacyMigrationPlan } from "../api";
import { applyLegacyImport, previewLegacyImport } from "../api";

interface Props {
  bump: () => void;
}

interface SubChoice {
  tagId: number | null;
  newTagName: string | null;
}

interface BudgetChoiceState {
  accountId: number | null;
}

export default function LegacyImportView({ bump }: Props) {
  const [dir, setDir] = useState<string | null>(null);
  const [plan, setPlan] = useState<LegacyMigrationPlan | null>(null);
  const [subChoices, setSubChoices] = useState<Record<number, SubChoice>>({});
  const [budgetChoices, setBudgetChoices] = useState<Record<number, BudgetChoiceState>>({});
  const [loading, setLoading] = useState(false);
  const [applying, setApplying] = useState(false);
  const [error, setError] = useState("");
  const [result, setResult] = useState<LegacyImportSummary | null>(null);

  const selectStyle = { borderColor: "var(--border)" };
  const cardStyle = { borderColor: "var(--border)", backgroundColor: "var(--surface-1)" };

  async function handlePickFolder() {
    const picked = await open({ directory: true, title: "Choose the old backup folder" });
    if (!picked || Array.isArray(picked)) return;
    setError("");
    setResult(null);
    setLoading(true);
    try {
      const p = await previewLegacyImport(picked);
      setDir(picked);
      setPlan(p);

      const subInit: Record<number, SubChoice> = {};
      for (const r of p.subscription_resolutions) {
        subInit[r.subscription_id] = {
          tagId: r.suggested_tag_id,
          newTagName: r.suggested_tag_id === null ? r.suggested_new_tag_name : null,
        };
      }
      setSubChoices(subInit);

      const budgetInit: Record<number, BudgetChoiceState> = {};
      for (const r of p.budget_resolutions) {
        budgetInit[r.budget_id] = { accountId: r.suggested_account_id };
      }
      setBudgetChoices(budgetInit);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }

  function cancel() {
    setPlan(null);
    setDir(null);
  }

  async function handleApply() {
    if (!plan || !dir) return;
    setError("");
    setApplying(true);
    try {
      const summary = await applyLegacyImport(
        dir,
        plan.subscription_resolutions.map((r) => ({
          subscriptionId: r.subscription_id,
          tagId: subChoices[r.subscription_id]?.tagId ?? null,
          newTagName: subChoices[r.subscription_id]?.newTagName ?? null,
        })),
        plan.budget_resolutions.map((r) => ({
          budgetId: r.budget_id,
          accountId: budgetChoices[r.budget_id]?.accountId ?? null,
        })),
      );
      setResult(summary);
      cancel();
      bump();
    } catch (err) {
      setError(String(err));
    } finally {
      setApplying(false);
    }
  }

  const reviewSubs = plan?.subscription_resolutions.filter((r) => r.needs_review) ?? [];
  const reviewBudgets = plan?.budget_resolutions.filter((r) => r.needs_review) ?? [];

  return (
    <div className="rounded-lg border p-4" style={cardStyle}>
      <h3 className="font-medium">Import from an older version</h3>
      <p className="mt-1 text-sm" style={{ color: "var(--text-secondary)" }}>
        Converts a backup made before Recurring/Categories existed - subscriptions become
        Recurring items, ledger-scoped budgets become account-scoped. Transactions and amounts
        carry over exactly; nothing ambiguous is guessed silently.
      </p>

      {!plan && (
        <button
          onClick={handlePickFolder}
          disabled={loading}
          className="mt-3 rounded border px-3 py-1.5 text-sm font-medium"
          style={selectStyle}
        >
          {loading ? "Reading…" : "Choose old backup folder"}
        </button>
      )}

      {plan && (
        <div className="mt-4 flex flex-col gap-4">
          <p className="text-sm" style={{ color: "var(--text-secondary)" }}>
            Found {plan.accounts} account{plan.accounts === 1 ? "" : "s"}, {plan.tags} tag{plan.tags === 1 ? "" : "s"},{" "}
            {plan.transactions} transaction{plan.transactions === 1 ? "" : "s"},{" "}
            {plan.subscription_resolutions.length} subscription{plan.subscription_resolutions.length === 1 ? "" : "s"},{" "}
            {plan.budget_resolutions.length} budget{plan.budget_resolutions.length === 1 ? "" : "s"}.
            {plan.occurrences_dropped > 0 && (
              <> {plan.occurrences_dropped} old occurrence record{plan.occurrences_dropped === 1 ? "" : "s"} won't carry
              over - Recurring tracks payment live from transactions instead.</>
            )}
          </p>

          {reviewSubs.length > 0 && (
            <div className="flex flex-col gap-2">
              <p className="text-sm font-medium">Subscriptions needing a tag decision</p>
              {reviewSubs.map((r) => {
                const choice = subChoices[r.subscription_id];
                return (
                  <div key={r.subscription_id} className="flex flex-wrap items-center gap-2 rounded border p-2 text-sm" style={selectStyle}>
                    <span className="font-medium">{r.subscription_name}</span>
                    {r.dropped_fields_note && (
                      <span className="text-xs" style={{ color: "var(--text-muted)" }}>({r.dropped_fields_note})</span>
                    )}
                    <select
                      value={choice?.tagId ?? ""}
                      onChange={(e) => {
                        const v = e.target.value;
                        setSubChoices((prev) => ({
                          ...prev,
                          [r.subscription_id]: v === ""
                            ? { tagId: null, newTagName: r.suggested_new_tag_name }
                            : { tagId: Number(v), newTagName: null },
                        }));
                      }}
                      className="rounded border bg-transparent px-2 py-1 text-sm"
                      style={selectStyle}
                    >
                      <option value="">Create new tag…</option>
                      {plan.all_tags.map((t) => (
                        <option key={t.id} value={t.id} style={{ color: "black" }}>{t.name}</option>
                      ))}
                    </select>
                    {choice?.tagId === null && (
                      <input
                        value={choice?.newTagName ?? ""}
                        onChange={(e) =>
                          setSubChoices((prev) => ({ ...prev, [r.subscription_id]: { tagId: null, newTagName: e.target.value } }))
                        }
                        placeholder="New tag name"
                        className="rounded border bg-transparent px-2 py-1 text-sm"
                        style={selectStyle}
                      />
                    )}
                  </div>
                );
              })}
            </div>
          )}

          {reviewBudgets.length > 0 && (
            <div className="flex flex-col gap-2">
              <p className="text-sm font-medium">Budgets needing an account decision</p>
              {reviewBudgets.map((r) => (
                <div key={r.budget_id} className="flex flex-wrap items-center gap-2 rounded border p-2 text-sm" style={selectStyle}>
                  <span className="font-medium">Ledger "{r.ledger}"</span>
                  <select
                    value={budgetChoices[r.budget_id]?.accountId ?? ""}
                    onChange={(e) =>
                      setBudgetChoices((prev) => ({
                        ...prev,
                        [r.budget_id]: { accountId: e.target.value ? Number(e.target.value) : null },
                      }))
                    }
                    className="rounded border bg-transparent px-2 py-1 text-sm"
                    style={selectStyle}
                  >
                    <option value="">Any account</option>
                    {plan.all_accounts.map((a) => (
                      <option key={a.id} value={a.id} style={{ color: "black" }}>{a.name}</option>
                    ))}
                  </select>
                </div>
              ))}
            </div>
          )}

          <div className="flex gap-2">
            <button
              onClick={handleApply}
              disabled={applying}
              className="rounded px-3 py-1.5 text-sm font-medium"
              style={{ backgroundColor: "var(--accent)", color: "white" }}
            >
              {applying ? "Importing…" : "Confirm & import"}
            </button>
            <button onClick={cancel} className="rounded border px-3 py-1.5 text-sm" style={selectStyle}>
              Cancel
            </button>
          </div>
        </div>
      )}

      {result && (
        <p className="mt-3 text-sm" style={{ color: "var(--status-good)" }}>
          Imported {result.accounts} accounts, {result.tags} tags, {result.transactions} transactions,{" "}
          {result.recurring} recurring items, {result.budgets} budgets.
        </p>
      )}
      {error && <p className="mt-3 text-sm" style={{ color: "var(--status-critical)" }}>{error}</p>}
    </div>
  );
}
