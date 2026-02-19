//! ISO 4217 currency code validation.
//!
//! Provides a lookup of commonly used ISO 4217 currency codes for invoice
//! validation. Covers all major world currencies relevant to European
//! e-invoicing (EN 16931).

/// Check whether `code` is a known ISO 4217 currency code.
pub fn is_known_currency_code(code: &str) -> bool {
    CURRENCY_CODES.binary_search(&code).is_ok()
}

/// Sorted list of common ISO 4217 currency codes.
/// Sorted for binary search.
static CURRENCY_CODES: &[&str] = &[
    "AED", // UAE Dirham
    "AMD", // Armenian Dram
    "AUD", // Australian Dollar
    "BGN", // Bulgarian Lev
    "BRL", // Brazilian Real
    "CAD", // Canadian Dollar
    "CHF", // Swiss Franc
    "CNY", // Chinese Yuan
    "CZK", // Czech Koruna
    "DKK", // Danish Krone
    "EGP", // Egyptian Pound
    "EUR", // Euro
    "GBP", // Pound Sterling
    "GEL", // Georgian Lari
    "HKD", // Hong Kong Dollar
    "HRK", // Croatian Kuna
    "HUF", // Hungarian Forint
    "IDR", // Indonesian Rupiah
    "ILS", // Israeli Shekel
    "INR", // Indian Rupee
    "ISK", // Icelandic Krona
    "JPY", // Japanese Yen
    "KES", // Kenyan Shilling
    "KRW", // South Korean Won
    "KZT", // Kazakhstani Tenge
    "MXN", // Mexican Peso
    "MYR", // Malaysian Ringgit
    "NGN", // Nigerian Naira
    "NOK", // Norwegian Krone
    "NZD", // New Zealand Dollar
    "PHP", // Philippine Peso
    "PLN", // Polish Zloty
    "RON", // Romanian Leu
    "RUB", // Russian Ruble
    "SAR", // Saudi Riyal
    "SEK", // Swedish Krona
    "SGD", // Singapore Dollar
    "THB", // Thai Baht
    "TRY", // Turkish Lira
    "TWD", // New Taiwan Dollar
    "UAH", // Ukrainian Hryvnia
    "USD", // US Dollar
    "VND", // Vietnamese Dong
    "ZAR", // South African Rand
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_currencies() {
        assert!(is_known_currency_code("EUR"));
        assert!(is_known_currency_code("USD"));
        assert!(is_known_currency_code("GBP"));
        assert!(is_known_currency_code("CHF"));
        assert!(is_known_currency_code("JPY"));
        assert!(is_known_currency_code("SEK"));
    }

    #[test]
    fn unknown_currencies() {
        assert!(!is_known_currency_code("XYZ"));
        assert!(!is_known_currency_code(""));
        assert!(!is_known_currency_code("EURO"));
        assert!(!is_known_currency_code("eu"));
    }

    #[test]
    fn list_is_sorted() {
        for window in CURRENCY_CODES.windows(2) {
            assert!(
                window[0] < window[1],
                "currency codes not sorted: {} >= {}",
                window[0],
                window[1]
            );
        }
    }
}
