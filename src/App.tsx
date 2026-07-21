import { useEffect, useLayoutEffect, useState } from "react";
import type { Account, Budget, DebtProgress, RecurringProgress, Tag } from "./api";
import { listAccounts, listBudgets, listDebts, listRecurring, listTags } from "./api";
import "./App.css";
import CategoriesView from "./components/CategoriesView";
import DashboardView from "./components/DashboardView";
import MiscView from "./components/MiscView";
import type { Theme } from "./components/SettingsView";
import TransactionsView from "./components/TransactionsView";
import type { DateFormat } from "./utils";
import { getCurrencyCode, getDateFormat, setCurrencyCode, setDateFormat } from "./utils";

const THEME_STORAGE_KEY = "budget-tracker:theme";

type Tab = "dashboard" | "transactions" | "categories" | "misc";

// Misc always comes last - it's accounts/backup/settings/debt calculator, the stuff you set up
// once and rarely touch again, not part of the everyday flow.
const TABS: { key: Tab; label: string }[] = [
  { key: "dashboard", label: "Dashboard" },
  { key: "transactions", label: "Transactions" },
  { key: "categories", label: "Categories" },
  { key: "misc", label: "Misc" },
];

function App() {
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [tags, setTags] = useState<Tag[]>([]);
  const [recurring, setRecurring] = useState<RecurringProgress[]>([]);
  const [budgets, setBudgets] = useState<Budget[]>([]);
  const [debts, setDebts] = useState<DebtProgress[]>([]);
  const [dataVersion, setDataVersion] = useState(0);

  const now = new Date();
  const [year, setYear] = useState(now.getFullYear());
  const [month, setMonth] = useState(now.getMonth() + 1);
  const [tab, setTab] = useState<Tab>("dashboard");

  const [theme, setTheme] = useState<Theme>(() => (localStorage.getItem(THEME_STORAGE_KEY) as Theme | null) ?? "dark");
  const [currencyCode, setCurrencyCodeState] = useState(() => getCurrencyCode());
  const [dateFormat, setDateFormatState] = useState<DateFormat>(() => getDateFormat());

  // Runs before paint so there's no flash of the wrong theme on launch.
  useLayoutEffect(() => {
    document.documentElement.setAttribute("data-theme", theme);
  }, [theme]);

  function handleThemeChange(next: Theme) {
    setTheme(next);
    localStorage.setItem(THEME_STORAGE_KEY, next);
  }

  function handleCurrencyChange(code: string) {
    setCurrencyCode(code);
    setCurrencyCodeState(code);
  }

  function handleDateFormatChange(format: DateFormat) {
    setDateFormat(format);
    setDateFormatState(format);
  }

  async function refreshAll() {
    const [acc, tg, rec, bud, dbts] = await Promise.all([listAccounts(), listTags(), listRecurring(), listBudgets(), listDebts()]);
    setAccounts(acc);
    setTags(tg);
    setRecurring(rec);
    setBudgets(bud);
    setDebts(dbts);
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
      <div className="mx-auto max-w-[96rem] px-6 py-8">
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

        {accounts.length === 0 && (tab === "dashboard" || tab === "transactions") && (
          <p className="mb-4 rounded-lg border px-4 py-3 text-sm" style={{ borderColor: "var(--border)", color: "var(--text-muted)" }}>
            Add an account first (Misc tab) before adding transactions or recurring items.
          </p>
        )}

        {tab === "dashboard" && (
          <DashboardView
            accounts={accounts}
            tags={tags}
            recurring={recurring}
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
        {tab === "transactions" && (
          <TransactionsView
            accounts={accounts}
            tags={tags}
            onTagCreated={handleTagCreated}
            dataVersion={dataVersion}
            bump={bump}
          />
        )}
        {tab === "categories" && (
          <CategoriesView
            accounts={accounts}
            tags={tags}
            recurring={recurring}
            budgets={budgets}
            year={year}
            month={month}
            dataVersion={dataVersion}
            bump={bump}
          />
        )}
        {tab === "misc" && (
          <MiscView
            accounts={accounts}
            tags={tags}
            debts={debts}
            bump={bump}
            onAccountCreated={handleAccountCreated}
            theme={theme}
            onThemeChange={handleThemeChange}
            currencyCode={currencyCode}
            onCurrencyChange={handleCurrencyChange}
            dateFormat={dateFormat}
            onDateFormatChange={handleDateFormatChange}
          />
        )}
      </div>
    </main>
  );
}

export default App;
