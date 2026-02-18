//! Electronic Address Scheme (EAS) codes for Peppol EndpointID.

use serde::{Deserialize, Serialize};

/// Common EAS (Electronic Address Scheme) codes for Peppol participant identification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EasScheme {
    /// The numeric scheme code (e.g. "0088", "9930").
    pub code: &'static str,
    /// Human-readable description.
    pub description: &'static str,
}

impl EasScheme {
    /// GS1 GLN (Global Location Number) — international.
    pub const GLN: Self = Self {
        code: "0088",
        description: "GS1 GLN",
    };
    /// German Leitweg-ID (public sector routing).
    pub const LEITWEG_ID: Self = Self {
        code: "0204",
        description: "Leitweg-ID",
    };
    /// Belgian enterprise number.
    pub const BE_EN: Self = Self {
        code: "0208",
        description: "Belgian enterprise number",
    };
    /// DIGSTORG (Denmark).
    pub const DK_DIGST: Self = Self {
        code: "0184",
        description: "DIGSTORG",
    };
    /// Dutch OIN.
    pub const NL_OIN: Self = Self {
        code: "0190",
        description: "Dutch OIN",
    };
    /// Dutch KvK.
    pub const NL_KVK: Self = Self {
        code: "0106",
        description: "Dutch KvK",
    };
    /// Italian Codice Fiscale.
    pub const IT_CF: Self = Self {
        code: "0210",
        description: "Italian Codice Fiscale",
    };
    /// Italian Partita IVA.
    pub const IT_IVA: Self = Self {
        code: "0211",
        description: "Italian Partita IVA",
    };
    /// German VAT number (DE + 9 digits).
    pub const DE_VAT: Self = Self {
        code: "9930",
        description: "German VAT number",
    };
    /// Austrian VAT number.
    pub const AT_VAT: Self = Self {
        code: "9914",
        description: "Austrian VAT number",
    };
    /// Belgian VAT number.
    pub const BE_VAT: Self = Self {
        code: "9925",
        description: "Belgian VAT number",
    };
    /// French VAT number.
    pub const FR_VAT: Self = Self {
        code: "9957",
        description: "French VAT number",
    };
    /// Italian VAT number.
    pub const IT_VAT: Self = Self {
        code: "9906",
        description: "Italian VAT number",
    };
    /// Dutch VAT number.
    pub const NL_VAT: Self = Self {
        code: "9944",
        description: "Dutch VAT number",
    };
    /// Finnish OVT.
    pub const FI_OVT: Self = Self {
        code: "0037",
        description: "Finnish OVT",
    };
    /// Swedish Org number.
    pub const SE_ORG: Self = Self {
        code: "0007",
        description: "Swedish Org number",
    };
    /// Norwegian Org number.
    pub const NO_ORG: Self = Self {
        code: "0192",
        description: "Norwegian Org number",
    };
}

/// Return the default EAS scheme for a given country code.
///
/// This provides a reasonable default for the most common identifier
/// type used in each country. For Germany, this returns the Leitweg-ID
/// scheme for public sector; use `EasScheme::DE_VAT` for B2B.
pub fn eas_scheme_for_country(country_code: &str) -> Option<EasScheme> {
    match country_code.to_uppercase().as_str() {
        "DE" => Some(EasScheme::LEITWEG_ID),
        "AT" => Some(EasScheme::AT_VAT),
        "BE" => Some(EasScheme::BE_EN),
        "DK" => Some(EasScheme::DK_DIGST),
        "FI" => Some(EasScheme::FI_OVT),
        "FR" => Some(EasScheme::FR_VAT),
        "IT" => Some(EasScheme::IT_CF),
        "NL" => Some(EasScheme::NL_OIN),
        "NO" => Some(EasScheme::NO_ORG),
        "SE" => Some(EasScheme::SE_ORG),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn de_default_is_leitweg() {
        let s = eas_scheme_for_country("DE").unwrap();
        assert_eq!(s.code, "0204");
    }

    #[test]
    fn at_default_is_vat() {
        let s = eas_scheme_for_country("AT").unwrap();
        assert_eq!(s.code, "9914");
    }

    #[test]
    fn unknown_country_returns_none() {
        assert!(eas_scheme_for_country("XX").is_none());
    }

    #[test]
    fn case_insensitive() {
        assert!(eas_scheme_for_country("de").is_some());
    }

    #[test]
    fn scheme_constants() {
        assert_eq!(EasScheme::GLN.code, "0088");
        assert_eq!(EasScheme::DE_VAT.code, "9930");
        assert_eq!(EasScheme::LEITWEG_ID.code, "0204");
    }
}
