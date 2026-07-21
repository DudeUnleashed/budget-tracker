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

/// A move between two accounts. If both accounts share the same ledger (e.g. Personal Checking
/// -> Personal Savings) it's recorded as a same-pocket transfer, excluded from income/expense
/// totals. If the accounts are on different ledgers (e.g. Business -> Personal) it's recorded as
/// a linked expense/income pair instead, since real money changed ownership between the two
/// books and should count toward both sides' totals. Either way it's one linked pair of rows
/// sharing tags, decided automatically - the user never has to pick which kind it is.
#[tauri::command]
pub fn create_account_transfer(
    db: State<Db>,
    from_account_id: i64,
    to_account_id: i64,
    date: String,
    amount_cents: i64,
    description: String,
    tag_ids: Vec<i64>,
) -> Result<Vec<Transaction>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    create_account_transfer_inner(&conn, from_account_id, to_account_id, &date, amount_cents, &description, &tag_ids)
}

#[allow(clippy::too_many_arguments)]
fn create_account_transfer_inner(
    conn: &Connection,
    from_account_id: i64,
    to_account_id: i64,
    date: &str,
    amount_cents: i64,
    description: &str,
    tag_ids: &[i64],
) -> Result<Vec<Transaction>, String> {
    let from_ledger: String = conn
        .query_row("SELECT ledger FROM accounts WHERE id = ?1", params![from_account_id], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    let to_ledger: String = conn
        .query_row("SELECT ledger FROM accounts WHERE id = ?1", params![to_account_id], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    let (from_kind, to_kind) = if from_ledger == to_ledger { ("transfer", "transfer") } else { ("expense", "income") };
    let group_id = uuid::Uuid::new_v4().to_string();
    let amt = amount_cents.abs();
    let from_id = insert_transaction_row(
        conn, from_account_id, date, -amt, from_kind, description, None, tag_ids, Some(&group_id),
    )
    .map_err(|e| e.to_string())?;
    let to_id = insert_transaction_row(
        conn, to_account_id, date, amt, to_kind, description, None, tag_ids, Some(&group_id),
    )
    .map_err(|e| e.to_string())?;
    Ok(vec![
        get_transaction(conn, from_id).map_err(|e| e.to_string())?,
        get_transaction(conn, to_id).map_err(|e| e.to_string())?,
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
    tag_id: Option<i64>,
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
    transactions_in_range(&conn, account_id, tag_id, &start, &end).map_err(|e| e.to_string())
}

/// Arbitrary date-range lookup (not necessarily calendar-month-aligned) - powers Recurring
/// cycle drill-down, where cycles can span quarters/years rather than a single month.
#[tauri::command]
pub fn list_transactions_in_range(
    db: State<Db>,
    account_id: Option<i64>,
    tag_id: Option<i64>,
    start: String,
    end: String,
) -> Result<Vec<Transaction>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    transactions_in_range(&conn, account_id, tag_id, &start, &end).map_err(|e| e.to_string())
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
    tag_id: Option<i64>,
    start: &str,
    end: &str,
) -> rusqlite::Result<Vec<Transaction>> {
    let mut stmt = conn.prepare(
        "SELECT DISTINCT t.id, t.account_id, t.date, t.amount_cents, t.type, t.description, t.notes, t.linked_group_id, t.created_at, t.updated_at
         FROM transactions t
         LEFT JOIN transaction_tags tt ON tt.transaction_id = t.id AND tt.tag_id = ?4
         WHERE t.date >= ?1 AND t.date < ?2 AND (?3 IS NULL OR t.account_id = ?3)
           AND (?4 IS NULL OR tt.tag_id IS NOT NULL)
         ORDER BY t.date",
    )?;
    let mut txns: Vec<Transaction> = stmt
        .query_map(params![start, end, account_id, tag_id], row_to_transaction)?
        .collect::<Result<_, _>>()?;
    for t in txns.iter_mut() {
        t.tags = tags_for_transaction(conn, t.id)?;
    }
    Ok(txns)
}

/// Sum of signed amount_cents for transactions matching tag/account within [start, end).
pub(crate) fn sum_in_range(
    conn: &Connection,
    account_id: Option<i64>,
    tag_id: i64,
    start: &str,
    end: &str,
) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT COALESCE(SUM(t.amount_cents), 0)
         FROM transactions t
         JOIN transaction_tags tt ON tt.transaction_id = t.id
         WHERE tt.tag_id = ?1 AND t.date >= ?2 AND t.date < ?3 AND (?4 IS NULL OR t.account_id = ?4)",
        params![tag_id, start, end, account_id],
        |r| r.get(0),
    )
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
        "SELECT t.id, t.name, t.color, t.parent_id FROM tags t
         JOIN transaction_tags tt ON tt.tag_id = t.id
         WHERE tt.transaction_id = ?1 ORDER BY t.name",
    )?;
    let rows = stmt.query_map(params![transaction_id], |row| {
        Ok(Tag {
            id: row.get(0)?,
            name: row.get(1)?,
            color: row.get(2)?,
            parent_id: row.get(3)?,
        })
    })?;
    rows.collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_in_memory_for_test;

    fn seed_account(conn: &Connection, id: i64, name: &str, ledger: &str) {
        conn.execute(
            "INSERT INTO accounts (id, name, ledger, starting_balance_cents, active, is_default, created_at, updated_at)
             VALUES (?1, ?2, ?3, 0, 1, 0, '2024-01-01T00:00:00', '2024-01-01T00:00:00')",
            params![id, name, ledger],
        )
        .unwrap();
    }

    #[test]
    fn same_ledger_transfer_is_excluded_from_income_expense_totals() {
        let conn = init_in_memory_for_test();
        seed_account(&conn, 1, "Checking", "personal");
        seed_account(&conn, 2, "Savings", "personal");

        let rows = create_account_transfer_inner(&conn, 1, 2, "2024-01-05", 5000, "Move to savings", &[]).unwrap();

        assert_eq!(rows[0].kind, "transfer");
        assert_eq!(rows[1].kind, "transfer");
        assert_eq!(rows[0].amount_cents, -5000);
        assert_eq!(rows[1].amount_cents, 5000);
        assert_eq!(rows[0].linked_group_id, rows[1].linked_group_id, "both legs share one group id");
    }

    #[test]
    fn cross_ledger_transfer_becomes_an_expense_income_pair() {
        let conn = init_in_memory_for_test();
        seed_account(&conn, 1, "Business Checking", "business");
        seed_account(&conn, 2, "Personal Checking", "personal");

        let rows = create_account_transfer_inner(&conn, 1, 2, "2024-01-05", 5000, "Owner's draw", &[]).unwrap();

        assert_eq!(rows[0].kind, "expense", "money leaving the business ledger counts as an expense there");
        assert_eq!(rows[1].kind, "income", "money entering the personal ledger counts as income there");
        assert_eq!(rows[0].amount_cents, -5000);
        assert_eq!(rows[1].amount_cents, 5000);
    }

    #[test]
    fn transfer_amount_is_always_normalized_to_a_positive_magnitude() {
        let conn = init_in_memory_for_test();
        seed_account(&conn, 1, "Checking", "personal");
        seed_account(&conn, 2, "Savings", "personal");

        // Passing a negative amount shouldn't flip which side is debited/credited.
        let rows = create_account_transfer_inner(&conn, 1, 2, "2024-01-05", -5000, "Move to savings", &[]).unwrap();
        assert_eq!(rows[0].amount_cents, -5000);
        assert_eq!(rows[1].amount_cents, 5000);
    }
}
