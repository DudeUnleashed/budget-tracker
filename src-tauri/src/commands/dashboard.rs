use crate::commands::subscriptions::occurrences_in_range;
use crate::commands::transactions::transactions_in_range;
use crate::db::Db;
use crate::models::{SubscriptionOccurrence, Transaction};
use crate::occurrences::auto_confirm_due;
use serde::Serialize;
use tauri::State;

#[derive(Debug, Serialize)]
pub struct MonthSummary {
    pub transactions: Vec<Transaction>,
    pub occurrences: Vec<SubscriptionOccurrence>,
    pub total_income_cents: i64,
    pub total_expense_cents: i64,
    pub net_cents: i64,
}

/// Unified view for a single month: one-off transactions plus subscription occurrences
/// (whether already-confirmed history or still-projected future cycles), combined into totals.
#[tauri::command]
pub fn get_month_summary(db: State<Db>, year: i32, month: u32, account_id: Option<i64>) -> Result<MonthSummary, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    auto_confirm_due(&conn).map_err(|e| e.to_string())?;

    let start = format!("{:04}-{:02}-01", year, month);
    let end = if month == 12 {
        format!("{:04}-01-01", year + 1)
    } else {
        format!("{:04}-{:02}-01", year, month + 1)
    };

    let transactions = transactions_in_range(&conn, account_id, &start, &end).map_err(|e| e.to_string())?;
    let occurrences = occurrences_in_range(&conn, account_id, &start, &end).map_err(|e| e.to_string())?;

    let mut total_income_cents = 0i64;
    let mut total_expense_cents = 0i64;

    for t in &transactions {
        match t.kind.as_str() {
            "income" => total_income_cents += t.amount_cents,
            "expense" => total_expense_cents += -t.amount_cents,
            _ => {} // transfers don't affect income/expense totals
        }
    }
    for o in &occurrences {
        if o.status != "confirmed" {
            continue;
        }
        if o.amount_cents >= 0 {
            total_income_cents += o.amount_cents;
        } else {
            total_expense_cents += -o.amount_cents;
        }
    }

    Ok(MonthSummary {
        transactions,
        occurrences,
        total_income_cents,
        total_expense_cents,
        net_cents: total_income_cents - total_expense_cents,
    })
}
