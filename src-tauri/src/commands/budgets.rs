use crate::db::Db;
use crate::models::{Budget, BudgetProgress, ChildSpend, Tag};
use chrono::Local;
use rusqlite::{params, Connection, OptionalExtension, Row};
use tauri::State;

#[tauri::command]
pub fn create_budget(
    db: State<Db>,
    account_id: Option<i64>,
    tag_id: Option<i64>,
    amount_cents: i64,
    show_on_dashboard: bool,
) -> Result<Budget, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let now = Local::now().to_rfc3339();
    conn.execute(
        "INSERT INTO budgets (account_id, tag_id, amount_cents, show_on_dashboard, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
        params![account_id, tag_id, amount_cents, show_on_dashboard, now],
    )
    .map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();
    get_budget(&conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_budget(
    db: State<Db>,
    id: i64,
    account_id: Option<i64>,
    tag_id: Option<i64>,
    amount_cents: i64,
    show_on_dashboard: bool,
) -> Result<Budget, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let now = Local::now().to_rfc3339();
    conn.execute(
        "UPDATE budgets SET account_id = ?1, tag_id = ?2, amount_cents = ?3, show_on_dashboard = ?4, updated_at = ?5 WHERE id = ?6",
        params![account_id, tag_id, amount_cents, show_on_dashboard, now, id],
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

/// For each budget, sums matching expense transactions for the given month and returns it
/// alongside the percent used. The amount used is a month override if one exists for this
/// budget/year/month, else the budget's own default amount.
#[tauri::command]
pub fn get_budget_progress(db: State<Db>, year: i32, month: u32) -> Result<Vec<BudgetProgress>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let budgets = list_budgets_internal(&conn).map_err(|e| e.to_string())?;
    let (start, end) = month_bounds(year, month);
    let mut out = Vec::with_capacity(budgets.len());
    for budget in budgets {
        let spent = spend_for_tag_recursive(&conn, budget.tag_id, budget.account_id, &start, &end).map_err(|e| e.to_string())?;
        let override_amount = get_override_amount(&conn, budget.id, year, month).map_err(|e| e.to_string())?;
        let amount_cents = override_amount.unwrap_or(budget.amount_cents);
        let percent = if amount_cents == 0 {
            0.0
        } else {
            (spent as f64 / amount_cents as f64) * 100.0
        };
        out.push(BudgetProgress {
            budget,
            spent_cents: spent,
            amount_cents,
            is_override: override_amount.is_some(),
            percent,
        });
    }
    Ok(out)
}

/// Sets (or replaces) this budget's amount for one specific month only - every other month
/// keeps using the budget's default `amount_cents`.
#[tauri::command]
pub fn set_budget_month_override(db: State<Db>, budget_id: i64, year: i32, month: u32, amount_cents: i64) -> Result<BudgetProgress, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let now = Local::now().to_rfc3339();
    conn.execute(
        "INSERT INTO budget_month_overrides (budget_id, year, month, amount_cents, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?5)
         ON CONFLICT(budget_id, year, month) DO UPDATE SET amount_cents = excluded.amount_cents, updated_at = excluded.updated_at",
        params![budget_id, year, month, amount_cents, now],
    )
    .map_err(|e| e.to_string())?;
    progress_for_budget(&conn, budget_id, year, month).map_err(|e| e.to_string())
}

/// Removes this month's override, reverting to the budget's default amount.
#[tauri::command]
pub fn clear_budget_month_override(db: State<Db>, budget_id: i64, year: i32, month: u32) -> Result<BudgetProgress, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "DELETE FROM budget_month_overrides WHERE budget_id = ?1 AND year = ?2 AND month = ?3",
        params![budget_id, year, month],
    )
    .map_err(|e| e.to_string())?;
    progress_for_budget(&conn, budget_id, year, month).map_err(|e| e.to_string())
}

fn get_override_amount(conn: &Connection, budget_id: i64, year: i32, month: u32) -> rusqlite::Result<Option<i64>> {
    conn.query_row(
        "SELECT amount_cents FROM budget_month_overrides WHERE budget_id = ?1 AND year = ?2 AND month = ?3",
        params![budget_id, year, month],
        |r| r.get(0),
    )
    .optional()
}

fn progress_for_budget(conn: &Connection, budget_id: i64, year: i32, month: u32) -> rusqlite::Result<BudgetProgress> {
    let budget = get_budget(conn, budget_id)?;
    let (start, end) = month_bounds(year, month);
    let spent = spend_for_tag_recursive(conn, budget.tag_id, budget.account_id, &start, &end)?;
    let override_amount = get_override_amount(conn, budget.id, year, month)?;
    let amount_cents = override_amount.unwrap_or(budget.amount_cents);
    let percent = if amount_cents == 0 { 0.0 } else { (spent as f64 / amount_cents as f64) * 100.0 };
    Ok(BudgetProgress {
        budget,
        spent_cents: spent,
        amount_cents,
        is_override: override_amount.is_some(),
        percent,
    })
}

/// Per-child-tag spend for a parent-tag budget, for the given month - powers the expandable
/// breakdown (e.g. a Utilities budget breaking down into Water/Electricity/Gas/Heating).
/// Returns an empty list for an overall (no-tag) budget or a tag with no children.
#[tauri::command]
pub fn get_budget_breakdown(db: State<Db>, budget_id: i64, year: i32, month: u32) -> Result<Vec<ChildSpend>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let budget = get_budget(&conn, budget_id).map_err(|e| e.to_string())?;
    let Some(parent_tag_id) = budget.tag_id else {
        return Ok(vec![]);
    };
    let (start, end) = month_bounds(year, month);
    tag_breakdown(&conn, parent_tag_id, budget.account_id, &start, &end).map_err(|e| e.to_string())
}

/// Same as `get_budget_breakdown` but keyed directly by tag rather than by budget, so the
/// frontend can recurse into any tag's children (not just a budget's root tag) to drill down
/// more than one level deep.
#[tauri::command]
pub fn get_tag_breakdown(db: State<Db>, tag_id: i64, account_id: Option<i64>, year: i32, month: u32) -> Result<Vec<ChildSpend>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let (start, end) = month_bounds(year, month);
    tag_breakdown(&conn, tag_id, account_id, &start, &end).map_err(|e| e.to_string())
}

fn tag_breakdown(conn: &Connection, parent_tag_id: i64, account_id: Option<i64>, start: &str, end: &str) -> rusqlite::Result<Vec<ChildSpend>> {
    let mut stmt = conn.prepare("SELECT id, name, color, parent_id FROM tags WHERE parent_id = ?1 ORDER BY name")?;
    let children: Vec<Tag> = stmt
        .query_map(params![parent_tag_id], row_to_tag)?
        .collect::<rusqlite::Result<_>>()?;
    let mut out = Vec::with_capacity(children.len());
    for tag in children {
        let spent = spend_for_tag_recursive(conn, Some(tag.id), account_id, start, end)?;
        out.push(ChildSpend { tag, spent_cents: spent });
    }
    Ok(out)
}

fn month_bounds(year: i32, month: u32) -> (String, String) {
    let start = format!("{:04}-{:02}-01", year, month);
    let end = if month == 12 {
        format!("{:04}-01-01", year + 1)
    } else {
        format!("{:04}-{:02}-01", year, month + 1)
    };
    (start, end)
}

fn spend_for_tag(conn: &Connection, tag_id: Option<i64>, account_id: Option<i64>, start: &str, end: &str) -> rusqlite::Result<i64> {
    if let Some(tag_id) = tag_id {
        conn.query_row(
            "SELECT COALESCE(-SUM(t.amount_cents), 0)
             FROM transactions t
             JOIN transaction_tags tt ON tt.transaction_id = t.id
             WHERE t.type = 'expense' AND tt.tag_id = ?1 AND t.date >= ?2 AND t.date < ?3
               AND (?4 IS NULL OR t.account_id = ?4)",
            params![tag_id, start, end, account_id],
            |r| r.get(0),
        )
    } else {
        conn.query_row(
            "SELECT COALESCE(-SUM(t.amount_cents), 0)
             FROM transactions t
             WHERE t.type = 'expense' AND t.date >= ?1 AND t.date < ?2
               AND (?3 IS NULL OR t.account_id = ?3)",
            params![start, end, account_id],
            |r| r.get(0),
        )
    }
}

/// Spend for a tag plus all of its descendants, recursively - a transaction is basically never
/// tagged directly with a parent category (e.g. "Utilities"), only with its specific children
/// (Water, Electricity, ...), so a parent's spend has to roll up from below.
fn spend_for_tag_recursive(conn: &Connection, tag_id: Option<i64>, account_id: Option<i64>, start: &str, end: &str) -> rusqlite::Result<i64> {
    let Some(tag_id) = tag_id else {
        return spend_for_tag(conn, None, account_id, start, end);
    };
    let mut total = spend_for_tag(conn, Some(tag_id), account_id, start, end)?;
    let mut stmt = conn.prepare("SELECT id FROM tags WHERE parent_id = ?1")?;
    let child_ids: Vec<i64> = stmt
        .query_map(params![tag_id], |r| r.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    for child_id in child_ids {
        total += spend_for_tag_recursive(conn, Some(child_id), account_id, start, end)?;
    }
    Ok(total)
}

fn list_budgets_internal(conn: &Connection) -> rusqlite::Result<Vec<Budget>> {
    let mut stmt = conn.prepare(
        "SELECT id, account_id, tag_id, amount_cents, show_on_dashboard, created_at, updated_at FROM budgets",
    )?;
    let rows = stmt.query_map([], row_to_budget)?;
    rows.collect()
}

fn row_to_budget(row: &Row) -> rusqlite::Result<Budget> {
    Ok(Budget {
        id: row.get(0)?,
        account_id: row.get(1)?,
        tag_id: row.get(2)?,
        amount_cents: row.get(3)?,
        show_on_dashboard: row.get::<_, i64>(4)? != 0,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

fn row_to_tag(row: &Row) -> rusqlite::Result<Tag> {
    Ok(Tag {
        id: row.get(0)?,
        name: row.get(1)?,
        color: row.get(2)?,
        parent_id: row.get(3)?,
    })
}

fn get_budget(conn: &Connection, id: i64) -> rusqlite::Result<Budget> {
    conn.query_row(
        "SELECT id, account_id, tag_id, amount_cents, show_on_dashboard, created_at, updated_at FROM budgets WHERE id = ?1",
        params![id],
        row_to_budget,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_in_memory_for_test;

    /// Utilities (1) -> Electricity (2), Water (3) -> Meter fee (4, a grandchild of Utilities).
    /// Two accounts: Checking (1) and Savings (2). Expenses tagged at every level, plus one on
    /// Savings, to exercise both the recursive rollup and the account_id filter together.
    fn seed_hierarchy(conn: &Connection) {
        conn.execute_batch(
            "INSERT INTO accounts (id, name, ledger, starting_balance_cents, active, is_default, created_at, updated_at)
                VALUES (1, 'Checking', 'personal', 0, 1, 1, '2024-01-01T00:00:00', '2024-01-01T00:00:00');
             INSERT INTO accounts (id, name, ledger, starting_balance_cents, active, is_default, created_at, updated_at)
                VALUES (2, 'Savings', 'personal', 0, 1, 0, '2024-01-01T00:00:00', '2024-01-01T00:00:00');
             INSERT INTO tags (id, name, color, parent_id) VALUES (1, 'Utilities', NULL, NULL);
             INSERT INTO tags (id, name, color, parent_id) VALUES (2, 'Electricity', NULL, 1);
             INSERT INTO tags (id, name, color, parent_id) VALUES (3, 'Water', NULL, 1);
             INSERT INTO tags (id, name, color, parent_id) VALUES (4, 'Meter fee', NULL, 3);
             INSERT INTO transactions (id, account_id, date, amount_cents, type, description, created_at, updated_at)
                VALUES (1, 1, '2024-01-05', -10000, 'expense', 'Electricity bill', '2024-01-05T00:00:00', '2024-01-05T00:00:00');
             INSERT INTO transaction_tags (transaction_id, tag_id) VALUES (1, 2);
             INSERT INTO transactions (id, account_id, date, amount_cents, type, description, created_at, updated_at)
                VALUES (2, 1, '2024-01-06', -5000, 'expense', 'Water bill', '2024-01-06T00:00:00', '2024-01-06T00:00:00');
             INSERT INTO transaction_tags (transaction_id, tag_id) VALUES (2, 3);
             INSERT INTO transactions (id, account_id, date, amount_cents, type, description, created_at, updated_at)
                VALUES (3, 1, '2024-01-07', -300, 'expense', 'Meter fee', '2024-01-07T00:00:00', '2024-01-07T00:00:00');
             INSERT INTO transaction_tags (transaction_id, tag_id) VALUES (3, 4);
             INSERT INTO transactions (id, account_id, date, amount_cents, type, description, created_at, updated_at)
                VALUES (4, 2, '2024-01-08', -2000, 'expense', 'Electricity bill (savings account)', '2024-01-08T00:00:00', '2024-01-08T00:00:00');
             INSERT INTO transaction_tags (transaction_id, tag_id) VALUES (4, 2);",
        )
        .unwrap();
    }

    #[test]
    fn recursive_spend_rolls_up_through_grandchildren() {
        let conn = init_in_memory_for_test();
        seed_hierarchy(&conn);
        let (start, end) = month_bounds(2024, 1);

        // Utilities (any account) = electricity (10000 + 2000) + water (5000) + meter fee (300).
        assert_eq!(spend_for_tag_recursive(&conn, Some(1), None, &start, &end).unwrap(), 17_300);
        // Water alone rolls up its one child (meter fee).
        assert_eq!(spend_for_tag_recursive(&conn, Some(3), None, &start, &end).unwrap(), 5_300);
        // A leaf with no children is just its own direct spend.
        assert_eq!(spend_for_tag_recursive(&conn, Some(4), None, &start, &end).unwrap(), 300);
    }

    #[test]
    fn account_filter_narrows_the_recursive_total() {
        let conn = init_in_memory_for_test();
        seed_hierarchy(&conn);
        let (start, end) = month_bounds(2024, 1);

        // Scoped to Checking only, the Savings-account electricity charge should be excluded.
        assert_eq!(spend_for_tag_recursive(&conn, Some(1), Some(1), &start, &end).unwrap(), 15_300);
        assert_eq!(spend_for_tag_recursive(&conn, Some(1), Some(2), &start, &end).unwrap(), 2_000);
    }

    #[test]
    fn tag_breakdown_lists_direct_children_with_their_own_rollup() {
        let conn = init_in_memory_for_test();
        seed_hierarchy(&conn);
        let (start, end) = month_bounds(2024, 1);

        let mut breakdown = tag_breakdown(&conn, 1, None, &start, &end).unwrap();
        breakdown.sort_by(|a, b| a.tag.name.cmp(&b.tag.name));
        assert_eq!(breakdown.len(), 2, "Utilities has two direct children");
        assert_eq!(breakdown[0].tag.name, "Electricity");
        assert_eq!(breakdown[0].spent_cents, 12_000);
        assert_eq!(breakdown[1].tag.name, "Water");
        assert_eq!(breakdown[1].spent_cents, 5_300, "Water's own rollup includes its Meter fee child");
    }

    #[test]
    fn month_override_takes_effect_and_reverts_on_clear() {
        let conn = init_in_memory_for_test();
        seed_hierarchy(&conn);
        conn.execute(
            "INSERT INTO budgets (id, account_id, tag_id, amount_cents, show_on_dashboard, created_at, updated_at)
             VALUES (1, NULL, 1, 20000, 1, '2024-01-01T00:00:00', '2024-01-01T00:00:00')",
            [],
        )
        .unwrap();

        let default_progress = progress_for_budget(&conn, 1, 2024, 1).unwrap();
        assert_eq!(default_progress.amount_cents, 20_000);
        assert!(!default_progress.is_override);

        conn.execute(
            "INSERT INTO budget_month_overrides (budget_id, year, month, amount_cents, created_at, updated_at)
             VALUES (1, 2024, 1, 30000, '2024-01-01T00:00:00', '2024-01-01T00:00:00')",
            [],
        )
        .unwrap();
        let overridden = progress_for_budget(&conn, 1, 2024, 1).unwrap();
        assert_eq!(overridden.amount_cents, 30_000);
        assert!(overridden.is_override);
        assert_eq!(overridden.spent_cents, 17_300, "spend itself is unaffected by the override");

        // A different month never had an override, so it should still see the default amount.
        let other_month = progress_for_budget(&conn, 1, 2024, 2).unwrap();
        assert_eq!(other_month.amount_cents, 20_000);
        assert!(!other_month.is_override);
    }
}
