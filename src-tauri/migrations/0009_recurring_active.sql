-- Lets a Recurring item be paused (e.g. a loan that's paid off, a subscription that's cancelled)
-- without deleting it outright and losing its cadence/amount history. Paused items are excluded
-- from budget projections but keep their transaction history and can be reactivated any time.
ALTER TABLE recurring ADD COLUMN active INTEGER NOT NULL DEFAULT 1;
