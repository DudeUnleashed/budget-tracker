use crate::recurrence::next_date;
use chrono::NaiveDate;
use serde::Serialize;

/// A safety cap, not a realistic outcome - guards against a payment that barely covers interest
/// (rounding could otherwise let the balance creep toward zero indefinitely) or a bad input
/// combination looping forever.
const MAX_MONTHS: i64 = 1200;

#[derive(Debug, Clone, Serialize)]
pub struct DebtPayoffResult {
    pub payoff_date: Option<String>,
    pub months: i64,
    pub total_paid_cents: i64,
    pub total_interest_cents: i64,
    /// True if the payment never exceeds the interest accruing each month (or the cap above was
    /// hit first) - the balance would never reach zero at this payment/rate.
    pub never_pays_off: bool,
}

/// Standard monthly amortization: each month, interest accrues on the remaining balance at
/// `interest_rate_bps / 12`, the payment covers that interest first, and whatever's left reduces
/// principal - rounding interest to the cent each month rather than carrying a fractional
/// remainder, the same way a real loan statement would.
pub fn calculate_payoff(
    start_date: NaiveDate,
    principal_cents: i64,
    monthly_payment_cents: i64,
    interest_rate_bps: i64,
) -> DebtPayoffResult {
    if principal_cents <= 0 {
        return DebtPayoffResult {
            payoff_date: Some(start_date.format("%Y-%m-%d").to_string()),
            months: 0,
            total_paid_cents: 0,
            total_interest_cents: 0,
            never_pays_off: false,
        };
    }

    let monthly_rate = (interest_rate_bps as f64 / 10_000.0) / 12.0;
    let mut balance_cents = principal_cents;
    let mut total_paid_cents: i64 = 0;
    let mut total_interest_cents: i64 = 0;

    for months in 1..=MAX_MONTHS {
        let interest_cents = (balance_cents as f64 * monthly_rate).round() as i64;
        if monthly_payment_cents <= interest_cents {
            return DebtPayoffResult {
                payoff_date: None,
                months: months - 1,
                total_paid_cents,
                total_interest_cents,
                never_pays_off: true,
            };
        }
        let owed_cents = balance_cents + interest_cents;
        if owed_cents <= monthly_payment_cents {
            total_paid_cents += owed_cents;
            total_interest_cents += interest_cents;
            return DebtPayoffResult {
                payoff_date: Some(next_date(start_date, "month", months).format("%Y-%m-%d").to_string()),
                months,
                total_paid_cents,
                total_interest_cents,
                never_pays_off: false,
            };
        }
        balance_cents = owed_cents - monthly_payment_cents;
        total_paid_cents += monthly_payment_cents;
        total_interest_cents += interest_cents;
    }

    DebtPayoffResult {
        payoff_date: None,
        months: MAX_MONTHS,
        total_paid_cents,
        total_interest_cents,
        never_pays_off: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_interest_pays_off_in_exact_installments() {
        let start = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let result = calculate_payoff(start, 120_000, 10_000, 0);
        assert!(!result.never_pays_off);
        assert_eq!(result.months, 12);
        assert_eq!(result.total_paid_cents, 120_000);
        assert_eq!(result.total_interest_cents, 0);
        assert_eq!(result.payoff_date.as_deref(), Some("2027-01-01"));
    }

    #[test]
    fn interest_bearing_debt_matches_hand_computed_schedule() {
        // $100 at 12%/yr (1% monthly): month 1 accrues $1.00 interest, $60 payment leaves $41.00
        // owed; month 2 accrues $0.41, and $60 more than covers the remaining $41.41.
        let start = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let result = calculate_payoff(start, 10_000, 6_000, 1_200);
        assert!(!result.never_pays_off);
        assert_eq!(result.months, 2);
        assert_eq!(result.total_interest_cents, 141);
        assert_eq!(result.total_paid_cents, 10_141);
        assert_eq!(result.payoff_date.as_deref(), Some("2026-03-01"));
    }

    #[test]
    fn payment_below_interest_never_pays_off() {
        let start = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        // $1000 at 24%/yr (2% monthly) accrues $20/month in interest - a $1 payment never dents it.
        let result = calculate_payoff(start, 100_000, 100, 2_400);
        assert!(result.never_pays_off);
        assert_eq!(result.payoff_date, None);
        assert_eq!(result.months, 0);
    }

    #[test]
    fn zero_principal_pays_off_immediately() {
        let start = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
        let result = calculate_payoff(start, 0, 5_000, 1_000);
        assert!(!result.never_pays_off);
        assert_eq!(result.months, 0);
        assert_eq!(result.total_paid_cents, 0);
        assert_eq!(result.payoff_date.as_deref(), Some("2026-01-01"));
    }
}
