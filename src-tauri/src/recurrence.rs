use chrono::{Datelike, NaiveDate};

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

/// Finds the cycle `[start, end)` that contains `on_date`, walking forward from `anchor` in
/// steps of `interval_count` `interval_unit`s. If `on_date` is before `anchor`, returns the
/// first (upcoming) cycle rather than stepping backward, since a recurring item has no cycles
/// before its own anchor date.
pub fn cycle_containing(anchor: NaiveDate, interval_unit: &str, interval_count: i64, on_date: NaiveDate) -> (NaiveDate, NaiveDate) {
    if on_date < anchor {
        return (anchor, next_date(anchor, interval_unit, interval_count));
    }
    let mut start = anchor;
    let mut end = next_date(start, interval_unit, interval_count);
    while end <= on_date {
        start = end;
        end = next_date(start, interval_unit, interval_count);
    }
    (start, end)
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

    #[test]
    fn cycle_containing_finds_current_cycle() {
        let anchor = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
        let today = NaiveDate::from_ymd_opt(2026, 4, 20).unwrap();
        let (start, end) = cycle_containing(anchor, "month", 1, today);
        assert_eq!(start, NaiveDate::from_ymd_opt(2026, 4, 15).unwrap());
        assert_eq!(end, NaiveDate::from_ymd_opt(2026, 5, 15).unwrap());
    }

    #[test]
    fn cycle_containing_before_anchor_returns_first_cycle() {
        let anchor = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let today = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let (start, end) = cycle_containing(anchor, "month", 3, today);
        assert_eq!(start, anchor);
        assert_eq!(end, NaiveDate::from_ymd_opt(2026, 9, 1).unwrap());
    }

    #[test]
    fn cycle_containing_on_boundary_belongs_to_new_cycle() {
        let anchor = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let boundary = NaiveDate::from_ymd_opt(2026, 2, 1).unwrap();
        let (start, end) = cycle_containing(anchor, "month", 1, boundary);
        assert_eq!(start, boundary);
        assert_eq!(end, NaiveDate::from_ymd_opt(2026, 3, 1).unwrap());
    }
}
