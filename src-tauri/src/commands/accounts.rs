use crate::db::Db;
use crate::models::Account;
use chrono::Local;
use rusqlite::{params, Connection, Row};
use tauri::State;

#[tauri::command]
pub fn create_account(
    db: State<Db>,
    name: String,
    ledger: String,
    starting_balance_cents: i64,
) -> Result<Account, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let now = Local::now().to_rfc3339();
    conn.execute(
        "INSERT INTO accounts (name, ledger, starting_balance_cents, active, created_at, updated_at)
         VALUES (?1, ?2, ?3, 1, ?4, ?4)",
        params![name, ledger, starting_balance_cents, now],
    )
    .map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();
    get_account(&conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_accounts(db: State<Db>) -> Result<Vec<Account>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, name, ledger, starting_balance_cents, active, is_default, created_at, updated_at
             FROM accounts ORDER BY ledger, name",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], row_to_account).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

/// Marks this account as the default (used to pre-select an account when adding new
/// transactions/subscriptions) and unmarks every other account.
#[tauri::command]
pub fn set_default_account(db: State<Db>, id: i64) -> Result<Account, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.execute("UPDATE accounts SET is_default = 0", [])
        .map_err(|e| e.to_string())?;
    conn.execute("UPDATE accounts SET is_default = 1 WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    get_account(&conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_account(
    db: State<Db>,
    id: i64,
    name: String,
    ledger: String,
    starting_balance_cents: i64,
) -> Result<Account, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let now = Local::now().to_rfc3339();
    conn.execute(
        "UPDATE accounts SET name = ?1, ledger = ?2, starting_balance_cents = ?3, updated_at = ?4 WHERE id = ?5",
        params![name, ledger, starting_balance_cents, now, id],
    )
    .map_err(|e| e.to_string())?;
    get_account(&conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_account_balance(db: State<Db>, account_id: i64) -> Result<i64, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    balance_for_account(&conn, account_id).map_err(|e| e.to_string())
}

pub(crate) fn balance_for_account(conn: &Connection, account_id: i64) -> rusqlite::Result<i64> {
    let starting: i64 = conn.query_row(
        "SELECT starting_balance_cents FROM accounts WHERE id = ?1",
        params![account_id],
        |r| r.get(0),
    )?;
    let txn_sum: i64 = conn.query_row(
        "SELECT COALESCE(SUM(amount_cents), 0) FROM transactions WHERE account_id = ?1",
        params![account_id],
        |r| r.get(0),
    )?;
    let occ_sum: i64 = conn.query_row(
        "SELECT COALESCE(SUM(so.amount_cents), 0)
         FROM subscription_occurrences so
         JOIN subscriptions s ON s.id = so.subscription_id
         WHERE s.account_id = ?1 AND so.status = 'confirmed'",
        params![account_id],
        |r| r.get(0),
    )?;
    Ok(starting + txn_sum + occ_sum)
}

fn row_to_account(row: &Row) -> rusqlite::Result<Account> {
    Ok(Account {
        id: row.get(0)?,
        name: row.get(1)?,
        ledger: row.get(2)?,
        starting_balance_cents: row.get(3)?,
        active: row.get::<_, i64>(4)? != 0,
        is_default: row.get::<_, i64>(5)? != 0,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

fn get_account(conn: &Connection, id: i64) -> rusqlite::Result<Account> {
    conn.query_row(
        "SELECT id, name, ledger, starting_balance_cents, active, is_default, created_at, updated_at
         FROM accounts WHERE id = ?1",
        params![id],
        row_to_account,
    )
}
