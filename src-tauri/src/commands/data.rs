use crate::db::Db;
use csv::{ReaderBuilder, WriterBuilder};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::State;

// Full relational backup/restore, not a bank-statement import - every table round-trips
// exactly (original ids preserved) so a restore reconstructs the database precisely.
// Tag links on transactions are denormalized into a pipe-joined `tag_ids` column to avoid an
// extra join-table file.

#[derive(Serialize, Deserialize)]
struct AccountRow {
    id: i64,
    name: String,
    ledger: String,
    starting_balance_cents: i64,
    active: bool,
    is_default: bool,
    created_at: String,
    updated_at: String,
}

#[derive(Serialize, Deserialize)]
struct TagRow {
    id: i64,
    name: String,
    color: Option<String>,
    parent_id: Option<i64>,
}

#[derive(Serialize, Deserialize)]
struct TransactionRow {
    id: i64,
    account_id: i64,
    date: String,
    amount_cents: i64,
    r#type: String,
    description: String,
    notes: Option<String>,
    linked_group_id: Option<String>,
    tag_ids: String,
    created_at: String,
    updated_at: String,
}

#[derive(Serialize, Deserialize)]
struct RecurringRow {
    id: i64,
    tag_id: i64,
    account_id: Option<i64>,
    r#type: String,
    interval_unit: String,
    interval_count: i64,
    anchor_date: String,
    projected_amount_cents: i64,
    notes: Option<String>,
    active: bool,
    created_at: String,
    updated_at: String,
}

#[derive(Serialize, Deserialize)]
struct BudgetRow {
    id: i64,
    account_id: Option<i64>,
    tag_id: Option<i64>,
    amount_cents: i64,
    show_on_dashboard: bool,
    created_at: String,
    updated_at: String,
}

#[derive(Serialize, Deserialize)]
struct DebtRow {
    id: i64,
    name: String,
    start_date: String,
    principal_cents: i64,
    monthly_payment_cents: i64,
    interest_rate_bps: i64,
    created_at: String,
    updated_at: String,
}

#[derive(Serialize)]
pub struct ImportSummary {
    pub accounts: usize,
    pub tags: usize,
    pub transactions: usize,
    pub recurring: usize,
    pub budgets: usize,
    pub debts: usize,
}

#[tauri::command]
pub fn export_data(db: State<Db>, dir: String) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let dir = Path::new(&dir);
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;

    write_csv(dir.join("accounts.csv"), &accounts_rows(&conn).map_err(|e| e.to_string())?)?;
    write_csv(dir.join("tags.csv"), &tags_rows(&conn).map_err(|e| e.to_string())?)?;
    write_csv(dir.join("transactions.csv"), &transactions_rows(&conn).map_err(|e| e.to_string())?)?;
    write_csv(dir.join("recurring.csv"), &recurring_rows(&conn).map_err(|e| e.to_string())?)?;
    write_csv(dir.join("budgets.csv"), &budgets_rows(&conn).map_err(|e| e.to_string())?)?;
    write_csv(dir.join("debts.csv"), &debts_rows(&conn).map_err(|e| e.to_string())?)?;
    Ok(())
}

/// Restores from a folder produced by `export_data`. Inserts with original ids preserved,
/// wrapped in one transaction, so a failure partway through leaves nothing changed. Intended
/// for restoring into an empty database - importing into a populated one will fail loudly on
/// the first id/uniqueness collision rather than silently duplicating or merging data.
#[tauri::command]
pub fn import_data(db: State<Db>, dir: String) -> Result<ImportSummary, String> {
    let mut conn = db.0.lock().map_err(|e| e.to_string())?;
    let dir = Path::new(&dir);

    let accounts: Vec<AccountRow> = read_csv(dir.join("accounts.csv"))?;
    let tags: Vec<TagRow> = read_csv(dir.join("tags.csv"))?;
    let transactions: Vec<TransactionRow> = read_csv(dir.join("transactions.csv"))?;
    let recurring: Vec<RecurringRow> = read_csv(dir.join("recurring.csv"))?;
    let budgets: Vec<BudgetRow> = read_csv(dir.join("budgets.csv"))?;
    let debts: Vec<DebtRow> = read_csv(dir.join("debts.csv"))?;

    let summary = ImportSummary {
        accounts: accounts.len(),
        tags: tags.len(),
        transactions: transactions.len(),
        recurring: recurring.len(),
        budgets: budgets.len(),
        debts: debts.len(),
    };

    let tx = conn.transaction().map_err(|e| e.to_string())?;
    crate::db::clear_seed_tags_for_fresh_import(&tx)?;

    for a in &accounts {
        tx.execute(
            "INSERT INTO accounts (id, name, ledger, starting_balance_cents, active, is_default, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![a.id, a.name, a.ledger, a.starting_balance_cents, a.active, a.is_default, a.created_at, a.updated_at],
        ).map_err(|e| e.to_string())?;
    }
    // Insert tags with a placeholder name and no parent first, then restore the real name and
    // parent together - a child can be exported before its parent row, so self-referential FKs
    // would otherwise fail mid-loop. Name is uniquified per-row (not just NULLed) alongside
    // parent_id because tag names are now only unique among siblings: leaving every row's parent
    // NULL at once (with its real, possibly-shared-with-a-different-parent name already in
    // place) would collide against the top-level-uniqueness index before pass two ever runs.
    for t in &tags {
        tx.execute(
            "INSERT INTO tags (id, name, color, parent_id) VALUES (?1, ?2, ?3, NULL)",
            params![t.id, format!("__import_tmp_{}", t.id), t.color],
        )
        .map_err(|e| e.to_string())?;
    }
    for t in &tags {
        tx.execute(
            "UPDATE tags SET name = ?1, parent_id = ?2 WHERE id = ?3",
            params![t.name, t.parent_id, t.id],
        )
        .map_err(|e| e.to_string())?;
    }
    for t in &transactions {
        tx.execute(
            "INSERT INTO transactions (id, account_id, date, amount_cents, type, description, notes, linked_group_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![t.id, t.account_id, t.date, t.amount_cents, t.r#type, t.description, t.notes, t.linked_group_id, t.created_at, t.updated_at],
        ).map_err(|e| e.to_string())?;
        for tag_id in parse_ids(&t.tag_ids) {
            tx.execute(
                "INSERT INTO transaction_tags (transaction_id, tag_id) VALUES (?1, ?2)",
                params![t.id, tag_id],
            )
            .map_err(|e| e.to_string())?;
        }
    }
    for r in &recurring {
        tx.execute(
            "INSERT INTO recurring (id, tag_id, account_id, type, interval_unit, interval_count, anchor_date, projected_amount_cents, notes, active, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![r.id, r.tag_id, r.account_id, r.r#type, r.interval_unit, r.interval_count, r.anchor_date, r.projected_amount_cents, r.notes, r.active, r.created_at, r.updated_at],
        ).map_err(|e| e.to_string())?;
    }
    for b in &budgets {
        tx.execute(
            "INSERT INTO budgets (id, account_id, tag_id, amount_cents, show_on_dashboard, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![b.id, b.account_id, b.tag_id, b.amount_cents, b.show_on_dashboard, b.created_at, b.updated_at],
        )
        .map_err(|e| e.to_string())?;
    }
    for d in &debts {
        tx.execute(
            "INSERT INTO debts (id, name, start_date, principal_cents, monthly_payment_cents, interest_rate_bps, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![d.id, d.name, d.start_date, d.principal_cents, d.monthly_payment_cents, d.interest_rate_bps, d.created_at, d.updated_at],
        )
        .map_err(|e| e.to_string())?;
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(summary)
}

/// Deletes every account, tag, transaction, recurring item, budget, and debt - a full reset, not
/// a selective one. Order matters: transactions/recurring/budgets are deleted before the
/// accounts/tags they reference (transaction_tags and budget_month_overrides cascade away
/// automatically), so this never needs to touch the foreign_keys pragma - any real ordering bug
/// here would fail loudly instead of silently leaving orphaned rows.
#[tauri::command]
pub fn wipe_all_data(db: State<Db>) -> Result<(), String> {
    let mut conn = db.0.lock().map_err(|e| e.to_string())?;
    wipe_all_data_inner(&mut conn)
}

fn wipe_all_data_inner(conn: &mut Connection) -> Result<(), String> {
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    for table in ["transactions", "recurring", "budgets", "debts", "tags", "accounts"] {
        tx.execute(&format!("DELETE FROM {table}"), []).map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

fn parse_ids(s: &str) -> Vec<i64> {
    s.split('|').filter(|p| !p.is_empty()).filter_map(|p| p.parse().ok()).collect()
}

fn write_csv<T: Serialize>(path: PathBuf, rows: &[T]) -> Result<(), String> {
    let mut wtr = WriterBuilder::new().from_path(&path).map_err(|e| e.to_string())?;
    for row in rows {
        wtr.serialize(row).map_err(|e| e.to_string())?;
    }
    wtr.flush().map_err(|e| e.to_string())
}

fn read_csv<T: for<'de> Deserialize<'de>>(path: PathBuf) -> Result<Vec<T>, String> {
    let mut rdr = ReaderBuilder::new()
        .from_path(&path)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    rdr.deserialize::<T>()
        .collect::<Result<Vec<T>, _>>()
        .map_err(|e| format!("{}: {e}", path.display()))
}

fn accounts_rows(conn: &Connection) -> rusqlite::Result<Vec<AccountRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, ledger, starting_balance_cents, active, is_default, created_at, updated_at FROM accounts",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(AccountRow {
            id: r.get(0)?,
            name: r.get(1)?,
            ledger: r.get(2)?,
            starting_balance_cents: r.get(3)?,
            active: r.get::<_, i64>(4)? != 0,
            is_default: r.get::<_, i64>(5)? != 0,
            created_at: r.get(6)?,
            updated_at: r.get(7)?,
        })
    })?;
    rows.collect()
}

fn tags_rows(conn: &Connection) -> rusqlite::Result<Vec<TagRow>> {
    let mut stmt = conn.prepare("SELECT id, name, color, parent_id FROM tags")?;
    let rows = stmt.query_map([], |r| {
        Ok(TagRow { id: r.get(0)?, name: r.get(1)?, color: r.get(2)?, parent_id: r.get(3)? })
    })?;
    rows.collect()
}

fn transactions_rows(conn: &Connection) -> rusqlite::Result<Vec<TransactionRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, account_id, date, amount_cents, type, description, notes, linked_group_id, created_at, updated_at FROM transactions",
    )?;
    let mut tag_stmt = conn.prepare("SELECT tag_id FROM transaction_tags WHERE transaction_id = ?1")?;
    #[allow(clippy::type_complexity)]
    let base: Vec<(i64, i64, String, i64, String, String, Option<String>, Option<String>, String, String)> = stmt
        .query_map([], |r| {
            Ok((
                r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?,
                r.get(5)?, r.get(6)?, r.get(7)?, r.get(8)?, r.get(9)?,
            ))
        })?
        .collect::<rusqlite::Result<_>>()?;
    let mut out = Vec::with_capacity(base.len());
    for (id, account_id, date, amount_cents, kind, description, notes, linked_group_id, created_at, updated_at) in base {
        let tag_ids: Vec<i64> = tag_stmt.query_map(params![id], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?;
        out.push(TransactionRow {
            id, account_id, date, amount_cents, r#type: kind, description, notes, linked_group_id,
            tag_ids: tag_ids.iter().map(i64::to_string).collect::<Vec<_>>().join("|"),
            created_at, updated_at,
        });
    }
    Ok(out)
}

fn recurring_rows(conn: &Connection) -> rusqlite::Result<Vec<RecurringRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, tag_id, account_id, type, interval_unit, interval_count, anchor_date, projected_amount_cents, notes, active, created_at, updated_at FROM recurring",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(RecurringRow {
            id: r.get(0)?,
            tag_id: r.get(1)?,
            account_id: r.get(2)?,
            r#type: r.get(3)?,
            interval_unit: r.get(4)?,
            interval_count: r.get(5)?,
            anchor_date: r.get(6)?,
            projected_amount_cents: r.get(7)?,
            notes: r.get(8)?,
            active: r.get::<_, i64>(9)? != 0,
            created_at: r.get(10)?,
            updated_at: r.get(11)?,
        })
    })?;
    rows.collect()
}

fn debts_rows(conn: &Connection) -> rusqlite::Result<Vec<DebtRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, start_date, principal_cents, monthly_payment_cents, interest_rate_bps, created_at, updated_at FROM debts",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(DebtRow {
            id: r.get(0)?,
            name: r.get(1)?,
            start_date: r.get(2)?,
            principal_cents: r.get(3)?,
            monthly_payment_cents: r.get(4)?,
            interest_rate_bps: r.get(5)?,
            created_at: r.get(6)?,
            updated_at: r.get(7)?,
        })
    })?;
    rows.collect()
}

fn budgets_rows(conn: &Connection) -> rusqlite::Result<Vec<BudgetRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, account_id, tag_id, amount_cents, show_on_dashboard, created_at, updated_at FROM budgets",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(BudgetRow {
            id: r.get(0)?,
            account_id: r.get(1)?,
            tag_id: r.get(2)?,
            amount_cents: r.get(3)?,
            show_on_dashboard: r.get::<_, i64>(4)? != 0,
            created_at: r.get(5)?,
            updated_at: r.get(6)?,
        })
    })?;
    rows.collect()
}

#[cfg(test)]
mod tests {
    use super::wipe_all_data_inner;
    use crate::db::init_in_memory_for_test;

    /// Seeds one row in every table this command touches, including a self-referential tag
    /// (parent/child) and a budget month override (cascaded via budgets), then confirms a wipe
    /// leaves every one of them empty with no foreign-key errors along the way.
    #[test]
    fn wipe_all_data_clears_every_table() {
        let mut conn = init_in_memory_for_test();
        conn.execute_batch(
            "INSERT INTO accounts (id, name, ledger, starting_balance_cents, active, is_default, created_at, updated_at)
                VALUES (1, 'Checking', 'personal', 0, 1, 1, '2024-01-01T00:00:00', '2024-01-01T00:00:00');
             INSERT INTO tags (id, name, color, parent_id) VALUES (1, 'Groceries', NULL, NULL);
             INSERT INTO tags (id, name, color, parent_id) VALUES (2, 'Subscriptions', NULL, 1);
             INSERT INTO transactions (id, account_id, date, amount_cents, type, description, created_at, updated_at)
                VALUES (1, 1, '2024-01-01', -1000, 'expense', 'Test', '2024-01-01T00:00:00', '2024-01-01T00:00:00');
             INSERT INTO transaction_tags (transaction_id, tag_id) VALUES (1, 1);
             INSERT INTO recurring (id, tag_id, account_id, type, interval_unit, interval_count, anchor_date, projected_amount_cents, created_at, updated_at)
                VALUES (1, 2, 1, 'expense', 'month', 1, '2024-01-01', 500, '2024-01-01T00:00:00', '2024-01-01T00:00:00');
             INSERT INTO budgets (id, account_id, tag_id, amount_cents, show_on_dashboard, created_at, updated_at)
                VALUES (1, 1, 1, 5000, 1, '2024-01-01T00:00:00', '2024-01-01T00:00:00');
             INSERT INTO budget_month_overrides (budget_id, year, month, amount_cents, created_at, updated_at)
                VALUES (1, 2024, 1, 6000, '2024-01-01T00:00:00', '2024-01-01T00:00:00');
             INSERT INTO debts (id, name, start_date, principal_cents, monthly_payment_cents, interest_rate_bps, created_at, updated_at)
                VALUES (1, 'Car loan', '2024-01-01', 100000, 5000, 999, '2024-01-01T00:00:00', '2024-01-01T00:00:00');",
        )
        .expect("seed every table");

        wipe_all_data_inner(&mut conn).expect("wipe should succeed");

        for table in [
            "accounts",
            "tags",
            "transactions",
            "transaction_tags",
            "recurring",
            "budgets",
            "budget_month_overrides",
            "debts",
        ] {
            let count: i64 = conn
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
                .unwrap_or_else(|e| panic!("counting {table}: {e}"));
            assert_eq!(count, 0, "{table} should be empty after a wipe");
        }
    }
}
