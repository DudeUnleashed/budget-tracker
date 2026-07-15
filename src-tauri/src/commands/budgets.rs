use crate::db::Db;
use crate::models::{Budget, BudgetProgress};
use chrono::Local;
use rusqlite::{params, Connection, Row};
use tauri::State;

#[tauri::command]
pub fn create_budget(
    db: State<Db>,
    ledger: Option<String>,
    tag_id: Option<i64>,
    amount_cents: i64,
    show_on_dashboard: bool,
) -> Result<Budget, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let now = Local::now().to_rfc3339();
    conn.execute(
        "INSERT INTO budgets (ledger, tag_id, amount_cents, show_on_dashboard, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
        params![ledger, tag_id, amount_cents, show_on_dashboard, now],
    )
    .map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();
    get_budget(&conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_budget(
    db: State<Db>,
    id: i64,
    ledger: Option<String>,
    tag_id: Option<i64>,
    amount_cents: i64,
    show_on_dashboard: bool,
) -> Result<Budget, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let now = Local::now().to_rfc3339();
    conn.execute(
        "UPDATE budgets SET ledger = ?1, tag_id = ?2, amount_cents = ?3, show_on_dashboard = ?4, updated_at = ?5 WHERE id = ?6",
        params![ledger, tag_id, amount_cents, show_on_dashboard, now, id],
    )
    .map_err(|e| e.to_string())?;
    get_budget(&conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_budget(db: State<Db>, id: i64) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM budgets WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn list_budgets(db: State<Db>) -> Result<Vec<Budget>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    list_budgets_internal(&conn).map_err(|e| e.to_string())
}

/// For each budget, sums matching spend (transactions + confirmed subscription occurrences)
/// for the given month and returns it alongside the percent used.
#[tauri::command]
pub fn get_budget_progress(db: State<Db>, year: i32, month: u32) -> Result<Vec<BudgetProgress>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let budgets = list_budgets_internal(&conn).map_err(|e| e.to_string())?;
    let start = format!("{:04}-{:02}-01", year, month);
    let end = if month == 12 {
        format!("{:04}-01-01", year + 1)
    } else {
        format!("{:04}-{:02}-01", year, month + 1)
    };
    let mut out = Vec::with_capacity(budgets.len());
    for budget in budgets {
        let spent = spend_for_budget(&conn, &budget, &start, &end).map_err(|e| e.to_string())?;
        let percent = if budget.amount_cents == 0 {
            0.0
        } else {
            (spent as f64 / budget.amount_cents as f64) * 100.0
        };
        out.push(BudgetProgress {
            budget,
            spent_cents: spent,
            percent,
        });
    }
    Ok(out)
}

fn spend_for_budget(conn: &Connection, budget: &Budget, start: &str, end: &str) -> rusqlite::Result<i64> {
    let txn_spent: i64 = if let Some(tag_id) = budget.tag_id {
        conn.query_row(
            "SELECT COALESCE(-SUM(t.amount_cents), 0)
             FROM transactions t
             JOIN transaction_tags tt ON tt.transaction_id = t.id
             JOIN accounts a ON a.id = t.account_id
             WHERE t.type = 'expense' AND tt.tag_id = ?1 AND t.date >= ?2 AND t.date < ?3
               AND (?4 IS NULL OR a.ledger = ?4)",
            params![tag_id, start, end, budget.ledger],
            |r| r.get(0),
        )?
    } else {
        conn.query_row(
            "SELECT COALESCE(-SUM(t.amount_cents), 0)
             FROM transactions t
             JOIN accounts a ON a.id = t.account_id
             WHERE t.type = 'expense' AND t.date >= ?1 AND t.date < ?2
               AND (?3 IS NULL OR a.ledger = ?3)",
            params![start, end, budget.ledger],
            |r| r.get(0),
        )?
    };

    let occ_spent: i64 = if let Some(tag_id) = budget.tag_id {
        conn.query_row(
            "SELECT COALESCE(-SUM(so.amount_cents), 0)
             FROM subscription_occurrences so
             JOIN subscriptions s ON s.id = so.subscription_id
             JOIN subscription_tags st ON st.subscription_id = s.id
             JOIN accounts a ON a.id = s.account_id
             WHERE s.type = 'expense' AND st.tag_id = ?1 AND so.status = 'confirmed'
               AND so.due_date >= ?2 AND so.due_date < ?3
               AND (?4 IS NULL OR a.ledger = ?4)",
            params![tag_id, start, end, budget.ledger],
            |r| r.get(0),
        )?
    } else {
        conn.query_row(
            "SELECT COALESCE(-SUM(so.amount_cents), 0)
             FROM subscription_occurrences so
             JOIN subscriptions s ON s.id = so.subscription_id
             JOIN accounts a ON a.id = s.account_id
             WHERE s.type = 'expense' AND so.status = 'confirmed'
               AND so.due_date >= ?1 AND so.due_date < ?2
               AND (?3 IS NULL OR a.ledger = ?3)",
            params![start, end, budget.ledger],
            |r| r.get(0),
        )?
    };

    Ok(txn_spent + occ_spent)
}

fn list_budgets_internal(conn: &Connection) -> rusqlite::Result<Vec<Budget>> {
    let mut stmt = conn.prepare(
        "SELECT id, ledger, tag_id, amount_cents, show_on_dashboard, created_at, updated_at FROM budgets",
    )?;
    let rows = stmt.query_map([], row_to_budget)?;
    rows.collect()
}

fn row_to_budget(row: &Row) -> rusqlite::Result<Budget> {
    Ok(Budget {
        id: row.get(0)?,
        ledger: row.get(1)?,
        tag_id: row.get(2)?,
        amount_cents: row.get(3)?,
        show_on_dashboard: row.get::<_, i64>(4)? != 0,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

fn get_budget(conn: &Connection, id: i64) -> rusqlite::Result<Budget> {
    conn.query_row(
        "SELECT id, ledger, tag_id, amount_cents, show_on_dashboard, created_at, updated_at FROM budgets WHERE id = ?1",
        params![id],
        row_to_budget,
    )
}
