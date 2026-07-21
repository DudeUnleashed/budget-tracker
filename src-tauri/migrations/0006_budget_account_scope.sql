-- Budgets now scope by a specific account (or NULL = any account), matching how Recurring
-- already scopes - the ledger label stays on accounts as pure classification, but is no longer
-- a separate budget-scoping dimension.
ALTER TABLE budgets DROP COLUMN ledger;
ALTER TABLE budgets ADD COLUMN account_id INTEGER REFERENCES accounts(id);
