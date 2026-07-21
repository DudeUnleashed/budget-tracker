use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: i64,
    pub name: String,
    pub ledger: String,
    pub starting_balance_cents: i64,
    pub active: bool,
    pub is_default: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub id: i64,
    pub name: String,
    pub color: Option<String>,
    pub parent_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: i64,
    pub account_id: i64,
    pub date: String,
    /// Signed: negative = money out, positive = money in.
    pub amount_cents: i64,
    #[serde(rename = "type")]
    pub kind: String,
    pub description: String,
    pub notes: Option<String>,
    pub linked_group_id: Option<String>,
    pub tags: Vec<Tag>,
    pub created_at: String,
    pub updated_at: String,
}

/// A tracked expectation, not a generator - never writes a transaction. Paid amount and
/// difference are computed live from whatever transactions carry `tag_id`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recurring {
    pub id: i64,
    pub tag_id: i64,
    pub account_id: Option<i64>,
    #[serde(rename = "type")]
    pub kind: String,
    pub interval_unit: String,
    pub interval_count: i64,
    pub anchor_date: String,
    /// Positive magnitude - sign is derived from `kind` when matching against transactions.
    pub projected_amount_cents: i64,
    pub notes: Option<String>,
    /// Paused items (e.g. a paid-off loan, a cancelled subscription) are excluded from budget
    /// projections but keep their history and can be reactivated - the alternative would be
    /// deleting the definition outright and losing its cadence/amount config.
    pub active: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecurringProgress {
    pub recurring: Recurring,
    pub tag: Tag,
    pub cycle_start: String,
    pub cycle_end: String,
    pub paid_cents: i64,
    pub difference_cents: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Budget {
    pub id: i64,
    pub account_id: Option<i64>,
    pub tag_id: Option<i64>,
    pub amount_cents: i64,
    pub show_on_dashboard: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetProgress {
    pub budget: Budget,
    pub spent_cents: i64,
    /// Effective amount for this month - the month override if one exists, else `budget.amount_cents`.
    pub amount_cents: i64,
    pub is_override: bool,
    pub percent: f64,
}

/// One child tag's contribution to a parent-tag budget's breakdown, for a given month.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChildSpend {
    pub tag: Tag,
    pub spent_cents: i64,
}

/// A standalone payoff calculator, not linked to any account or tag - just the inputs needed to
/// project a payoff date and total cost. Each one is entirely independent of any other.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Debt {
    pub id: i64,
    pub name: String,
    pub start_date: String,
    pub principal_cents: i64,
    pub monthly_payment_cents: i64,
    /// Annual rate in hundredths of a percent (e.g. 1999 = 19.99%), to avoid floating point.
    pub interest_rate_bps: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DebtProgress {
    pub debt: Debt,
    pub payoff_date: Option<String>,
    pub months: i64,
    pub total_paid_cents: i64,
    pub total_interest_cents: i64,
    pub never_pays_off: bool,
}
