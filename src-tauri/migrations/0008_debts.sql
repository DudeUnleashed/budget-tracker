-- Standalone debt payoff calculators - no tags or accounts, just the inputs an amortization
-- schedule needs. Payoff date and total cost are computed live from these, never stored.
CREATE TABLE debts (
  id                    INTEGER PRIMARY KEY,
  name                  TEXT NOT NULL,
  start_date            TEXT NOT NULL,
  principal_cents       INTEGER NOT NULL,
  monthly_payment_cents INTEGER NOT NULL,
  interest_rate_bps     INTEGER NOT NULL,
  created_at            TEXT NOT NULL,
  updated_at            TEXT NOT NULL
);
