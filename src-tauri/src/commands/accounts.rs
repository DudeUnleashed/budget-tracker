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

/// Refuses to delete an account that still has transactions, recurring items, or budgets
/// pointing at it, returning a descriptive count instead of a raw foreign-key error.
#[tauri::command]
pub fn delete_account(db: State<Db>, id: i64) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    delete_account_inner(&conn, id)
}

fn delete_account_inner(conn: &Connection, id: i64) -> Result<(), String> {
    let txn_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM transactions WHERE account_id = ?1", params![id], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    let recurring_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM recurring WHERE account_id = ?1", params![id], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    let budget_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM budgets WHERE account_id = ?1", params![id], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if txn_count > 0 || recurring_count > 0 || budget_count > 0 {
        let mut parts = Vec::new();
        if txn_count > 0 {
            parts.push(format!("{txn_count} transaction{}", if txn_count == 1 { "" } else { "s" }));
        }
        if recurring_count > 0 {
            parts.push(format!("{recurring_count} recurring item{}", if recurring_count == 1 { "" } else { "s" }));
        }
        if budget_count > 0 {
            parts.push(format!("{budget_count} budget{}", if budget_count == 1 { "" } else { "s" }));
        }
        return Err(format!(
            "Can't delete - {} still linked to this account. Move or delete those first.",
            parts.join(" and ")
        ));
    }
    conn.execute("DELETE FROM accounts WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
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
    Ok(starting + txn_sum)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_in_memory_for_test;

    fn seed_account(conn: &Connection, id: i64, starting_balance_cents: i64) {
        conn.execute(
            "INSERT INTO accounts (id, name, ledger, starting_balance_cents, active, is_default, created_at, updated_at)
             VALUES (?1, 'Checking', 'personal', ?2, 1, 1, '2024-01-01T00:00:00', '2024-01-01T00:00:00')",
            params![id, starting_balance_cents],
        )
        .unwrap();
    }

    #[test]
    fn balance_is_starting_balance_plus_transaction_sum() {
        let conn = init_in_memory_for_test();
        seed_account(&conn, 1, 10_000);
        conn.execute_batch(
            "INSERT INTO transactions (id, account_id, date, amount_cents, type, description, created_at, updated_at)
                VALUES (1, 1, '2024-01-05', -2000, 'expense', 'Groceries', '2024-01-05T00:00:00', '2024-01-05T00:00:00');
             INSERT INTO transactions (id, account_id, date, amount_cents, type, description, created_at, updated_at)
                VALUES (2, 1, '2024-01-10', 5000, 'income', 'Refund', '2024-01-10T00:00:00', '2024-01-10T00:00:00');",
        )
        .unwrap();

        assert_eq!(balance_for_account(&conn, 1).unwrap(), 13_000);
    }

    #[test]
    fn delete_refuses_when_transactions_recurring_or_budgets_still_reference_it() {
        let conn = init_in_memory_for_test();
        seed_account(&conn, 1, 0);
        conn.execute(
            "INSERT INTO tags (id, name, color, parent_id) VALUES (1, 'Groceries', NULL, NULL)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO budgets (id, account_id, tag_id, amount_cents, show_on_dashboard, created_at, updated_at)
             VALUES (1, 1, 1, 5000, 1, '2024-01-01T00:00:00', '2024-01-01T00:00:00')",
            [],
        )
        .unwrap();

        let err = delete_account_inner(&conn, 1).unwrap_err();
        assert!(err.contains("budget"), "expected the budget to be mentioned, got: {err}");

        // With the blocking budget gone, the delete should succeed.
        conn.execute("DELETE FROM budgets WHERE id = 1", []).unwrap();
        delete_account_inner(&conn, 1).expect("delete should succeed once nothing references it");
    }
}
