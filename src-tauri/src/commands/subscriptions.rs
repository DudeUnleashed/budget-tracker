use crate::db::Db;
use crate::models::{Subscription, SubscriptionOccurrence, Tag};
use crate::occurrences::{auto_confirm_due, clear_future_projected, default_horizon, generate_occurrences};
use chrono::{Local, NaiveDate};
use rusqlite::{params, Connection, Row};
use tauri::State;

#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub fn create_subscription(
    db: State<Db>,
    account_id: i64,
    name: String,
    amount_cents: i64,
    kind: String,
    interval_unit: String,
    interval_count: i64,
    start_date: String,
    end_date: Option<String>,
    notes: Option<String>,
    tag_ids: Vec<i64>,
) -> Result<Subscription, String> {
    if kind != "expense" && kind != "income" {
        return Err("type must be 'expense' or 'income'".into());
    }
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let signed = if kind == "expense" { -amount_cents.abs() } else { amount_cents.abs() };
    let now = Local::now().to_rfc3339();
    conn.execute(
        "INSERT INTO subscriptions
            (account_id, name, amount_cents, type, interval_unit, interval_count, start_date, next_charge_date, end_date, active, paused_until, notes, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7, ?8, 1, NULL, ?9, ?10, ?10)",
        params![account_id, name, signed, kind, interval_unit, interval_count, start_date, end_date, notes, now],
    )
    .map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();
    for tag_id in &tag_ids {
        conn.execute(
            "INSERT INTO subscription_tags (subscription_id, tag_id) VALUES (?1, ?2)",
            params![id, tag_id],
        )
        .map_err(|e| e.to_string())?;
    }
    generate_occurrences(&conn, id, default_horizon()).map_err(|e| e.to_string())?;
    get_subscription(&conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_subscriptions(db: State<Db>, active_only: bool) -> Result<Vec<Subscription>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    auto_confirm_due(&conn).map_err(|e| e.to_string())?;
    let query = if active_only {
        "SELECT id, account_id, name, amount_cents, type, interval_unit, interval_count, start_date, next_charge_date, end_date, active, paused_until, notes, created_at, updated_at
         FROM subscriptions WHERE active = 1 ORDER BY name"
    } else {
        "SELECT id, account_id, name, amount_cents, type, interval_unit, interval_count, start_date, next_charge_date, end_date, active, paused_until, notes, created_at, updated_at
         FROM subscriptions ORDER BY name"
    };
    let mut stmt = conn.prepare(query).map_err(|e| e.to_string())?;
    let mut subs: Vec<Subscription> = stmt
        .query_map([], row_to_subscription)
        .map_err(|e| e.to_string())?
        .collect::<Result<_, _>>()
        .map_err(|e| e.to_string())?;
    for s in subs.iter_mut() {
        s.tags = tags_for_subscription(&conn, s.id).map_err(|e| e.to_string())?;
    }
    Ok(subs)
}

/// Sets the new default rate. Past/confirmed occurrences keep their original amount;
/// only occurrences still `projected` (not yet happened) are bumped to the new rate.
#[tauri::command]
pub fn update_subscription_amount(db: State<Db>, id: i64, amount_cents: i64) -> Result<Subscription, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let kind: String = conn
        .query_row("SELECT type FROM subscriptions WHERE id = ?1", params![id], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    let signed = if kind == "expense" { -amount_cents.abs() } else { amount_cents.abs() };
    let now = Local::now().to_rfc3339();
    conn.execute(
        "UPDATE subscriptions SET amount_cents = ?1, updated_at = ?2 WHERE id = ?3",
        params![signed, now, id],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE subscription_occurrences SET amount_cents = ?1 WHERE subscription_id = ?2 AND status = 'projected'",
        params![signed, id],
    )
    .map_err(|e| e.to_string())?;
    get_subscription(&conn, id).map_err(|e| e.to_string())
}

/// Edits the rule itself: name, account, amount, and cadence, plus tags. `type` (expense/income)
/// is intentionally not editable here - that's really a different subscription.
/// If the cadence changed, not-yet-happened `projected` occurrences are cleared and regenerated
/// under the new interval starting from the existing `next_charge_date`; history is untouched.
/// If only the amount changed, pending `projected` occurrences are bumped to the new rate
/// (same rule as `update_subscription_amount`).
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub fn update_subscription(
    db: State<Db>,
    id: i64,
    account_id: i64,
    name: String,
    amount_cents: i64,
    interval_unit: String,
    interval_count: i64,
    tag_ids: Vec<i64>,
) -> Result<Subscription, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let kind: String = conn
        .query_row("SELECT type FROM subscriptions WHERE id = ?1", params![id], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    let signed_amount = if kind == "expense" { -amount_cents.abs() } else { amount_cents.abs() };

    let (old_interval_unit, old_interval_count): (String, i64) = conn
        .query_row(
            "SELECT interval_unit, interval_count FROM subscriptions WHERE id = ?1",
            params![id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|e| e.to_string())?;
    let cadence_changed = old_interval_unit != interval_unit || old_interval_count != interval_count;

    let now = Local::now().to_rfc3339();
    conn.execute(
        "UPDATE subscriptions SET account_id = ?1, name = ?2, amount_cents = ?3, interval_unit = ?4, interval_count = ?5, updated_at = ?6 WHERE id = ?7",
        params![account_id, name, signed_amount, interval_unit, interval_count, now, id],
    )
    .map_err(|e| e.to_string())?;

    conn.execute("DELETE FROM subscription_tags WHERE subscription_id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    for tag_id in &tag_ids {
        conn.execute(
            "INSERT INTO subscription_tags (subscription_id, tag_id) VALUES (?1, ?2)",
            params![id, tag_id],
        )
        .map_err(|e| e.to_string())?;
    }

    if cadence_changed {
        clear_future_projected(&conn, id).map_err(|e| e.to_string())?;
        generate_occurrences(&conn, id, default_horizon()).map_err(|e| e.to_string())?;
    } else {
        conn.execute(
            "UPDATE subscription_occurrences SET amount_cents = ?1 WHERE subscription_id = ?2 AND status = 'projected'",
            params![signed_amount, id],
        )
        .map_err(|e| e.to_string())?;
    }

    get_subscription(&conn, id).map_err(|e| e.to_string())
}

/// Pauses starting now. `paused_until` defaults to the next scheduled charge date if not given,
/// and is what drives the "resume or stay paused?" prompt once that date arrives. Any
/// already-materialized future (not-yet-happened) occurrences are cleared.
#[tauri::command]
pub fn pause_subscription(db: State<Db>, id: i64, paused_until: Option<String>) -> Result<Subscription, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let resume_hint = match paused_until {
        Some(d) => d,
        None => conn
            .query_row(
                "SELECT next_charge_date FROM subscriptions WHERE id = ?1",
                params![id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?,
    };
    let now = Local::now().to_rfc3339();
    conn.execute(
        "UPDATE subscriptions SET active = 0, paused_until = ?1, updated_at = ?2 WHERE id = ?3",
        params![resume_hint, now, id],
    )
    .map_err(|e| e.to_string())?;
    clear_future_projected(&conn, id).map_err(|e| e.to_string())?;
    get_subscription(&conn, id).map_err(|e| e.to_string())
}

/// Cancels indefinitely and removes it from all future predictions. The subscription row and
/// its history are kept (not deleted) so it can be reactivated later without re-entering details.
#[tauri::command]
pub fn cancel_subscription(db: State<Db>, id: i64) -> Result<Subscription, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let now = Local::now().to_rfc3339();
    conn.execute(
        "UPDATE subscriptions SET active = 0, paused_until = NULL, updated_at = ?1 WHERE id = ?2",
        params![now, id],
    )
    .map_err(|e| e.to_string())?;
    clear_future_projected(&conn, id).map_err(|e| e.to_string())?;
    get_subscription(&conn, id).map_err(|e| e.to_string())
}

/// Resumes a paused/cancelled subscription. Optionally re-anchors the billing cadence to a new
/// `next_charge_date` (e.g. cancelled billing on the 15th, restarting on the 3rd instead).
#[tauri::command]
pub fn reactivate_subscription(
    db: State<Db>,
    id: i64,
    next_charge_date: Option<String>,
) -> Result<Subscription, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    let now = Local::now().to_rfc3339();
    if let Some(date) = &next_charge_date {
        NaiveDate::parse_from_str(date, "%Y-%m-%d").map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE subscriptions SET next_charge_date = ?1 WHERE id = ?2",
            params![date, id],
        )
        .map_err(|e| e.to_string())?;
    }
    conn.execute(
        "UPDATE subscriptions SET active = 1, paused_until = NULL, updated_at = ?1 WHERE id = ?2",
        params![now, id],
    )
    .map_err(|e| e.to_string())?;
    generate_occurrences(&conn, id, default_horizon()).map_err(|e| e.to_string())?;
    get_subscription(&conn, id).map_err(|e| e.to_string())
}

/// Manually edit a single billing cycle - e.g. the real charge differed from the projected
/// amount, or you want to explicitly confirm/skip/mark-paid a cycle.
#[tauri::command]
pub fn update_occurrence(
    db: State<Db>,
    id: i64,
    amount_cents: Option<i64>,
    status: Option<String>,
    paid_date: Option<String>,
) -> Result<SubscriptionOccurrence, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    if let Some(amount) = amount_cents {
        let current: i64 = conn
            .query_row(
                "SELECT amount_cents FROM subscription_occurrences WHERE id = ?1",
                params![id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        let signed = if current < 0 { -amount.abs() } else { amount.abs() };
        conn.execute(
            "UPDATE subscription_occurrences SET amount_cents = ?1 WHERE id = ?2",
            params![signed, id],
        )
        .map_err(|e| e.to_string())?;
    }
    if let Some(s) = &status {
        conn.execute(
            "UPDATE subscription_occurrences SET status = ?1 WHERE id = ?2",
            params![s, id],
        )
        .map_err(|e| e.to_string())?;
    }
    if let Some(pd) = &paid_date {
        conn.execute(
            "UPDATE subscription_occurrences SET paid_date = ?1 WHERE id = ?2",
            params![pd, id],
        )
        .map_err(|e| e.to_string())?;
    }
    get_occurrence(&conn, id).map_err(|e| e.to_string())
}

pub(crate) fn occurrences_in_range(
    conn: &Connection,
    account_id: Option<i64>,
    start: &str,
    end: &str,
) -> rusqlite::Result<Vec<SubscriptionOccurrence>> {
    let mut stmt = conn.prepare(
        "SELECT so.id, so.subscription_id, so.due_date, so.amount_cents, so.status, so.paid_date
         FROM subscription_occurrences so
         JOIN subscriptions s ON s.id = so.subscription_id
         WHERE so.due_date >= ?1 AND so.due_date < ?2 AND so.status != 'skipped'
           AND (?3 IS NULL OR s.account_id = ?3)
         ORDER BY so.due_date",
    )?;
    let rows = stmt.query_map(params![start, end, account_id], row_to_occurrence)?;
    rows.collect()
}

fn row_to_subscription(row: &Row) -> rusqlite::Result<Subscription> {
    Ok(Subscription {
        id: row.get(0)?,
        account_id: row.get(1)?,
        name: row.get(2)?,
        amount_cents: row.get(3)?,
        kind: row.get(4)?,
        interval_unit: row.get(5)?,
        interval_count: row.get(6)?,
        start_date: row.get(7)?,
        next_charge_date: row.get(8)?,
        end_date: row.get(9)?,
        active: row.get::<_, i64>(10)? != 0,
        paused_until: row.get(11)?,
        notes: row.get(12)?,
        tags: vec![],
        created_at: row.get(13)?,
        updated_at: row.get(14)?,
    })
}

fn row_to_occurrence(row: &Row) -> rusqlite::Result<SubscriptionOccurrence> {
    Ok(SubscriptionOccurrence {
        id: row.get(0)?,
        subscription_id: row.get(1)?,
        due_date: row.get(2)?,
        amount_cents: row.get(3)?,
        status: row.get(4)?,
        paid_date: row.get(5)?,
    })
}

fn get_subscription(conn: &Connection, id: i64) -> rusqlite::Result<Subscription> {
    let mut s = conn.query_row(
        "SELECT id, account_id, name, amount_cents, type, interval_unit, interval_count, start_date, next_charge_date, end_date, active, paused_until, notes, created_at, updated_at
         FROM subscriptions WHERE id = ?1",
        params![id],
        row_to_subscription,
    )?;
    s.tags = tags_for_subscription(conn, id)?;
    Ok(s)
}

fn get_occurrence(conn: &Connection, id: i64) -> rusqlite::Result<SubscriptionOccurrence> {
    conn.query_row(
        "SELECT id, subscription_id, due_date, amount_cents, status, paid_date FROM subscription_occurrences WHERE id = ?1",
        params![id],
        row_to_occurrence,
    )
}

fn tags_for_subscription(conn: &Connection, subscription_id: i64) -> rusqlite::Result<Vec<Tag>> {
    let mut stmt = conn.prepare(
        "SELECT t.id, t.name, t.color FROM tags t
         JOIN subscription_tags st ON st.tag_id = t.id
         WHERE st.subscription_id = ?1 ORDER BY t.name",
    )?;
    let rows = stmt.query_map(params![subscription_id], |row| {
        Ok(Tag {
            id: row.get(0)?,
            name: row.get(1)?,
            color: row.get(2)?,
        })
    })?;
    rows.collect()
}
