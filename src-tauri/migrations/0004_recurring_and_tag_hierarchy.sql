DROP TABLE IF EXISTS subscription_tags;
DROP TABLE IF EXISTS subscription_occurrences;
DROP TABLE IF EXISTS subscriptions;

ALTER TABLE tags ADD COLUMN parent_id INTEGER REFERENCES tags(id) ON DELETE SET NULL;
CREATE INDEX idx_tags_parent ON tags(parent_id);

-- A tracked expectation, not a generator: never writes a transaction itself. Paid amount and
-- difference are computed live by summing transactions carrying `tag_id` within the current
-- cycle window (derived from anchor_date + cadence), optionally scoped to one account.
CREATE TABLE recurring (
  id                     INTEGER PRIMARY KEY,
  tag_id                 INTEGER NOT NULL UNIQUE REFERENCES tags(id) ON DELETE CASCADE,
  account_id             INTEGER REFERENCES accounts(id),
  type                   TEXT NOT NULL CHECK(type IN ('expense','income')),
  interval_unit          TEXT NOT NULL CHECK(interval_unit IN ('day','week','month','year')),
  interval_count         INTEGER NOT NULL DEFAULT 1,
  anchor_date            TEXT NOT NULL,
  projected_amount_cents INTEGER NOT NULL,
  notes                  TEXT,
  created_at             TEXT NOT NULL,
  updated_at             TEXT NOT NULL
);
