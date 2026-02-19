//! UNTDID 5189 / 7161 reason code validation.
//!
//! UNTDID 5189 defines allowance reason codes, UNTDID 7161 defines
//! charge reason codes. Used for document-level and line-level
//! allowances/charges in EN 16931 invoicing.

/// Check whether `code` is a known UNTDID 5189 allowance reason code.
pub fn is_known_allowance_reason(code: &str) -> bool {
    ALLOWANCE_REASON_CODES.binary_search(&code).is_ok()
}

/// Check whether `code` is a known UNTDID 7161 charge reason code.
pub fn is_known_charge_reason(code: &str) -> bool {
    CHARGE_REASON_CODES.binary_search(&code).is_ok()
}

/// UNTDID 5189 — Allowance reason codes (sorted for binary search).
static ALLOWANCE_REASON_CODES: &[&str] = &[
    "100", // Special agreement
    "102", // Fixed long term
    "103", // Temporary
    "104", // Standard
    "105", // Yearly turnover
    "41",  // Bonus for works ahead of schedule
    "42",  // Other bonus
    "60",  // Manufacturer's consumer discount
    "62",  // Due to military status
    "63",  // Due to work accident
    "64",  // Special agreement
    "65",  // Production error discount
    "66",  // New outlet discount
    "67",  // Sample discount
    "68",  // End-of-range discount
    "70",  // Incoterm discount
    "71",  // Point of sales threshold allowance
    "88",  // Material surcharge/deduction
    "95",  // Discount
];

/// UNTDID 7161 — Charge reason codes (sorted for binary search).
static CHARGE_REASON_CODES: &[&str] = &[
    "AA",  // Advertising
    "AAA", // Telecommunication
    "AAC", // Technical modification
    "AAD", // Job-order production
    "AAE", // Outlays
    "AAF", // Off-premises
    "ABK", // Miscellaneous
    "ABL", // Additional packaging
    "ADR", // Other services
    "ADT", // Pick-up
    "AEW", // Environmental protection service
    "FC",  // Freight service
    "FI",  // Financing
    "FL",  // Flat rate
    "LA",  // Labelling
    "PC",  // Packing
    "TS",  // Testing
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_allowance_codes() {
        assert!(is_known_allowance_reason("95"));
        assert!(is_known_allowance_reason("41"));
        assert!(is_known_allowance_reason("100"));
        assert!(is_known_allowance_reason("42"));
    }

    #[test]
    fn unknown_allowance_codes() {
        assert!(!is_known_allowance_reason("99"));
        assert!(!is_known_allowance_reason(""));
        assert!(!is_known_allowance_reason("DISCOUNT"));
    }

    #[test]
    fn known_charge_codes() {
        assert!(is_known_charge_reason("FC"));
        assert!(is_known_charge_reason("PC"));
        assert!(is_known_charge_reason("AA"));
        assert!(is_known_charge_reason("ABK"));
    }

    #[test]
    fn unknown_charge_codes() {
        assert!(!is_known_charge_reason("ZZ"));
        assert!(!is_known_charge_reason(""));
        assert!(!is_known_charge_reason("FREIGHT"));
    }

    #[test]
    fn allowance_list_is_sorted() {
        for window in ALLOWANCE_REASON_CODES.windows(2) {
            assert!(
                window[0] < window[1],
                "allowance codes not sorted: {} >= {}",
                window[0],
                window[1]
            );
        }
    }

    #[test]
    fn charge_list_is_sorted() {
        for window in CHARGE_REASON_CODES.windows(2) {
            assert!(
                window[0] < window[1],
                "charge codes not sorted: {} >= {}",
                window[0],
                window[1]
            );
        }
    }
}
