-- A budget's amount_cents is the default; a row here overrides it for one specific month
-- (e.g. a 5-week grocery month, or a December gift budget), reverting to the default
-- automatically for every other month.
CREATE TABLE budget_month_overrides (
  id           INTEGER PRIMARY KEY,
  budget_id    INTEGER NOT NULL REFERENCES budgets(id) ON DELETE CASCADE,
  year         INTEGER NOT NULL,
  month        INTEGER NOT NULL,
  amount_cents INTEGER NOT NULL,
  created_at   TEXT NOT NULL,
  updated_at   TEXT NOT NULL,
  UNIQUE(budget_id, year, month)
);
