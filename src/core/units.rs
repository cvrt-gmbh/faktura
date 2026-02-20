//! UN/CEFACT Recommendation 20 unit codes.
//!
//! Provides a lookup of commonly used unit codes for invoice line items.
//! The full Rec 20 list has ~2000 codes; this covers the subset most
//! relevant to European e-invoicing (EN 16931).

/// Check whether `code` is a known UN/CEFACT Rec 20 unit code.
pub fn is_known_unit_code(code: &str) -> bool {
    COMMON_UNIT_CODES.binary_search(&code).is_ok()
}

/// Sorted list of common UN/CEFACT Rec 20 unit codes used in EN 16931 invoicing.
/// Sorted for binary search.
static COMMON_UNIT_CODES: &[&str] = &[
    "2N",  // Decibel
    "4K",  // Kilovolt-ampere (reactive)
    "ANN", // Year
    "BAR", // Bar (pressure)
    "BLL", // Barrel (US)
    "BX",  // Box
    "C62", // One (piece/unit)
    "CCM", // Cubic centimetre
    "CLT", // Centilitre
    "CMK", // Square centimetre
    "CMT", // Centimetre
    "CS",  // Case
    "CT",  // Carton
    "DAY", // Day
    "DMQ", // Cubic decimetre (litre)
    "DMT", // Decimetre
    "DZN", // Dozen
    "EA",  // Each
    "FOT", // Foot
    "GLL", // Gallon (US)
    "GM",  // Gram per square metre
    "GRM", // Gram
    "GRO", // Gross
    "GWH", // Gigawatt-hour
    "HAR", // Hectare
    "HLT", // Hectolitre
    "HUR", // Hour
    "INH", // Inch
    "JOU", // Joule
    "KGM", // Kilogram
    "KGS", // Kilogram per second
    "KHZ", // Kilohertz
    "KMH", // Kilometre per hour
    "KMT", // Kilometre
    "KTM", // Kilometre
    "KVA", // Kilovolt-ampere
    "KVT", // Kilovolt
    "KWH", // Kilowatt-hour
    "KWT", // Kilowatt
    "LBR", // Pound
    "LE",  // Lite
    "LM",  // Linear metre
    "LPA", // Litre of pure alcohol
    "LS",  // Lump sum
    "LTR", // Litre
    "MAW", // Megawatt
    "MBR", // Millibar
    "MGM", // Milligram
    "MHZ", // Megahertz
    "MIN", // Minute
    "MLT", // Millilitre
    "MMK", // Square millimetre
    "MMT", // Millimetre
    "MON", // Month
    "MQH", // Cubic metre per hour
    "MTK", // Square metre
    "MTQ", // Cubic metre
    "MTR", // Metre
    "MTS", // Metre per second
    "MWH", // Megawatt-hour
    "NAR", // Number of articles
    "NPR", // Number of pairs
    "P1",  // Percent
    "PA",  // Packet
    "PK",  // Pack
    "PR",  // Pair
    "QTI", // Quantity (imperial)
    "RO",  // Roll
    "SA",  // Sack
    "SEC", // Second
    "SET", // Set
    "SMI", // Mile (statute)
    "ST",  // Sheet
    "STN", // Short ton (US)
    "TNE", // Tonne (metric ton)
    "WEE", // Week
    "XBD", // Bundle
    "XBG", // Bag
    "XBX", // Box
    "XCT", // Carton
    "XPA", // Packet
    "XPK", // Package
    "XPX", // Pallet
    "XRO", // Roll
    "XSA", // Sack
    "XST", // Sheet
    "YRD", // Yard
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_codes() {
        assert!(is_known_unit_code("C62"));
        assert!(is_known_unit_code("HUR"));
        assert!(is_known_unit_code("KGM"));
        assert!(is_known_unit_code("LTR"));
        assert!(is_known_unit_code("MTR"));
        assert!(is_known_unit_code("DAY"));
        assert!(is_known_unit_code("MON"));
        assert!(is_known_unit_code("SET"));
    }

    #[test]
    fn unknown_codes() {
        assert!(!is_known_unit_code("XYZ"));
        assert!(!is_known_unit_code(""));
        assert!(!is_known_unit_code("PIECE"));
    }

    #[test]
    fn list_is_sorted() {
        for window in COMMON_UNIT_CODES.windows(2) {
            assert!(
                window[0] < window[1],
                "unit codes not sorted: {} >= {}",
                window[0],
                window[1]
            );
        }
    }
}
