use crate::db::Db;
use crate::debt_math::{calculate_payoff, DebtPayoffResult};
use crate::models::{Debt, DebtProgress};
use chrono::{Local, NaiveDate};
use rusqlite::{params, Connection, Row};
use tauri::State;

#[tauri::command]
pub fn create_debt(
    db: State<Db>,
    name: String,
    start_date: String,
    principal_cents: i64,
    monthly_payment_cents: i64,
    interest_rate_bps: i64,
) -> Result<DebtProgress, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let now = Local::now().to_rfc3339();
    conn.execute(
        "INSERT INTO debts (name, start_date, principal_cents, monthly_payment_cents, interest_rate_bps, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
        params![name, start_date, principal_cents, monthly_payment_cents, interest_rate_bps, now],
    )
    .map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();
    get_debt_progress(&conn, id)
}

#[tauri::command]
pub fn update_debt(
    db: State<Db>,
    id: i64,
    name: String,
    start_date: String,
    principal_cents: i64,
    monthly_payment_cents: i64,
    interest_rate_bps: i64,
) -> Result<DebtProgress, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let now = Local::now().to_rfc3339();
    conn.execute(
        "UPDATE debts SET name = ?1, start_date = ?2, principal_cents = ?3, monthly_payment_cents = ?4, interest_rate_bps = ?5, updated_at = ?6 WHERE id = ?7",
        params![name, start_date, principal_cents, monthly_payment_cents, interest_rate_bps, now, id],
    )
    .map_err(|e| e.to_string())?;
    get_debt_progress(&conn, id)
}

#[tauri::command]
pub fn delete_debt(db: State<Db>, id: i64) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM debts WHERE id = ?1", params![id]).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn list_debts(db: State<Db>) -> Result<Vec<DebtProgress>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, name, start_date, principal_cents, monthly_payment_cents, interest_rate_bps, created_at, updated_at
             FROM debts ORDER BY created_at",
        )
        .map_err(|e| e.to_string())?;
    let debts: Vec<Debt> = stmt
        .query_map([], row_to_debt)
        .map_err(|e| e.to_string())?
        .collect::<rusqlite::Result<_>>()
        .map_err(|e| e.to_string())?;
    debts.into_iter().map(progress_for).collect()
}

/// Live "what if" preview for a form that hasn't been saved (or hasn't been saved yet with its
/// latest edits) - same math as a stored debt, without touching the database.
#[tauri::command]
pub fn preview_debt_payoff(
    start_date: String,
    principal_cents: i64,
    monthly_payment_cents: i64,
    interest_rate_bps: i64,
) -> Result<DebtPayoffResult, String> {
    let start = NaiveDate::parse_from_str(&start_date, "%Y-%m-%d").map_err(|e| e.to_string())?;
    Ok(calculate_payoff(start, principal_cents, monthly_payment_cents, interest_rate_bps))
}

fn get_debt_progress(conn: &Connection, id: i64) -> Result<DebtProgress, String> {
    let debt = conn
        .query_row(
            "SELECT id, name, start_date, principal_cents, monthly_payment_cents, interest_rate_bps, created_at, updated_at
             FROM debts WHERE id = ?1",
            params![id],
            row_to_debt,
        )
        .map_err(|e| e.to_string())?;
    progress_for(debt)
}

fn progress_for(debt: Debt) -> Result<DebtProgress, String> {
    let start = NaiveDate::parse_from_str(&debt.start_date, "%Y-%m-%d").map_err(|e| e.to_string())?;
    let result = calculate_payoff(start, debt.principal_cents, debt.monthly_payment_cents, debt.interest_rate_bps);
    Ok(DebtProgress {
        debt,
        payoff_date: result.payoff_date,
        months: result.months,
        total_paid_cents: result.total_paid_cents,
        total_interest_cents: result.total_interest_cents,
        never_pays_off: result.never_pays_off,
    })
}

fn row_to_debt(row: &Row) -> rusqlite::Result<Debt> {
    Ok(Debt {
        id: row.get(0)?,
        name: row.get(1)?,
        start_date: row.get(2)?,
        principal_cents: row.get(3)?,
        monthly_payment_cents: row.get(4)?,
        interest_rate_bps: row.get(5)?,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
    })
}
