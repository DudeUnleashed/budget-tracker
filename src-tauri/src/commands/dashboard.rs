use crate::commands::transactions::transactions_in_range;
use crate::db::Db;
use crate::models::Transaction;
use rusqlite::{params, Connection};
use serde::Serialize;
use tauri::State;

#[derive(Debug, Serialize)]
pub struct MonthSummary {
    pub transactions: Vec<Transaction>,
    pub total_income_cents: i64,
    pub total_expense_cents: i64,
    pub net_cents: i64,
}

/// Unified view for a single month - purely what was actually entered, nothing auto-generated.
#[tauri::command]
pub fn get_month_summary(db: State<Db>, year: i32, month: u32, account_id: Option<i64>) -> Result<MonthSummary, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    get_month_summary_inner(&conn, year, month, account_id)
}

fn get_month_summary_inner(conn: &Connection, year: i32, month: u32, account_id: Option<i64>) -> Result<MonthSummary, String> {
    let (start, end) = month_bounds(year, month);

    let transactions = transactions_in_range(conn, account_id, None, &start, &end).map_err(|e| e.to_string())?;

    let mut total_income_cents = 0i64;
    let mut total_expense_cents = 0i64;

    for t in &transactions {
        match t.kind.as_str() {
            "income" => total_income_cents += t.amount_cents,
            "expense" => total_expense_cents += -t.amount_cents,
            _ => {} // transfers don't affect income/expense totals
        }
    }

    Ok(MonthSummary {
        transactions,
        total_income_cents,
        total_expense_cents,
        net_cents: total_income_cents - total_expense_cents,
    })
}

#[derive(Debug, Serialize)]
pub struct MonthTotal {
    pub year: i32,
    pub month: u32,
    pub total_income_cents: i64,
    pub total_expense_cents: i64,
    pub net_cents: i64,
}

/// Income/expense/net for each of the `months` calendar months ending at (and including)
/// year/month, oldest first - powers the Dashboard's trend chart. Uses plain aggregate sums
/// rather than `transactions_in_range` (which also fetches each transaction's tags) since only
/// the totals are needed here.
#[tauri::command]
pub fn get_monthly_trend(db: State<Db>, year: i32, month: u32, months: u32, account_id: Option<i64>) -> Result<Vec<MonthTotal>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    get_monthly_trend_inner(&conn, year, month, months, account_id)
}

fn get_monthly_trend_inner(
    conn: &Connection,
    year: i32,
    month: u32,
    months: u32,
    account_id: Option<i64>,
) -> Result<Vec<MonthTotal>, String> {
    let mut points = Vec::with_capacity(months as usize);
    let (mut y, mut m) = (year, month);
    for _ in 0..months {
        let (total_income_cents, total_expense_cents) = month_totals(conn, y, m, account_id).map_err(|e| e.to_string())?;
        points.push(MonthTotal {
            year: y,
            month: m,
            total_income_cents,
            total_expense_cents,
            net_cents: total_income_cents - total_expense_cents,
        });
        if m == 1 {
            y -= 1;
            m = 12;
        } else {
            m -= 1;
        }
    }
    points.reverse();
    Ok(points)
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

fn month_totals(conn: &Connection, year: i32, month: u32, account_id: Option<i64>) -> rusqlite::Result<(i64, i64)> {
    let (start, end) = month_bounds(year, month);
    let income_cents: i64 = conn.query_row(
        "SELECT COALESCE(SUM(amount_cents), 0) FROM transactions
         WHERE type = 'income' AND date >= ?1 AND date < ?2 AND (?3 IS NULL OR account_id = ?3)",
        params![start, end, account_id],
        |r| r.get(0),
    )?;
    let expense_raw_cents: i64 = conn.query_row(
        "SELECT COALESCE(SUM(amount_cents), 0) FROM transactions
         WHERE type = 'expense' AND date >= ?1 AND date < ?2 AND (?3 IS NULL OR account_id = ?3)",
        params![start, end, account_id],
        |r| r.get(0),
    )?;
    Ok((income_cents, -expense_raw_cents))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_in_memory_for_test;
    use rusqlite::params;

    fn seed_account(conn: &Connection, id: i64, ledger: &str) {
        conn.execute(
            "INSERT INTO accounts (id, name, ledger, starting_balance_cents, active, is_default, created_at, updated_at)
             VALUES (?1, 'Checking', ?2, 0, 1, 0, '2024-01-01T00:00:00', '2024-01-01T00:00:00')",
            params![id, ledger],
        )
        .unwrap();
    }

    fn insert_txn(conn: &Connection, id: i64, account_id: i64, date: &str, amount_cents: i64, kind: &str) {
        conn.execute(
            "INSERT INTO transactions (id, account_id, date, amount_cents, type, description, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 'Test', ?3, ?3)",
            params![id, account_id, date, amount_cents, kind],
        )
        .unwrap();
    }

    #[test]
    fn totals_exclude_transfers_and_out_of_month_transactions() {
        let conn = init_in_memory_for_test();
        seed_account(&conn, 1, "personal");
        seed_account(&conn, 2, "personal");
        insert_txn(&conn, 1, 1, "2024-02-05", 300_00, "income");
        insert_txn(&conn, 2, 1, "2024-02-10", -120_00, "expense");
        insert_txn(&conn, 3, 1, "2024-02-15", -50_00, "transfer");
        insert_txn(&conn, 4, 1, "2024-01-31", -999_00, "expense"); // previous month, excluded
        insert_txn(&conn, 5, 1, "2024-03-01", -999_00, "expense"); // next month, excluded

        let summary = get_month_summary_inner(&conn, 2024, 2, None).unwrap();

        assert_eq!(summary.transactions.len(), 3, "only February's three rows should be included");
        assert_eq!(summary.total_income_cents, 300_00);
        assert_eq!(summary.total_expense_cents, 120_00, "the transfer should not count as an expense");
        assert_eq!(summary.net_cents, 180_00);
    }

    #[test]
    fn account_filter_narrows_the_summary() {
        let conn = init_in_memory_for_test();
        seed_account(&conn, 1, "personal");
        seed_account(&conn, 2, "business");
        insert_txn(&conn, 1, 1, "2024-02-05", 100_00, "income");
        insert_txn(&conn, 2, 2, "2024-02-05", 500_00, "income");

        let summary = get_month_summary_inner(&conn, 2024, 2, Some(1)).unwrap();
        assert_eq!(summary.transactions.len(), 1);
        assert_eq!(summary.total_income_cents, 100_00);
    }

    #[test]
    fn december_rolls_over_to_next_january_for_the_month_boundary() {
        let conn = init_in_memory_for_test();
        seed_account(&conn, 1, "personal");
        insert_txn(&conn, 1, 1, "2024-12-31", -50_00, "expense");
        insert_txn(&conn, 2, 1, "2025-01-01", -999_00, "expense"); // should be excluded

        let summary = get_month_summary_inner(&conn, 2024, 12, None).unwrap();
        assert_eq!(summary.transactions.len(), 1);
        assert_eq!(summary.total_expense_cents, 50_00);
    }

    #[test]
    fn monthly_trend_spans_a_year_boundary_oldest_first_including_empty_months() {
        let conn = init_in_memory_for_test();
        seed_account(&conn, 1, "personal");
        insert_txn(&conn, 1, 1, "2023-12-05", 500_00, "income");
        insert_txn(&conn, 2, 1, "2023-12-10", -200_00, "expense");
        // January has no transactions at all - should still appear as a zeroed-out point.
        insert_txn(&conn, 3, 1, "2024-02-05", 100_00, "income");

        let trend = get_monthly_trend_inner(&conn, 2024, 2, 3, None).unwrap();

        assert_eq!(trend.len(), 3);
        assert_eq!((trend[0].year, trend[0].month), (2023, 12), "oldest month first");
        assert_eq!(trend[0].total_income_cents, 500_00);
        assert_eq!(trend[0].total_expense_cents, 200_00);
        assert_eq!(trend[0].net_cents, 300_00);

        assert_eq!((trend[1].year, trend[1].month), (2024, 1));
        assert_eq!(trend[1].total_income_cents, 0);
        assert_eq!(trend[1].total_expense_cents, 0);
        assert_eq!(trend[1].net_cents, 0);

        assert_eq!((trend[2].year, trend[2].month), (2024, 2), "requested month is last");
        assert_eq!(trend[2].total_income_cents, 100_00);
    }
}
