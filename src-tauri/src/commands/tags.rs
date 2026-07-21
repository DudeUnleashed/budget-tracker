use crate::db::Db;
use crate::models::Tag;
use rusqlite::{params, Connection, Row};
use tauri::State;

#[tauri::command]
pub fn create_tag(
    db: State<Db>,
    name: String,
    color: Option<String>,
    parent_id: Option<i64>,
) -> Result<Tag, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO tags (name, color, parent_id) VALUES (?1, ?2, ?3)",
        params![name, color, parent_id],
    )
    .map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();
    conn.query_row(
        "SELECT id, name, color, parent_id FROM tags WHERE id = ?1",
        params![id],
        row_to_tag,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_tag(
    db: State<Db>,
    id: i64,
    name: String,
    color: Option<String>,
    parent_id: Option<i64>,
) -> Result<Tag, String> {
    validate_parent(id, parent_id)?;
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE tags SET name = ?1, color = ?2, parent_id = ?3 WHERE id = ?4",
        params![name, color, parent_id, id],
    )
    .map_err(|e| e.to_string())?;
    conn.query_row(
        "SELECT id, name, color, parent_id FROM tags WHERE id = ?1",
        params![id],
        row_to_tag,
    )
    .map_err(|e| e.to_string())
}

/// Refuses to delete a tag that still has a recurring item or budget attached, since those would
/// otherwise cascade away entirely (not just get untagged) with no way back. Plain transaction
/// tagging isn't blocked - removing a tag from whatever transactions happen to carry it is the
/// expected, non-destructive part of deleting a category.
#[tauri::command]
pub fn delete_tag(db: State<Db>, id: i64) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    delete_tag_inner(&conn, id)
}

fn delete_tag_inner(conn: &Connection, id: i64) -> Result<(), String> {
    let recurring_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM recurring WHERE tag_id = ?1", params![id], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    let budget_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM budgets WHERE tag_id = ?1", params![id], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if recurring_count > 0 || budget_count > 0 {
        let mut parts = Vec::new();
        if recurring_count > 0 {
            parts.push(format!("{recurring_count} recurring item{}", if recurring_count == 1 { "" } else { "s" }));
        }
        if budget_count > 0 {
            parts.push(format!("{budget_count} budget{}", if budget_count == 1 { "" } else { "s" }));
        }
        return Err(format!(
            "Can't delete - {} still use this category. Remove those first.",
            parts.join(" and ")
        ));
    }
    conn.execute("DELETE FROM tags WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn list_tags(db: State<Db>) -> Result<Vec<Tag>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, name, color, parent_id FROM tags ORDER BY name")
        .map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], row_to_tag).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

fn validate_parent(id: i64, parent_id: Option<i64>) -> Result<(), String> {
    if parent_id == Some(id) {
        return Err("a tag can't be its own parent".into());
    }
    Ok(())
}

fn row_to_tag(row: &Row) -> rusqlite::Result<Tag> {
    Ok(Tag {
        id: row.get(0)?,
        name: row.get(1)?,
        color: row.get(2)?,
        parent_id: row.get(3)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_in_memory_for_test;

    #[test]
    fn delete_allows_plain_transaction_tagging_but_blocks_recurring_and_budgets() {
        let conn = init_in_memory_for_test();
        conn.execute_batch(
            "INSERT INTO tags (id, name, color, parent_id) VALUES (1, 'Groceries', NULL, NULL);
             INSERT INTO accounts (id, name, ledger, starting_balance_cents, active, is_default, created_at, updated_at)
                VALUES (1, 'Checking', 'personal', 0, 1, 1, '2024-01-01T00:00:00', '2024-01-01T00:00:00');
             INSERT INTO transactions (id, account_id, date, amount_cents, type, description, created_at, updated_at)
                VALUES (1, 1, '2024-01-05', -1000, 'expense', 'Groceries run', '2024-01-05T00:00:00', '2024-01-05T00:00:00');
             INSERT INTO transaction_tags (transaction_id, tag_id) VALUES (1, 1);",
        )
        .unwrap();

        // A tag used only by transactions is fine to delete - untagging history isn't destructive.
        delete_tag_inner(&conn, 1).expect("deleting a plainly-tagged category should succeed");
        let remaining_tag_links: i64 = conn
            .query_row("SELECT COUNT(*) FROM transaction_tags", [], |r| r.get(0))
            .unwrap();
        assert_eq!(remaining_tag_links, 0, "the transaction_tags row should cascade away");

        // A tag with a live Recurring definition must be blocked - that data would be lost outright.
        conn.execute(
            "INSERT INTO tags (id, name, color, parent_id) VALUES (2, 'Netflix', NULL, NULL)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO recurring (id, tag_id, account_id, type, interval_unit, interval_count, anchor_date, projected_amount_cents, created_at, updated_at)
             VALUES (1, 2, 1, 'expense', 'month', 1, '2024-01-01', 1500, '2024-01-01T00:00:00', '2024-01-01T00:00:00')",
            [],
        )
        .unwrap();
        let err = delete_tag_inner(&conn, 2).unwrap_err();
        assert!(err.contains("recurring"), "expected the recurring item to be mentioned, got: {err}");
    }

    #[test]
    fn validate_parent_rejects_self_parenting_but_allows_others() {
        assert!(validate_parent(1, Some(1)).is_err());
        assert!(validate_parent(1, Some(2)).is_ok());
        assert!(validate_parent(1, None).is_ok());
    }
}
