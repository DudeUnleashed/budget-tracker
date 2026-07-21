CREATE TABLE tags (
  id     INTEGER PRIMARY KEY,
  name   TEXT NOT NULL UNIQUE,
  color  TEXT
);

CREATE TABLE accounts (
  id                     INTEGER PRIMARY KEY,
  name                   TEXT NOT NULL,
  ledger                 TEXT NOT NULL,
  starting_balance_cents INTEGER NOT NULL DEFAULT 0,
  active                 INTEGER NOT NULL DEFAULT 1,
  created_at             TEXT NOT NULL,
  updated_at             TEXT NOT NULL
);

CREATE TABLE transactions (
  id               INTEGER PRIMARY KEY,
  account_id       INTEGER NOT NULL REFERENCES accounts(id),
  date             TEXT NOT NULL,
  amount_cents     INTEGER NOT NULL,
  type             TEXT NOT NULL CHECK(type IN ('expense','income','transfer')),
  description      TEXT NOT NULL,
  notes            TEXT,
  linked_group_id  TEXT,
  created_at       TEXT NOT NULL,
  updated_at       TEXT NOT NULL
);
CREATE INDEX idx_transactions_account_date ON transactions(account_id, date);
CREATE INDEX idx_transactions_date ON transactions(date);
CREATE INDEX idx_transactions_linked_group ON transactions(linked_group_id);

CREATE TABLE transaction_tags (
  transaction_id INTEGER NOT NULL REFERENCES transactions(id) ON DELETE CASCADE,
  tag_id         INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
  PRIMARY KEY (transaction_id, tag_id)
);

CREATE TABLE subscriptions (
  id               INTEGER PRIMARY KEY,
  account_id       INTEGER NOT NULL REFERENCES accounts(id),
  name             TEXT NOT NULL,
  amount_cents     INTEGER NOT NULL,
  type             TEXT NOT NULL CHECK(type IN ('expense','income')),
  interval_unit    TEXT NOT NULL CHECK(interval_unit IN ('day','week','month','year')),
  interval_count   INTEGER NOT NULL DEFAULT 1,
  start_date       TEXT NOT NULL,
  next_charge_date TEXT NOT NULL,
  end_date         TEXT,
  active           INTEGER NOT NULL DEFAULT 1,
  paused_until     TEXT,
  notes            TEXT,
  created_at       TEXT NOT NULL,
  updated_at       TEXT NOT NULL
);
CREATE INDEX idx_subscriptions_account ON subscriptions(account_id);

CREATE TABLE subscription_tags (
  subscription_id INTEGER NOT NULL REFERENCES subscriptions(id) ON DELETE CASCADE,
  tag_id          INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
  PRIMARY KEY (subscription_id, tag_id)
);

CREATE TABLE subscription_occurrences (
  id              INTEGER PRIMARY KEY,
  subscription_id INTEGER NOT NULL REFERENCES subscriptions(id) ON DELETE CASCADE,
  due_date        TEXT NOT NULL,
  amount_cents    INTEGER NOT NULL,
  status          TEXT NOT NULL CHECK(status IN ('projected','confirmed','skipped')) DEFAULT 'projected',
  paid_date       TEXT,
  UNIQUE(subscription_id, due_date)
);
CREATE INDEX idx_occurrences_due_date ON subscription_occurrences(due_date);
CREATE INDEX idx_occurrences_subscription ON subscription_occurrences(subscription_id);

CREATE TABLE budgets (
  id           INTEGER PRIMARY KEY,
  ledger       TEXT,
  tag_id       INTEGER REFERENCES tags(id) ON DELETE CASCADE,
  amount_cents INTEGER NOT NULL,
  created_at   TEXT NOT NULL,
  updated_at   TEXT NOT NULL
);
