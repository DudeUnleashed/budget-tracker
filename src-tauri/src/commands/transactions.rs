use crate::db::Db;
use crate::models::{Tag, Transaction};
use chrono::Local;
use rusqlite::{params, Connection, Row};
use tauri::State;

#[tauri::command]
pub fn create_transaction(
    db: State<Db>,
    account_id: i64,
    date: String,
    amount_cents: i64,
    kind: String,
    description: String,
    notes: Option<String>,
    tag_ids: Vec<i64>,
) -> Result<Transaction, String> {
    if kind != "expense" && kind != "income" {
        return Err("type must be 'expense' or 'income'".into());
    }
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let signed = if kind == "expense" { -amount_cents.abs() } else { amount_cents.abs() };
    let id = insert_transaction_row(
        &conn,
        account_id,
        &date,
        signed,
        &kind,
        &description,
        notes.as_deref(),
        &tag_ids,
        None,
    )
    .map_err(|e| e.to_string())?;
    get_transaction(&conn, id).map_err(|e| e.to_string())
}

/// A move between two accounts in the same pocket (e.g. Personal Checking -> Personal Savings).
/// Excluded from income/expense totals.
#[tauri::command]
pub fn create_transfer(
    db: State<Db>,
    from_account_id: i64,
    to_account_id: i64,
    date: String,
    amount_cents: i64,
    description: String,
    tag_ids: Vec<i64>,
) -> Result<Vec<Transaction>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let group_id = uuid::Uuid::new_v4().to_string();
    let amt = amount_cents.abs();
    let from_id = insert_transaction_row(
        &conn, from_account_id, &date, -amt, "transfer", &description, None, &tag_ids, Some(&group_id),
    )
    .map_err(|e| e.to_string())?;
    let to_id = insert_transaction_row(
        &conn, to_account_id, &date, amt, "transfer", &description, None, &tag_ids, Some(&group_id),
    )
    .map_err(|e| e.to_string())?;
    Ok(vec![
        get_transaction(&conn, from_id).map_err(|e| e.to_string())?,
        get_transaction(&conn, to_id).map_err(|e| e.to_string())?,
    ])
}

/// A move between ledgers (e.g. Business -> Personal as an Owner's Draw, or Personal -> Business
/// as a Business Loan). Recorded as a linked expense/income pair, not a transfer, since real money
/// changed ownership between the two books.
#[tauri::command]
pub fn create_cross_ledger_movement(
    db: State<Db>,
    from_account_id: i64,
    to_account_id: i64,
    date: String,
    amount_cents: i64,
    description: String,
    tag_id: i64,
) -> Result<Vec<Transaction>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let group_id = uuid::Uuid::new_v4().to_string();
    let amt = amount_cents.abs();
    let from_id = insert_transaction_row(
        &conn, from_account_id, &date, -amt, "expense", &description, None, &[tag_id], Some(&group_id),
    )
    .map_err(|e| e.to_string())?;
    let to_id = insert_transaction_row(
        &conn, to_account_id, &date, amt, "income", &description, None, &[tag_id], Some(&group_id),
    )
    .map_err(|e| e.to_string())?;
    Ok(vec![
        get_transaction(&conn, from_id).map_err(|e| e.to_string())?,
        get_transaction(&conn, to_id).map_err(|e| e.to_string())?,
    ])
}

/// Edits any field of an existing transaction, including ones that are half of a linked
/// pair (transfer / cross-ledger movement) - only this row changes, its counterpart is untouched.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub fn update_transaction(
    db: State<Db>,
    id: i64,
    account_id: i64,
    date: String,
    amount_cents: i64,
    kind: String,
    description: String,
    notes: Option<String>,
    tag_ids: Vec<i64>,
) -> Result<Transaction, String> {
    if kind != "expense" && kind != "income" && kind != "transfer" {
        return Err("type must be 'expense', 'income', or 'transfer'".into());
    }
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let signed = if kind == "transfer" {
        let current: i64 = conn
            .query_row(
                "SELECT amount_cents FROM transactions WHERE id = ?1",
                params![id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if current < 0 { -amount_cents.abs() } else { amount_cents.abs() }
    } else if kind == "expense" {
        -amount_cents.abs()
    } else {
        amount_cents.abs()
    };
    let now = Local::now().to_rfc3339();
    conn.execute(
        "UPDATE transactions SET account_id = ?1, date = ?2, amount_cents = ?3, type = ?4, description = ?5, notes = ?6, updated_at = ?7 WHERE id = ?8",
        params![account_id, date, signed, kind, description, notes, now, id],
    )
    .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM transaction_tags WHERE transaction_id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    for tag_id in &tag_ids {
        conn.execute(
            "INSERT INTO transaction_tags (transaction_id, tag_id) VALUES (?1, ?2)",
            params![id, tag_id],
        )
        .map_err(|e| e.to_string())?;
    }
    get_transaction(&conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_transaction(db: State<Db>, id: i64) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM transactions WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Free-text search across description/notes/tag name, most recent first, independent of month.
/// Powers the standalone Transactions tab (search bar).
#[tauri::command]
pub fn search_transactions(
    db: State<Db>,
    query: Option<String>,
    account_id: Option<i64>,
) -> Result<Vec<Transaction>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let like_query = query
        .as_ref()
        .filter(|q| !q.trim().is_empty())
        .map(|q| format!("%{}%", q.trim()));
    let mut stmt = conn
        .prepare(
            "SELECT DISTINCT t.id, t.account_id, t.date, t.amount_cents, t.type, t.description, t.notes, t.linked_group_id, t.created_at, t.updated_at
             FROM transactions t
             LEFT JOIN transaction_tags tt ON tt.transaction_id = t.id
             LEFT JOIN tags tg ON tg.id = tt.tag_id
             WHERE (?1 IS NULL OR t.description LIKE ?1 ESCAPE '\\' OR t.notes LIKE ?1 ESCAPE '\\' OR tg.name LIKE ?1 ESCAPE '\\')
               AND (?2 IS NULL OR t.account_id = ?2)
             ORDER BY t.date DESC, t.id DESC
             LIMIT 300",
        )
        .map_err(|e| e.to_string())?;
    let mut txns: Vec<Transaction> = stmt
        .query_map(params![like_query, account_id], row_to_transaction)
        .map_err(|e| e.to_string())?
        .collect::<Result<_, _>>()
        .map_err(|e| e.to_string())?;
    for t in txns.iter_mut() {
        t.tags = tags_for_transaction(&conn, t.id).map_err(|e| e.to_string())?;
    }
    Ok(txns)
}

#[tauri::command]
pub fn list_transactions_for_month(
    db: State<Db>,
    account_id: Option<i64>,
    year: i32,
    month: u32,
) -> Result<Vec<Transaction>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let start = format!("{:04}-{:02}-01", year, month);
    let end = if month == 12 {
        format!("{:04}-01-01", year + 1)
    } else {
        format!("{:04}-{:02}-01", year, month + 1)
    };
    transactions_in_range(&conn, account_id, &start, &end).map_err(|e| e.to_string())
}

#[allow(clippy::too_many_arguments)]
fn insert_transaction_row(
    conn: &Connection,
    account_id: i64,
    date: &str,
    signed_amount_cents: i64,
    kind: &str,
    description: &str,
    notes: Option<&str>,
    tag_ids: &[i64],
    linked_group_id: Option<&str>,
) -> rusqlite::Result<i64> {
    let now = Local::now().to_rfc3339();
    conn.execute(
        "INSERT INTO transactions (account_id, date, amount_cents, type, description, notes, linked_group_id, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
        params![account_id, date, signed_amount_cents, kind, description, notes, linked_group_id, now],
    )?;
    let id = conn.last_insert_rowid();
    for tag_id in tag_ids {
        conn.execute(
            "INSERT INTO transaction_tags (transaction_id, tag_id) VALUES (?1, ?2)",
            params![id, tag_id],
        )?;
    }
    Ok(id)
}

pub(crate) fn transactions_in_range(
    conn: &Connection,
    account_id: Option<i64>,
    start: &str,
    end: &str,
) -> rusqlite::Result<Vec<Transaction>> {
    let mut stmt = conn.prepare(
        "SELECT id, account_id, date, amount_cents, type, description, notes, linked_group_id, created_at, updated_at
         FROM transactions
         WHERE date >= ?1 AND date < ?2 AND (?3 IS NULL OR account_id = ?3)
         ORDER BY date",
    )?;
    let mut txns: Vec<Transaction> = stmt
        .query_map(params![start, end, account_id], row_to_transaction)?
        .collect::<Result<_, _>>()?;
    for t in txns.iter_mut() {
        t.tags = tags_for_transaction(conn, t.id)?;
    }
    Ok(txns)
}

fn get_transaction(conn: &Connection, id: i64) -> rusqlite::Result<Transaction> {
    let mut t = conn.query_row(
        "SELECT id, account_id, date, amount_cents, type, description, notes, linked_group_id, created_at, updated_at
         FROM transactions WHERE id = ?1",
        params![id],
        row_to_transaction,
    )?;
    t.tags = tags_for_transaction(conn, id)?;
    Ok(t)
}

fn row_to_transaction(row: &Row) -> rusqlite::Result<Transaction> {
    Ok(Transaction {
        id: row.get(0)?,
        account_id: row.get(1)?,
        date: row.get(2)?,
        amount_cents: row.get(3)?,
        kind: row.get(4)?,
        description: row.get(5)?,
        notes: row.get(6)?,
        linked_group_id: row.get(7)?,
        tags: vec![],
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

fn tags_for_transaction(conn: &Connection, transaction_id: i64) -> rusqlite::Result<Vec<Tag>> {
    let mut stmt = conn.prepare(
        "SELECT t.id, t.name, t.color FROM tags t
         JOIN transaction_tags tt ON tt.tag_id = t.id
         WHERE tt.transaction_id = ?1 ORDER BY t.name",
    )?;
    let rows = stmt.query_map(params![transaction_id], |row| {
        Ok(Tag {
            id: row.get(0)?,
            name: row.get(1)?,
            color: row.get(2)?,
        })
    })?;
    rows.collect()
}
