use crate::commands::transactions::{sum_in_range, transactions_in_range};
use crate::db::Db;
use crate::models::{Recurring, RecurringProgress, Tag, Transaction};
use crate::recurrence::cycle_containing;
use chrono::{Local, NaiveDate};
use rusqlite::{params, Connection, OptionalExtension, Row};
use tauri::State;

#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub fn create_recurring(
    db: State<Db>,
    tag_id: i64,
    account_id: Option<i64>,
    kind: String,
    interval_unit: String,
    interval_count: i64,
    anchor_date: String,
    projected_amount_cents: i64,
    notes: Option<String>,
) -> Result<RecurringProgress, String> {
    if kind != "expense" && kind != "income" {
        return Err("type must be 'expense' or 'income'".into());
    }
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    check_tag_available(&conn, tag_id, None)?;
    let now = Local::now().to_rfc3339();
    conn.execute(
        "INSERT INTO recurring (tag_id, account_id, type, interval_unit, interval_count, anchor_date, projected_amount_cents, notes, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
        params![tag_id, account_id, kind, interval_unit, interval_count, anchor_date, projected_amount_cents.abs(), notes, now],
    )
    .map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();
    get_progress(&conn, id).map_err(|e| e.to_string())
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub fn update_recurring(
    db: State<Db>,
    id: i64,
    tag_id: i64,
    account_id: Option<i64>,
    kind: String,
    interval_unit: String,
    interval_count: i64,
    anchor_date: String,
    projected_amount_cents: i64,
    notes: Option<String>,
) -> Result<RecurringProgress, String> {
    if kind != "expense" && kind != "income" {
        return Err("type must be 'expense' or 'income'".into());
    }
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    check_tag_available(&conn, tag_id, Some(id))?;
    let now = Local::now().to_rfc3339();
    conn.execute(
        "UPDATE recurring SET tag_id = ?1, account_id = ?2, type = ?3, interval_unit = ?4, interval_count = ?5, anchor_date = ?6, projected_amount_cents = ?7, notes = ?8, updated_at = ?9
         WHERE id = ?10",
        params![tag_id, account_id, kind, interval_unit, interval_count, anchor_date, projected_amount_cents.abs(), notes, now, id],
    )
    .map_err(|e| e.to_string())?;
    get_progress(&conn, id).map_err(|e| e.to_string())
}

/// A tag can have at most one Recurring definition (the `recurring.tag_id` column is UNIQUE) -
/// this checks that up front with a friendly message naming the tag, instead of letting a raw
/// SQLite constraint violation reach the user.
fn check_tag_available(conn: &Connection, tag_id: i64, exclude_id: Option<i64>) -> Result<(), String> {
    let existing: Option<i64> = conn
        .query_row(
            "SELECT id FROM recurring WHERE tag_id = ?1 AND (?2 IS NULL OR id != ?2)",
            params![tag_id, exclude_id],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    if existing.is_some() {
        let tag_name: String = conn
            .query_row("SELECT name FROM tags WHERE id = ?1", params![tag_id], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        return Err(format!("\"{tag_name}\" already has a recurring item - edit that one instead of creating a new one."));
    }
    Ok(())
}

/// A real delete - history is untouched since transactions link to the tag, not to this row.
#[tauri::command]
pub fn delete_recurring(db: State<Db>, id: i64) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM recurring WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Pauses or resumes a recurring item without touching its cadence/amount config - a paused item
/// is excluded from budget projections but its transaction history and tag are untouched, and it
/// can be reactivated any time. The alternative (deleting it) loses that config outright.
#[tauri::command]
pub fn set_recurring_active(db: State<Db>, id: i64, active: bool) -> Result<RecurringProgress, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.execute("UPDATE recurring SET active = ?1 WHERE id = ?2", params![active, id])
        .map_err(|e| e.to_string())?;
    get_progress(&conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_recurring(db: State<Db>) -> Result<Vec<RecurringProgress>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, tag_id, account_id, type, interval_unit, interval_count, anchor_date, projected_amount_cents, notes, active, created_at, updated_at
             FROM recurring",
        )
        .map_err(|e| e.to_string())?;
    let ids: Vec<i64> = stmt
        .query_map([], |r| r.get(0))
        .map_err(|e| e.to_string())?
        .collect::<rusqlite::Result<_>>()
        .map_err(|e| e.to_string())?;
    ids.into_iter().map(|id| get_progress(&conn, id).map_err(|e| e.to_string())).collect()
}

/// The transactions that landed in this recurring item's current cycle - i.e. exactly what's
/// summed into its paid amount.
#[tauri::command]
pub fn list_recurring_cycle_transactions(db: State<Db>, id: i64) -> Result<Vec<Transaction>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let r = get_recurring(&conn, id).map_err(|e| e.to_string())?;
    let (start, end) = current_cycle(&r);
    transactions_in_range(&conn, r.account_id, Some(r.tag_id), &start.to_string(), &end.to_string())
        .map_err(|e| e.to_string())
}

fn current_cycle(r: &Recurring) -> (NaiveDate, NaiveDate) {
    cycle_on(r, Local::now().date_naive())
}

fn cycle_on(r: &Recurring, on_date: NaiveDate) -> (NaiveDate, NaiveDate) {
    let anchor = NaiveDate::parse_from_str(&r.anchor_date, "%Y-%m-%d").expect("invalid anchor_date in db");
    cycle_containing(anchor, &r.interval_unit, r.interval_count, on_date)
}

fn get_progress(conn: &Connection, id: i64) -> rusqlite::Result<RecurringProgress> {
    get_progress_on(conn, id, Local::now().date_naive())
}

fn get_progress_on(conn: &Connection, id: i64, on_date: NaiveDate) -> rusqlite::Result<RecurringProgress> {
    let recurring = get_recurring(conn, id)?;
    let tag = get_tag(conn, recurring.tag_id)?;
    let (start, end) = cycle_on(&recurring, on_date);
    let start_str = start.to_string();
    let end_str = end.to_string();
    let raw_sum = sum_in_range(conn, recurring.account_id, recurring.tag_id, &start_str, &end_str)?;
    let paid_cents = if recurring.kind == "expense" { -raw_sum } else { raw_sum };
    let difference_cents = paid_cents - recurring.projected_amount_cents;
    Ok(RecurringProgress {
        recurring,
        tag,
        cycle_start: start_str,
        cycle_end: end_str,
        paid_cents,
        difference_cents,
    })
}

fn get_recurring(conn: &Connection, id: i64) -> rusqlite::Result<Recurring> {
    conn.query_row(
        "SELECT id, tag_id, account_id, type, interval_unit, interval_count, anchor_date, projected_amount_cents, notes, active, created_at, updated_at
         FROM recurring WHERE id = ?1",
        params![id],
        row_to_recurring,
    )
}

fn get_tag(conn: &Connection, id: i64) -> rusqlite::Result<Tag> {
    conn.query_row(
        "SELECT id, name, color, parent_id FROM tags WHERE id = ?1",
        params![id],
        |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
                parent_id: row.get(3)?,
            })
        },
    )
}

fn row_to_recurring(row: &Row) -> rusqlite::Result<Recurring> {
    Ok(Recurring {
        id: row.get(0)?,
        tag_id: row.get(1)?,
        account_id: row.get(2)?,
        kind: row.get(3)?,
        interval_unit: row.get(4)?,
        interval_count: row.get(5)?,
        anchor_date: row.get(6)?,
        projected_amount_cents: row.get(7)?,
        notes: row.get(8)?,
        active: row.get::<_, i64>(9)? != 0,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_in_memory_for_test;

    fn seed(conn: &Connection, kind: &str, projected_amount_cents: i64) {
        conn.execute_batch(&format!(
            "INSERT INTO tags (id, name, color, parent_id) VALUES (1, 'Netflix', NULL, NULL);
             INSERT INTO accounts (id, name, ledger, starting_balance_cents, active, is_default, created_at, updated_at)
                VALUES (1, 'Checking', 'personal', 0, 1, 1, '2024-01-01T00:00:00', '2024-01-01T00:00:00');
             INSERT INTO recurring (id, tag_id, account_id, type, interval_unit, interval_count, anchor_date, projected_amount_cents, created_at, updated_at)
                VALUES (1, 1, 1, '{kind}', 'month', 1, '2024-01-15', {projected_amount_cents}, '2024-01-01T00:00:00', '2024-01-01T00:00:00');"
        ))
        .unwrap();
    }

    fn insert_txn(conn: &Connection, id: i64, date: &str, signed_amount_cents: i64) {
        conn.execute(
            "INSERT INTO transactions (id, account_id, date, amount_cents, type, description, created_at, updated_at)
             VALUES (?1, 1, ?2, ?3, ?4, 'Netflix charge', ?2, ?2)",
            params![id, date, signed_amount_cents, if signed_amount_cents < 0 { "expense" } else { "income" }],
        )
        .unwrap();
        conn.execute("INSERT INTO transaction_tags (transaction_id, tag_id) VALUES (?1, 1)", params![id]).unwrap();
    }

    #[test]
    fn expense_paid_amount_is_positive_and_difference_reflects_overspend() {
        let conn = init_in_memory_for_test();
        seed(&conn, "expense", 1500);
        insert_txn(&conn, 1, "2024-02-20", -2000);

        let on_date = NaiveDate::from_ymd_opt(2024, 2, 25).unwrap();
        let progress = get_progress_on(&conn, 1, on_date).unwrap();

        assert_eq!(progress.cycle_start, "2024-02-15");
        assert_eq!(progress.cycle_end, "2024-03-15");
        assert_eq!(progress.paid_cents, 2000, "expense paid amount should be positive, not the raw negative signed amount");
        assert_eq!(progress.difference_cents, 500, "2000 paid - 1500 projected = 500 over");
    }

    #[test]
    fn income_paid_amount_is_positive_and_ignores_other_cycles() {
        let conn = init_in_memory_for_test();
        seed(&conn, "income", 1000);
        insert_txn(&conn, 1, "2024-02-20", 1000);
        // Outside the Feb 15->Mar 15 cycle entirely - should not be counted.
        insert_txn(&conn, 2, "2024-01-20", 1000);

        let on_date = NaiveDate::from_ymd_opt(2024, 2, 20).unwrap();
        let progress = get_progress_on(&conn, 1, on_date).unwrap();

        assert_eq!(progress.paid_cents, 1000);
        assert_eq!(progress.difference_cents, 0);
    }

    #[test]
    fn cycle_before_anchor_is_the_first_upcoming_cycle() {
        let conn = init_in_memory_for_test();
        seed(&conn, "expense", 1500);

        let before_anchor = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let progress = get_progress_on(&conn, 1, before_anchor).unwrap();

        assert_eq!(progress.cycle_start, "2024-01-15");
        assert_eq!(progress.cycle_end, "2024-02-15");
        assert_eq!(progress.paid_cents, 0);
    }

    #[test]
    fn check_tag_available_names_the_tag_in_a_friendly_message_and_ignores_its_own_row() {
        let conn = init_in_memory_for_test();
        seed(&conn, "expense", 1500);

        let err = check_tag_available(&conn, 1, None).unwrap_err();
        assert!(err.contains("Netflix"), "expected the tag name in the message, got: {err}");

        // Editing the same recurring row (excluding its own id) shouldn't collide with itself.
        check_tag_available(&conn, 1, Some(1)).expect("a row shouldn't conflict with itself");

        // A brand-new tag with no recurring item yet is fine.
        conn.execute("INSERT INTO tags (id, name, color, parent_id) VALUES (2, 'Spotify', NULL, NULL)", []).unwrap();
        check_tag_available(&conn, 2, None).expect("an unused tag should be available");
    }
}
