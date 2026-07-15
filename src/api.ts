import { invoke } from "@tauri-apps/api/core";

export interface Tag {
  id: number;
  name: string;
  color: string | null;
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

export interface Subscription {
  id: number;
  account_id: number;
  name: string;
  amount_cents: number;
  type: "expense" | "income";
  interval_unit: IntervalUnit;
  interval_count: number;
  start_date: string;
  next_charge_date: string;
  end_date: string | null;
  active: boolean;
  paused_until: string | null;
  notes: string | null;
  tags: Tag[];
  created_at: string;
  updated_at: string;
}

export interface SubscriptionOccurrence {
  id: number;
  subscription_id: number;
  due_date: string;
  amount_cents: number;
  status: "projected" | "confirmed" | "skipped";
  paid_date: string | null;
}

export interface Budget {
  id: number;
  ledger: string | null;
  tag_id: number | null;
  amount_cents: number;
  show_on_dashboard: boolean;
  created_at: string;
  updated_at: string;
}

export interface BudgetProgress {
  budget: Budget;
  spent_cents: number;
  percent: number;
}

export interface MonthSummary {
  transactions: Transaction[];
  occurrences: SubscriptionOccurrence[];
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

export const getAccountBalance = (accountId: number) =>
  invoke<number>("get_account_balance", { accountId });

// Tags
export const createTag = (name: string, color: string | null) =>
  invoke<Tag>("create_tag", { name, color });

export const updateTag = (id: number, name: string, color: string | null) =>
  invoke<Tag>("update_tag", { id, name, color });

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

export const createTransfer = (params: {
  fromAccountId: number;
  toAccountId: number;
  date: string;
  amountCents: number;
  description: string;
  tagIds: number[];
}) => invoke<Transaction[]>("create_transfer", params);

export const createCrossLedgerMovement = (params: {
  fromAccountId: number;
  toAccountId: number;
  date: string;
  amountCents: number;
  description: string;
  tagId: number;
}) => invoke<Transaction[]>("create_cross_ledger_movement", params);

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

export const listTransactionsForMonth = (accountId: number | null, year: number, month: number) =>
  invoke<Transaction[]>("list_transactions_for_month", { accountId, year, month });

export const searchTransactions = (query: string | null, accountId: number | null = null) =>
  invoke<Transaction[]>("search_transactions", { query, accountId });

// Subscriptions
export const createSubscription = (params: {
  accountId: number;
  name: string;
  amountCents: number;
  kind: "expense" | "income";
  intervalUnit: IntervalUnit;
  intervalCount: number;
  startDate: string;
  endDate?: string | null;
  notes?: string | null;
  tagIds: number[];
}) => invoke<Subscription>("create_subscription", { endDate: null, notes: null, ...params });

export const listSubscriptions = (activeOnly: boolean) =>
  invoke<Subscription[]>("list_subscriptions", { activeOnly });

export const updateSubscription = (params: {
  id: number;
  accountId: number;
  name: string;
  amountCents: number;
  intervalUnit: IntervalUnit;
  intervalCount: number;
  tagIds: number[];
}) => invoke<Subscription>("update_subscription", params);

export const updateSubscriptionAmount = (id: number, amountCents: number) =>
  invoke<Subscription>("update_subscription_amount", { id, amountCents });

export const pauseSubscription = (id: number, pausedUntil: string | null = null) =>
  invoke<Subscription>("pause_subscription", { id, pausedUntil });

export const cancelSubscription = (id: number) => invoke<Subscription>("cancel_subscription", { id });

export const reactivateSubscription = (id: number, nextChargeDate: string | null = null) =>
  invoke<Subscription>("reactivate_subscription", { id, nextChargeDate });

export const updateOccurrence = (params: {
  id: number;
  amountCents?: number | null;
  status?: string | null;
  paidDate?: string | null;
}) =>
  invoke<SubscriptionOccurrence>("update_occurrence", {
    amountCents: null,
    status: null,
    paidDate: null,
    ...params,
  });

// Budgets
export const createBudget = (
  ledger: string | null,
  tagId: number | null,
  amountCents: number,
  showOnDashboard = true,
) => invoke<Budget>("create_budget", { ledger, tagId, amountCents, showOnDashboard });

export const listBudgets = () => invoke<Budget[]>("list_budgets");

export const updateBudget = (
  id: number,
  ledger: string | null,
  tagId: number | null,
  amountCents: number,
  showOnDashboard: boolean,
) => invoke<Budget>("update_budget", { id, ledger, tagId, amountCents, showOnDashboard });

export const deleteBudget = (id: number) => invoke<void>("delete_budget", { id });

export const getBudgetProgress = (year: number, month: number) =>
  invoke<BudgetProgress[]>("get_budget_progress", { year, month });

// Dashboard
export const getMonthSummary = (year: number, month: number, accountId: number | null = null) =>
  invoke<MonthSummary>("get_month_summary", { year, month, accountId });

// Backup / restore
export interface ImportSummary {
  accounts: number;
  tags: number;
  transactions: number;
  subscriptions: number;
  occurrences: number;
  budgets: number;
}

export const exportData = (dir: string) => invoke<void>("export_data", { dir });

export const importData = (dir: string) => invoke<ImportSummary>("import_data", { dir });
