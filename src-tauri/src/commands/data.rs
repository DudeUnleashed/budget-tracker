use crate::db::Db;
use csv::{ReaderBuilder, WriterBuilder};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::State;

// Full relational backup/restore, not a bank-statement import - every table round-trips
// exactly (original ids preserved) so a restore reconstructs the database precisely.
// Tag links are denormalized into a pipe-joined `tag_ids` column to avoid extra join-table files.

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
struct SubscriptionRow {
    id: i64,
    account_id: i64,
    name: String,
    amount_cents: i64,
    r#type: String,
    interval_unit: String,
    interval_count: i64,
    start_date: String,
    next_charge_date: String,
    end_date: Option<String>,
    active: bool,
    paused_until: Option<String>,
    notes: Option<String>,
    tag_ids: String,
    created_at: String,
    updated_at: String,
}

#[derive(Serialize, Deserialize)]
struct OccurrenceRow {
    id: i64,
    subscription_id: i64,
    due_date: String,
    amount_cents: i64,
    status: String,
    paid_date: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct BudgetRow {
    id: i64,
    ledger: Option<String>,
    tag_id: Option<i64>,
    amount_cents: i64,
    show_on_dashboard: bool,
    created_at: String,
    updated_at: String,
}

#[derive(Serialize)]
pub struct ImportSummary {
    pub accounts: usize,
    pub tags: usize,
    pub transactions: usize,
    pub subscriptions: usize,
    pub occurrences: usize,
    pub budgets: usize,
}

#[tauri::command]
pub fn export_data(db: State<Db>, dir: String) -> Result<(), String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let dir = Path::new(&dir);
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;

    write_csv(dir.join("accounts.csv"), &accounts_rows(&conn).map_err(|e| e.to_string())?)?;
    write_csv(dir.join("tags.csv"), &tags_rows(&conn).map_err(|e| e.to_string())?)?;
    write_csv(dir.join("transactions.csv"), &transactions_rows(&conn).map_err(|e| e.to_string())?)?;
    write_csv(dir.join("subscriptions.csv"), &subscriptions_rows(&conn).map_err(|e| e.to_string())?)?;
    write_csv(dir.join("subscription_occurrences.csv"), &occurrences_rows(&conn).map_err(|e| e.to_string())?)?;
    write_csv(dir.join("budgets.csv"), &budgets_rows(&conn).map_err(|e| e.to_string())?)?;
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
    let subscriptions: Vec<SubscriptionRow> = read_csv(dir.join("subscriptions.csv"))?;
    let occurrences: Vec<OccurrenceRow> = read_csv(dir.join("subscription_occurrences.csv"))?;
    let budgets: Vec<BudgetRow> = read_csv(dir.join("budgets.csv"))?;

    let summary = ImportSummary {
        accounts: accounts.len(),
        tags: tags.len(),
        transactions: transactions.len(),
        subscriptions: subscriptions.len(),
        occurrences: occurrences.len(),
        budgets: budgets.len(),
    };

    let tx = conn.transaction().map_err(|e| e.to_string())?;

    for a in &accounts {
        tx.execute(
            "INSERT INTO accounts (id, name, ledger, starting_balance_cents, active, is_default, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![a.id, a.name, a.ledger, a.starting_balance_cents, a.active, a.is_default, a.created_at, a.updated_at],
        ).map_err(|e| e.to_string())?;
    }
    for t in &tags {
        tx.execute(
            "INSERT INTO tags (id, name, color) VALUES (?1, ?2, ?3)",
            params![t.id, t.name, t.color],
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
    for s in &subscriptions {
        tx.execute(
            "INSERT INTO subscriptions (id, account_id, name, amount_cents, type, interval_unit, interval_count, start_date, next_charge_date, end_date, active, paused_until, notes, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![s.id, s.account_id, s.name, s.amount_cents, s.r#type, s.interval_unit, s.interval_count, s.start_date, s.next_charge_date, s.end_date, s.active, s.paused_until, s.notes, s.created_at, s.updated_at],
        ).map_err(|e| e.to_string())?;
        for tag_id in parse_ids(&s.tag_ids) {
            tx.execute(
                "INSERT INTO subscription_tags (subscription_id, tag_id) VALUES (?1, ?2)",
                params![s.id, tag_id],
            )
            .map_err(|e| e.to_string())?;
        }
    }
    for o in &occurrences {
        tx.execute(
            "INSERT INTO subscription_occurrences (id, subscription_id, due_date, amount_cents, status, paid_date)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![o.id, o.subscription_id, o.due_date, o.amount_cents, o.status, o.paid_date],
        )
        .map_err(|e| e.to_string())?;
    }
    for b in &budgets {
        tx.execute(
            "INSERT INTO budgets (id, ledger, tag_id, amount_cents, show_on_dashboard, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![b.id, b.ledger, b.tag_id, b.amount_cents, b.show_on_dashboard, b.created_at, b.updated_at],
        )
        .map_err(|e| e.to_string())?;
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(summary)
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
    let mut stmt = conn.prepare("SELECT id, name, color FROM tags")?;
    let rows = stmt.query_map([], |r| {
        Ok(TagRow { id: r.get(0)?, name: r.get(1)?, color: r.get(2)? })
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

fn subscriptions_rows(conn: &Connection) -> rusqlite::Result<Vec<SubscriptionRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, account_id, name, amount_cents, type, interval_unit, interval_count, start_date, next_charge_date, end_date, active, paused_until, notes, created_at, updated_at FROM subscriptions",
    )?;
    let mut tag_stmt = conn.prepare("SELECT tag_id FROM subscription_tags WHERE subscription_id = ?1")?;
    #[allow(clippy::type_complexity)]
    let base: Vec<(i64, i64, String, i64, String, String, i64, String, String, Option<String>, bool, Option<String>, Option<String>, String, String)> = stmt
        .query_map([], |r| {
            Ok((
                r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?,
                r.get(7)?, r.get(8)?, r.get(9)?, r.get::<_, i64>(10)? != 0, r.get(11)?, r.get(12)?,
                r.get(13)?, r.get(14)?,
            ))
        })?
        .collect::<rusqlite::Result<_>>()?;
    let mut out = Vec::with_capacity(base.len());
    for (id, account_id, name, amount_cents, kind, interval_unit, interval_count, start_date, next_charge_date, end_date, active, paused_until, notes, created_at, updated_at) in base {
        let tag_ids: Vec<i64> = tag_stmt.query_map(params![id], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?;
        out.push(SubscriptionRow {
            id, account_id, name, amount_cents, r#type: kind, interval_unit, interval_count,
            start_date, next_charge_date, end_date, active, paused_until, notes,
            tag_ids: tag_ids.iter().map(i64::to_string).collect::<Vec<_>>().join("|"),
            created_at, updated_at,
        });
    }
    Ok(out)
}

fn occurrences_rows(conn: &Connection) -> rusqlite::Result<Vec<OccurrenceRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, subscription_id, due_date, amount_cents, status, paid_date FROM subscription_occurrences",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(OccurrenceRow {
            id: r.get(0)?,
            subscription_id: r.get(1)?,
            due_date: r.get(2)?,
            amount_cents: r.get(3)?,
            status: r.get(4)?,
            paid_date: r.get(5)?,
        })
    })?;
    rows.collect()
}

fn budgets_rows(conn: &Connection) -> rusqlite::Result<Vec<BudgetRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, ledger, tag_id, amount_cents, show_on_dashboard, created_at, updated_at FROM budgets",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(BudgetRow {
            id: r.get(0)?,
            ledger: r.get(1)?,
            tag_id: r.get(2)?,
            amount_cents: r.get(3)?,
            show_on_dashboard: r.get::<_, i64>(4)? != 0,
            created_at: r.get(5)?,
            updated_at: r.get(6)?,
        })
    })?;
    rows.collect()
}
