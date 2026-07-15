use chrono::{Datelike, Local, NaiveDate};
use rusqlite::{params, Connection, Result};

/// How far ahead to keep occurrences materialized for an active subscription.
pub const GENERATION_HORIZON_MONTHS: i32 = 12;

fn last_day_of_month(year: i32, month: u32) -> u32 {
    let (next_year, next_month) = if month == 12 { (year + 1, 1) } else { (year, month + 1) };
    NaiveDate::from_ymd_opt(next_year, next_month, 1)
        .unwrap()
        .pred_opt()
        .unwrap()
        .day()
}

fn add_months_clamped(date: NaiveDate, months: i32) -> NaiveDate {
    let total_months = date.year() * 12 + (date.month() as i32 - 1) + months;
    let year = total_months.div_euclid(12);
    let month = (total_months.rem_euclid(12) + 1) as u32;
    let day = date.day().min(last_day_of_month(year, month));
    NaiveDate::from_ymd_opt(year, month, day).unwrap()
}

/// Advance `date` by one recurrence step of `interval_count` `interval_unit`s.
/// Month/year steps clamp to the last valid day of the target month (e.g. Jan 31 + 1 month -> Feb 28/29).
pub fn next_date(date: NaiveDate, interval_unit: &str, interval_count: i64) -> NaiveDate {
    match interval_unit {
        "day" => date + chrono::Duration::days(interval_count),
        "week" => date + chrono::Duration::days(interval_count * 7),
        "month" => add_months_clamped(date, interval_count as i32),
        "year" => add_months_clamped(date, interval_count as i32 * 12),
        other => panic!("unknown interval_unit: {other}"),
    }
}

struct SubRow {
    amount_cents: i64,
    interval_unit: String,
    interval_count: i64,
    next_charge_date: String,
    end_date: Option<String>,
    active: bool,
}

fn get_sub_row(conn: &Connection, subscription_id: i64) -> Result<SubRow> {
    conn.query_row(
        "SELECT amount_cents, interval_unit, interval_count, next_charge_date, end_date, active
         FROM subscriptions WHERE id = ?1",
        params![subscription_id],
        |row| {
            Ok(SubRow {
                amount_cents: row.get(0)?,
                interval_unit: row.get(1)?,
                interval_count: row.get(2)?,
                next_charge_date: row.get(3)?,
                end_date: row.get(4)?,
                active: row.get::<_, i64>(5)? != 0,
            })
        },
    )
}

/// Materialize occurrences from the subscription's `next_charge_date` up through `horizon`,
/// then advance `next_charge_date` to the first not-yet-generated cycle.
/// No-op for inactive (paused/cancelled) subscriptions.
pub fn generate_occurrences(conn: &Connection, subscription_id: i64, horizon: NaiveDate) -> Result<()> {
    let sub = get_sub_row(conn, subscription_id)?;
    if !sub.active {
        return Ok(());
    }
    let end_date = sub
        .end_date
        .as_deref()
        .map(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d"))
        .transpose()
        .expect("invalid end_date stored in db");
    let mut cursor = NaiveDate::parse_from_str(&sub.next_charge_date, "%Y-%m-%d")
        .expect("invalid next_charge_date stored in db");

    while cursor <= horizon {
        if let Some(end) = end_date {
            if cursor > end {
                break;
            }
        }
        conn.execute(
            "INSERT OR IGNORE INTO subscription_occurrences (subscription_id, due_date, amount_cents, status)
             VALUES (?1, ?2, ?3, 'projected')",
            params![subscription_id, cursor.to_string(), sub.amount_cents],
        )?;
        cursor = next_date(cursor, &sub.interval_unit, sub.interval_count);
    }

    conn.execute(
        "UPDATE subscriptions SET next_charge_date = ?1 WHERE id = ?2",
        params![cursor.to_string(), subscription_id],
    )?;
    Ok(())
}

pub fn default_horizon() -> NaiveDate {
    add_months_clamped(Local::now().date_naive(), GENERATION_HORIZON_MONTHS)
}

/// Flip any `projected` occurrence whose due date has passed to `confirmed`, keeping its
/// existing (possibly already-updated) amount. Still freely editable afterward.
pub fn auto_confirm_due(conn: &Connection) -> Result<()> {
    let today = Local::now().date_naive().to_string();
    conn.execute(
        "UPDATE subscription_occurrences
         SET status = 'confirmed', paid_date = due_date
         WHERE status = 'projected' AND due_date <= ?1",
        params![today],
    )?;
    Ok(())
}

/// Remove not-yet-happened placeholder occurrences, used when pausing or cancelling a
/// subscription so it truly disappears from future projections. History is untouched.
pub fn clear_future_projected(conn: &Connection, subscription_id: i64) -> Result<()> {
    let today = Local::now().date_naive().to_string();
    conn.execute(
        "DELETE FROM subscription_occurrences
         WHERE subscription_id = ?1 AND status = 'projected' AND due_date > ?2",
        params![subscription_id, today],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn month_step_clamps_short_months() {
        let jan31 = NaiveDate::from_ymd_opt(2026, 1, 31).unwrap();
        assert_eq!(next_date(jan31, "month", 1), NaiveDate::from_ymd_opt(2026, 2, 28).unwrap());
    }

    #[test]
    fn month_step_preserves_day_when_valid() {
        let jan15 = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
        assert_eq!(next_date(jan15, "month", 3), NaiveDate::from_ymd_opt(2026, 4, 15).unwrap());
    }

    #[test]
    fn year_step_clamps_leap_day() {
        let leap_day = NaiveDate::from_ymd_opt(2024, 2, 29).unwrap();
        assert_eq!(next_date(leap_day, "year", 1), NaiveDate::from_ymd_opt(2025, 2, 28).unwrap());
    }

    #[test]
    fn week_and_day_steps() {
        let d = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();
        assert_eq!(next_date(d, "day", 10), NaiveDate::from_ymd_opt(2026, 3, 11).unwrap());
        assert_eq!(next_date(d, "week", 2), NaiveDate::from_ymd_opt(2026, 3, 15).unwrap());
    }

    #[test]
    fn month_step_across_year_boundary() {
        let nov30 = NaiveDate::from_ymd_opt(2026, 11, 30).unwrap();
        assert_eq!(next_date(nov30, "month", 3), NaiveDate::from_ymd_opt(2027, 2, 28).unwrap());
    }
}
