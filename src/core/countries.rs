//! ISO 3166-1 alpha-2 country code validation.
//!
//! Full list of currently assigned ISO 3166-1 alpha-2 country codes
//! for invoice address validation (EN 16931 BR-11/BR-12).

/// Check whether `code` is a known ISO 3166-1 alpha-2 country code.
pub fn is_known_country_code(code: &str) -> bool {
    COUNTRY_CODES.binary_search(&code).is_ok()
}

/// Complete list of ISO 3166-1 alpha-2 country codes (249 entries).
/// Sorted for binary search.
static COUNTRY_CODES: &[&str] = &[
    "AD", "AE", "AF", "AG", "AI", "AL", "AM", "AO", "AQ", "AR", "AS", "AT", "AU", "AW", "AX", "AZ",
    "BA", "BB", "BD", "BE", "BF", "BG", "BH", "BI", "BJ", "BL", "BM", "BN", "BO", "BQ", "BR", "BS",
    "BT", "BV", "BW", "BY", "BZ", "CA", "CC", "CD", "CF", "CG", "CH", "CI", "CK", "CL", "CM", "CN",
    "CO", "CR", "CU", "CV", "CW", "CX", "CY", "CZ", "DE", "DJ", "DK", "DM", "DO", "DZ", "EC", "EE",
    "EG", "EH", "ER", "ES", "ET", "FI", "FJ", "FK", "FM", "FO", "FR", "GA", "GB", "GD", "GE", "GF",
    "GG", "GH", "GI", "GL", "GM", "GN", "GP", "GQ", "GR", "GS", "GT", "GU", "GW", "GY", "HK", "HM",
    "HN", "HR", "HT", "HU", "ID", "IE", "IL", "IM", "IN", "IO", "IQ", "IR", "IS", "IT", "JE", "JM",
    "JO", "JP", "KE", "KG", "KH", "KI", "KM", "KN", "KP", "KR", "KW", "KY", "KZ", "LA", "LB", "LC",
    "LI", "LK", "LR", "LS", "LT", "LU", "LV", "LY", "MA", "MC", "MD", "ME", "MF", "MG", "MH", "MK",
    "ML", "MM", "MN", "MO", "MP", "MQ", "MR", "MS", "MT", "MU", "MV", "MW", "MX", "MY", "MZ", "NA",
    "NC", "NE", "NF", "NG", "NI", "NL", "NO", "NP", "NR", "NU", "NZ", "OM", "PA", "PE", "PF", "PG",
    "PH", "PK", "PL", "PM", "PN", "PR", "PS", "PT", "PW", "PY", "QA", "RE", "RO", "RS", "RU", "RW",
    "SA", "SB", "SC", "SD", "SE", "SG", "SH", "SI", "SJ", "SK", "SL", "SM", "SN", "SO", "SR", "SS",
    "ST", "SV", "SX", "SY", "SZ", "TC", "TD", "TF", "TG", "TH", "TJ", "TK", "TL", "TM", "TN", "TO",
    "TR", "TT", "TV", "TW", "TZ", "UA", "UG", "UM", "US", "UY", "UZ", "VA", "VC", "VE", "VG", "VI",
    "VN", "VU", "WF", "WS", "YE", "YT", "ZA", "ZM", "ZW",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_countries() {
        assert!(is_known_country_code("DE"));
        assert!(is_known_country_code("AT"));
        assert!(is_known_country_code("CH"));
        assert!(is_known_country_code("FR"));
        assert!(is_known_country_code("US"));
        assert!(is_known_country_code("GB"));
        assert!(is_known_country_code("JP"));
    }

    #[test]
    fn unknown_countries() {
        assert!(!is_known_country_code("XX"));
        assert!(!is_known_country_code(""));
        assert!(!is_known_country_code("DEU"));
        assert!(!is_known_country_code("de"));
    }

    #[test]
    fn list_is_sorted() {
        for window in COUNTRY_CODES.windows(2) {
            assert!(
                window[0] < window[1],
                "country codes not sorted: {} >= {}",
                window[0],
                window[1]
            );
        }
    }

    #[test]
    fn list_count() {
        assert_eq!(COUNTRY_CODES.len(), 249);
    }
}
