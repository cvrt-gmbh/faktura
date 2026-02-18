//! §19 UStG Kleinunternehmerregelung threshold checks.
//!
//! As of 2025 (Jahressteuergesetz 2024):
//! - Previous year revenue (net): ≤ 25,000 EUR
//! - Current year forecast (net): ≤ 100,000 EUR
//! - Exceeding 100k mid-year loses status immediately

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

/// Previous year net revenue threshold (§19 UStG, from 2025).
pub const KU_PREV_YEAR_LIMIT: Decimal = dec!(25_000);

/// Current year net revenue threshold (§19 UStG, from 2025).
pub const KU_CURR_YEAR_LIMIT: Decimal = dec!(100_000);

/// Result of a Kleinunternehmer eligibility check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KleinunternehmerStatus {
    /// Whether the business is eligible for Kleinunternehmer status.
    pub eligible: bool,
    /// Previous year net revenue used in the check.
    pub prev_year_revenue: Decimal,
    /// Current year net revenue (or forecast) used in the check.
    pub curr_year_revenue: Decimal,
    /// If not eligible, the reason why.
    pub reason: Option<String>,
}

/// Check Kleinunternehmer eligibility under §19 UStG (2025+ rules).
///
/// Both amounts must be **net** revenue (without VAT).
///
/// # Arguments
/// - `prev_year_revenue` — Actual net revenue from the previous calendar year
/// - `curr_year_revenue` — Current year net revenue (actual or forecast)
pub fn check_kleinunternehmer(
    prev_year_revenue: Decimal,
    curr_year_revenue: Decimal,
) -> KleinunternehmerStatus {
    if prev_year_revenue > KU_PREV_YEAR_LIMIT {
        return KleinunternehmerStatus {
            eligible: false,
            prev_year_revenue,
            curr_year_revenue,
            reason: Some(format!(
                "previous year net revenue {prev_year_revenue} exceeds limit of {KU_PREV_YEAR_LIMIT}"
            )),
        };
    }

    if curr_year_revenue > KU_CURR_YEAR_LIMIT {
        return KleinunternehmerStatus {
            eligible: false,
            prev_year_revenue,
            curr_year_revenue,
            reason: Some(format!(
                "current year net revenue {curr_year_revenue} exceeds limit of {KU_CURR_YEAR_LIMIT}"
            )),
        };
    }

    KleinunternehmerStatus {
        eligible: true,
        prev_year_revenue,
        curr_year_revenue,
        reason: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eligible_below_both_limits() {
        let s = check_kleinunternehmer(dec!(20_000), dec!(80_000));
        assert!(s.eligible);
        assert!(s.reason.is_none());
    }

    #[test]
    fn eligible_at_exact_limits() {
        let s = check_kleinunternehmer(dec!(25_000), dec!(100_000));
        assert!(s.eligible);
    }

    #[test]
    fn ineligible_prev_year_over() {
        let s = check_kleinunternehmer(dec!(25_001), dec!(50_000));
        assert!(!s.eligible);
        assert!(s.reason.as_ref().unwrap().contains("previous year"));
    }

    #[test]
    fn ineligible_curr_year_over() {
        let s = check_kleinunternehmer(dec!(20_000), dec!(100_001));
        assert!(!s.eligible);
        assert!(s.reason.as_ref().unwrap().contains("current year"));
    }

    #[test]
    fn zero_revenue_eligible() {
        let s = check_kleinunternehmer(dec!(0), dec!(0));
        assert!(s.eligible);
    }

    #[test]
    fn first_year_no_previous() {
        // First year of business: no previous year revenue
        let s = check_kleinunternehmer(dec!(0), dec!(90_000));
        assert!(s.eligible);
    }
}
