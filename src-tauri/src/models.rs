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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub id: i64,
    pub account_id: i64,
    pub name: String,
    pub amount_cents: i64,
    #[serde(rename = "type")]
    pub kind: String,
    pub interval_unit: String,
    pub interval_count: i64,
    pub start_date: String,
    pub next_charge_date: String,
    pub end_date: Option<String>,
    pub active: bool,
    pub paused_until: Option<String>,
    pub notes: Option<String>,
    pub tags: Vec<Tag>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionOccurrence {
    pub id: i64,
    pub subscription_id: i64,
    pub due_date: String,
    /// Signed to match the parent subscription's type.
    pub amount_cents: i64,
    pub status: String,
    pub paid_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Budget {
    pub id: i64,
    pub ledger: Option<String>,
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
    pub percent: f64,
}
