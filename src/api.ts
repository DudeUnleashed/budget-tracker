import { invoke } from "@tauri-apps/api/core";

export interface Tag {
  id: number;
  name: string;
  color: string | null;
  parent_id: number | null;
}

export interface Account {
  id: number;
  name: string;
  ledger: string;
  starting_balance_cents: number;
  active: boolean;
  is_default: boolean;
  created_at: string;
  updated_at: string;
}

export type TransactionType = "expense" | "income" | "transfer";

export interface Transaction {
  id: number;
  account_id: number;
  date: string;
  amount_cents: number;
  type: TransactionType;
  description: string;
  notes: string | null;
  linked_group_id: string | null;
  tags: Tag[];
  created_at: string;
  updated_at: string;
}

export type IntervalUnit = "day" | "week" | "month" | "year";

export interface Recurring {
  id: number;
  tag_id: number;
  account_id: number | null;
  type: "expense" | "income";
  interval_unit: IntervalUnit;
  interval_count: number;
  anchor_date: string;
  projected_amount_cents: number;
  notes: string | null;
  /** Paused items keep their history but are excluded from budget projections. */
  active: boolean;
  created_at: string;
  updated_at: string;
}

export interface RecurringProgress {
  recurring: Recurring;
  tag: Tag;
  cycle_start: string;
  cycle_end: string;
  paid_cents: number;
  difference_cents: number;
}

export interface Budget {
  id: number;
  account_id: number | null;
  tag_id: number | null;
  amount_cents: number;
  show_on_dashboard: boolean;
  created_at: string;
  updated_at: string;
}

export interface BudgetProgress {
  budget: Budget;
  spent_cents: number;
  /** Effective amount for this month - a month override if one exists, else budget.amount_cents. */
  amount_cents: number;
  is_override: boolean;
  percent: number;
}

export interface ChildSpend {
  tag: Tag;
  spent_cents: number;
}

export interface MonthSummary {
  transactions: Transaction[];
  total_income_cents: number;
  total_expense_cents: number;
  net_cents: number;
}

export interface MonthTotal {
  year: number;
  month: number;
  total_income_cents: number;
  total_expense_cents: number;
  net_cents: number;
}

// Accounts
export const createAccount = (name: string, ledger: string, startingBalanceCents: number) =>
  invoke<Account>("create_account", { name, ledger, startingBalanceCents });

export const listAccounts = () => invoke<Account[]>("list_accounts");

export const updateAccount = (id: number, name: string, ledger: string, startingBalanceCents: number) =>
  invoke<Account>("update_account", { id, name, ledger, startingBalanceCents });

export const setDefaultAccount = (id: number) => invoke<Account>("set_default_account", { id });

export const deleteAccount = (id: number) => invoke<void>("delete_account", { id });

export const getAccountBalance = (accountId: number) =>
  invoke<number>("get_account_balance", { accountId });

// Tags
export const createTag = (name: string, color: string | null, parentId: number | null = null) =>
  invoke<Tag>("create_tag", { name, color, parentId });

export const updateTag = (id: number, name: string, color: string | null, parentId: number | null) =>
  invoke<Tag>("update_tag", { id, name, color, parentId });

export const deleteTag = (id: number) => invoke<void>("delete_tag", { id });

export const listTags = () => invoke<Tag[]>("list_tags");

// Transactions
export const createTransaction = (params: {
  accountId: number;
  date: string;
  amountCents: number;
  kind: "expense" | "income";
  description: string;
  notes?: string | null;
  tagIds: number[];
}) => invoke<Transaction>("create_transaction", { notes: null, ...params });

/// One account transfer command covers both cases: same-ledger transfer or cross-ledger
/// movement. The backend decides which based on whether the two accounts share a ledger.
export const createAccountTransfer = (params: {
  fromAccountId: number;
  toAccountId: number;
  date: string;
  amountCents: number;
  description: string;
  tagIds: number[];
}) => invoke<Transaction[]>("create_account_transfer", params);

export const updateTransaction = (params: {
  id: number;
  accountId: number;
  date: string;
  amountCents: number;
  kind: TransactionType;
  description: string;
  notes?: string | null;
  tagIds: number[];
}) => invoke<Transaction>("update_transaction", { notes: null, ...params });

export const deleteTransaction = (id: number) => invoke<void>("delete_transaction", { id });

export const listTransactionsForMonth = (
  accountId: number | null,
  tagId: number | null,
  year: number,
  month: number,
) => invoke<Transaction[]>("list_transactions_for_month", { accountId, tagId, year, month });

export const listTransactionsInRange = (
  accountId: number | null,
  tagId: number | null,
  start: string,
  end: string,
) => invoke<Transaction[]>("list_transactions_in_range", { accountId, tagId, start, end });

export const searchTransactions = (query: string | null, accountId: number | null = null) =>
  invoke<Transaction[]>("search_transactions", { query, accountId });

// Recurring
export const createRecurring = (params: {
  tagId: number;
  accountId: number | null;
  kind: "expense" | "income";
  intervalUnit: IntervalUnit;
  intervalCount: number;
  anchorDate: string;
  projectedAmountCents: number;
  notes?: string | null;
}) => invoke<RecurringProgress>("create_recurring", { notes: null, ...params });

export const updateRecurring = (params: {
  id: number;
  tagId: number;
  accountId: number | null;
  kind: "expense" | "income";
  intervalUnit: IntervalUnit;
  intervalCount: number;
  anchorDate: string;
  projectedAmountCents: number;
  notes?: string | null;
}) => invoke<RecurringProgress>("update_recurring", { notes: null, ...params });

export const deleteRecurring = (id: number) => invoke<void>("delete_recurring", { id });

export const setRecurringActive = (id: number, active: boolean) =>
  invoke<RecurringProgress>("set_recurring_active", { id, active });

export const listRecurring = () => invoke<RecurringProgress[]>("list_recurring");

export const listRecurringCycleTransactions = (id: number) =>
  invoke<Transaction[]>("list_recurring_cycle_transactions", { id });

// Budgets
export const createBudget = (
  accountId: number | null,
  tagId: number | null,
  amountCents: number,
  showOnDashboard = true,
) => invoke<Budget>("create_budget", { accountId, tagId, amountCents, showOnDashboard });

export const listBudgets = () => invoke<Budget[]>("list_budgets");

export const updateBudget = (
  id: number,
  accountId: number | null,
  tagId: number | null,
  amountCents: number,
  showOnDashboard: boolean,
) => invoke<Budget>("update_budget", { id, accountId, tagId, amountCents, showOnDashboard });

export const deleteBudget = (id: number) => invoke<void>("delete_budget", { id });

export const getBudgetProgress = (year: number, month: number) =>
  invoke<BudgetProgress[]>("get_budget_progress", { year, month });

export const getBudgetBreakdown = (budgetId: number, year: number, month: number) =>
  invoke<ChildSpend[]>("get_budget_breakdown", { budgetId, year, month });

/** Like getBudgetBreakdown, but keyed by any tag directly - used to recurse into a child's
 * own children when drilling down more than one level. */
export const getTagBreakdown = (tagId: number, accountId: number | null, year: number, month: number) =>
  invoke<ChildSpend[]>("get_tag_breakdown", { tagId, accountId, year, month });

export const setBudgetMonthOverride = (budgetId: number, year: number, month: number, amountCents: number) =>
  invoke<BudgetProgress>("set_budget_month_override", { budgetId, year, month, amountCents });

export const clearBudgetMonthOverride = (budgetId: number, year: number, month: number) =>
  invoke<BudgetProgress>("clear_budget_month_override", { budgetId, year, month });

// Dashboard
export const getMonthSummary = (year: number, month: number, accountId: number | null = null) =>
  invoke<MonthSummary>("get_month_summary", { year, month, accountId });

export const getMonthlyTrend = (year: number, month: number, months: number, accountId: number | null = null) =>
  invoke<MonthTotal[]>("get_monthly_trend", { year, month, months, accountId });

// Debts (standalone payoff calculators - no tags/accounts, purely computed)
export interface Debt {
  id: number;
  name: string;
  start_date: string;
  principal_cents: number;
  monthly_payment_cents: number;
  /** Annual rate in hundredths of a percent (e.g. 1999 = 19.99%). */
  interest_rate_bps: number;
  created_at: string;
  updated_at: string;
}

export interface DebtPayoffResult {
  payoff_date: string | null;
  months: number;
  total_paid_cents: number;
  total_interest_cents: number;
  never_pays_off: boolean;
}

export interface DebtProgress extends DebtPayoffResult {
  debt: Debt;
}

export interface DebtInput {
  name: string;
  startDate: string;
  principalCents: number;
  monthlyPaymentCents: number;
  interestRateBps: number;
}

export const createDebt = (input: DebtInput) => invoke<DebtProgress>("create_debt", { ...input });

export const updateDebt = (id: number, input: DebtInput) => invoke<DebtProgress>("update_debt", { id, ...input });

export const deleteDebt = (id: number) => invoke<void>("delete_debt", { id });

export const listDebts = () => invoke<DebtProgress[]>("list_debts");

export const previewDebtPayoff = (input: DebtInput) =>
  invoke<DebtPayoffResult>("preview_debt_payoff", {
    startDate: input.startDate,
    principalCents: input.principalCents,
    monthlyPaymentCents: input.monthlyPaymentCents,
    interestRateBps: input.interestRateBps,
  });

// Backup / restore
export interface ImportSummary {
  accounts: number;
  tags: number;
  transactions: number;
  recurring: number;
  budgets: number;
  debts: number;
}

export const exportData = (dir: string) => invoke<void>("export_data", { dir });

export const importData = (dir: string) => invoke<ImportSummary>("import_data", { dir });

/** Deletes every account, tag, transaction, recurring item, budget, and debt. Irreversible. */
export const wipeAllData = () => invoke<void>("wipe_all_data");

// Legacy import (pre-Recurring backups: subscriptions/occurrences, ledger-scoped budgets, flat tags)
export interface TagOption {
  id: number;
  name: string;
}

export interface AccountOption {
  id: number;
  name: string;
}

export interface SubscriptionResolution {
  subscription_id: number;
  subscription_name: string;
  candidate_tags: TagOption[];
  suggested_tag_id: number | null;
  suggested_new_tag_name: string;
  needs_review: boolean;
  dropped_fields_note: string | null;
}

export interface BudgetResolution {
  budget_id: number;
  ledger: string | null;
  matching_accounts: AccountOption[];
  suggested_account_id: number | null;
  needs_review: boolean;
}

export interface LegacyMigrationPlan {
  accounts: number;
  tags: number;
  transactions: number;
  occurrences_dropped: number;
  all_tags: TagOption[];
  all_accounts: AccountOption[];
  subscription_resolutions: SubscriptionResolution[];
  budget_resolutions: BudgetResolution[];
}

export interface LegacyImportSummary {
  accounts: number;
  tags: number;
  transactions: number;
  recurring: number;
  budgets: number;
  occurrences_dropped: number;
}

export const previewLegacyImport = (dir: string) => invoke<LegacyMigrationPlan>("preview_legacy_import", { dir });

export const applyLegacyImport = (
  dir: string,
  subscriptionChoices: { subscriptionId: number; tagId: number | null; newTagName: string | null }[],
  budgetChoices: { budgetId: number; accountId: number | null }[],
) => invoke<LegacyImportSummary>("apply_legacy_import", { dir, subscriptionChoices, budgetChoices });
