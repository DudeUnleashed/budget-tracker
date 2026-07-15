import { useEffect, useState } from "react";
import type { Account, Subscription, Tag } from "./api";
import { listAccounts, listSubscriptions, listTags } from "./api";
import "./App.css";
import AccountsView from "./components/AccountsView";
import BackupView from "./components/BackupView";
import BudgetsView from "./components/BudgetsView";
import DashboardView from "./components/DashboardView";
import SubscriptionsView from "./components/SubscriptionsView";
import TransactionsView from "./components/TransactionsView";

type Tab = "dashboard" | "accounts" | "transactions" | "subscriptions" | "budgets" | "backup";

const TABS: { key: Tab; label: string }[] = [
  { key: "dashboard", label: "Dashboard" },
  { key: "accounts", label: "Accounts" },
  { key: "transactions", label: "Transactions" },
  { key: "subscriptions", label: "Subscriptions" },
  { key: "budgets", label: "Budgets" },
  { key: "backup", label: "Backup" },
];

function App() {
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [tags, setTags] = useState<Tag[]>([]);
  const [subscriptions, setSubscriptions] = useState<Subscription[]>([]);
  const [dataVersion, setDataVersion] = useState(0);

  const now = new Date();
  const [year, setYear] = useState(now.getFullYear());
  const [month, setMonth] = useState(now.getMonth() + 1);
  const [tab, setTab] = useState<Tab>("dashboard");

  async function refreshAll() {
    const [acc, tg, subs] = await Promise.all([listAccounts(), listTags(), listSubscriptions(false)]);
    setAccounts(acc);
    setTags(tg);
    setSubscriptions(subs);
    setDataVersion((v) => v + 1);
  }

  useEffect(() => {
    refreshAll();
  }, []);

  function bump() {
    refreshAll();
  }

  function handleAccountCreated(account: Account) {
    setAccounts((prev) => [...prev, account]);
    setDataVersion((v) => v + 1);
  }

  function handleTagCreated(tag: Tag) {
    setTags((prev) => [...prev, tag]);
  }

  return (
    <main className="min-h-screen" style={{ backgroundColor: "var(--page-plane)", color: "var(--text-primary)" }}>
      <div className="mx-auto max-w-5xl px-6 py-8">
        <div className="mb-6 flex items-center justify-between">
          <h1 className="text-xl font-semibold">Budget Tracker</h1>
          <nav className="flex gap-1 rounded-lg border p-1" style={{ borderColor: "var(--border)" }}>
            {TABS.map((t) => (
              <button
                key={t.key}
                onClick={() => setTab(t.key)}
                className="rounded px-3 py-1.5 text-sm font-medium"
                style={{
                  backgroundColor: tab === t.key ? "var(--accent)" : "transparent",
                  color: tab === t.key ? "white" : "var(--text-secondary)",
                }}
              >
                {t.label}
              </button>
            ))}
          </nav>
        </div>

        {accounts.length === 0 && tab !== "accounts" && tab !== "backup" && (
          <p className="mb-4 rounded-lg border px-4 py-3 text-sm" style={{ borderColor: "var(--border)", color: "var(--text-muted)" }}>
            Add an account first (Accounts tab) before adding transactions or subscriptions.
          </p>
        )}

        {tab === "dashboard" && (
          <DashboardView
            accounts={accounts}
            tags={tags}
            subscriptions={subscriptions}
            year={year}
            month={month}
            onMonthChange={(y, m) => {
              setYear(y);
              setMonth(m);
            }}
            onTagCreated={handleTagCreated}
            dataVersion={dataVersion}
            bump={bump}
          />
        )}
        {tab === "accounts" && (
          <AccountsView accounts={accounts} onAccountCreated={handleAccountCreated} bump={bump} />
        )}
        {tab === "transactions" && (
          <TransactionsView
            accounts={accounts}
            tags={tags}
            onTagCreated={handleTagCreated}
            dataVersion={dataVersion}
            bump={bump}
          />
        )}
        {tab === "subscriptions" && (
          <SubscriptionsView
            accounts={accounts}
            tags={tags}
            subscriptions={subscriptions}
            onTagCreated={handleTagCreated}
            bump={bump}
          />
        )}
        {tab === "budgets" && (
          <BudgetsView
            tags={tags}
            year={year}
            month={month}
            dataVersion={dataVersion}
            bump={bump}
            onTagCreated={handleTagCreated}
          />
        )}
        {tab === "backup" && <BackupView bump={bump} />}
      </div>
    </main>
  );
}

export default App;
