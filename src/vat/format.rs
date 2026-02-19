//! VAT ID and Steuernummer format validation.

use std::fmt;

/// Error returned when a VAT ID or Steuernummer fails format validation.
#[derive(Debug, Clone)]
pub struct VatFormatError {
    /// The invalid input value.
    pub value: String,
    /// Why the value failed validation.
    pub reason: String,
}

impl fmt::Display for VatFormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid VAT ID '{}': {}", self.value, self.reason)
    }
}

impl std::error::Error for VatFormatError {}

/// Validate a EU VAT ID by format (no network call).
///
/// The input must include the 2-letter country prefix (e.g. "DE123456789").
/// Returns the (country_code, number) split on success.
pub fn validate_vat_format(vat_id: &str) -> Result<(&str, &str), VatFormatError> {
    let vat_id = vat_id.trim();
    if vat_id.len() < 4 {
        return Err(VatFormatError {
            value: vat_id.into(),
            reason: "too short — must be at least 4 characters".into(),
        });
    }

    let country = &vat_id[..2];
    let number = &vat_id[2..];

    type VatValidator = fn(&str) -> bool;
    let pattern: &[(&str, VatValidator)] = &[
        ("AT", |n| {
            n.len() == 9 && n.starts_with('U') && n[1..].chars().all(|c| c.is_ascii_digit())
        }),
        ("BE", |n| {
            n.len() == 10 && n.chars().all(|c| c.is_ascii_digit())
        }),
        ("BG", |n| {
            (n.len() == 9 || n.len() == 10) && n.chars().all(|c| c.is_ascii_digit())
        }),
        ("CY", |n| {
            n.len() == 9
                && n[..8].chars().all(|c| c.is_ascii_digit())
                && n.as_bytes()[8].is_ascii_alphabetic()
        }),
        ("CZ", |n| {
            (8..=10).contains(&n.len()) && n.chars().all(|c| c.is_ascii_digit())
        }),
        ("DE", |n| {
            n.len() == 9 && n.chars().all(|c| c.is_ascii_digit()) && n.as_bytes()[0] != b'0'
        }),
        ("DK", |n| {
            n.len() == 8 && n.chars().all(|c| c.is_ascii_digit())
        }),
        ("EE", |n| {
            n.len() == 9 && n.chars().all(|c| c.is_ascii_digit())
        }),
        ("EL", |n| {
            n.len() == 9 && n.chars().all(|c| c.is_ascii_digit())
        }),
        ("ES", |n| {
            n.len() == 9 && n.chars().all(|c| c.is_ascii_alphanumeric())
        }),
        ("FI", |n| {
            n.len() == 8 && n.chars().all(|c| c.is_ascii_digit())
        }),
        ("FR", |n| {
            n.len() == 11
                && n[..2].chars().all(|c| c.is_ascii_alphanumeric())
                && n[2..].chars().all(|c| c.is_ascii_digit())
        }),
        ("HR", |n| {
            n.len() == 11 && n.chars().all(|c| c.is_ascii_digit())
        }),
        ("HU", |n| {
            n.len() == 8 && n.chars().all(|c| c.is_ascii_digit())
        }),
        ("IE", |n| {
            (n.len() == 8 || n.len() == 9) && n.chars().all(|c| c.is_ascii_alphanumeric())
        }),
        ("IT", |n| {
            n.len() == 11 && n.chars().all(|c| c.is_ascii_digit())
        }),
        ("LT", |n| {
            (n.len() == 9 || n.len() == 12) && n.chars().all(|c| c.is_ascii_digit())
        }),
        ("LU", |n| {
            n.len() == 8 && n.chars().all(|c| c.is_ascii_digit())
        }),
        ("LV", |n| {
            n.len() == 11 && n.chars().all(|c| c.is_ascii_digit())
        }),
        ("MT", |n| {
            n.len() == 8 && n.chars().all(|c| c.is_ascii_digit())
        }),
        ("NL", |n| {
            n.len() == 12
                && n[..9].chars().all(|c| c.is_ascii_digit())
                && n.as_bytes()[9] == b'B'
                && n[10..].chars().all(|c| c.is_ascii_digit())
        }),
        ("PL", |n| {
            n.len() == 10 && n.chars().all(|c| c.is_ascii_digit())
        }),
        ("PT", |n| {
            n.len() == 9 && n.chars().all(|c| c.is_ascii_digit())
        }),
        ("RO", |n| {
            (2..=10).contains(&n.len()) && n.chars().all(|c| c.is_ascii_digit())
        }),
        ("SE", |n| {
            n.len() == 12 && n.chars().all(|c| c.is_ascii_digit())
        }),
        ("SI", |n| {
            n.len() == 8 && n.chars().all(|c| c.is_ascii_digit())
        }),
        ("SK", |n| {
            n.len() == 10 && n.chars().all(|c| c.is_ascii_digit())
        }),
    ];

    let country_upper = country.to_uppercase();
    for &(code, validator) in pattern {
        if country_upper == code {
            if validator(number) {
                return Ok((country, number));
            } else {
                return Err(VatFormatError {
                    value: vat_id.into(),
                    reason: format!("invalid format for country {code}"),
                });
            }
        }
    }

    // XI (Northern Ireland) uses GB format
    if country_upper == "XI" {
        if number.len() == 9 && number.chars().all(|c| c.is_ascii_digit()) {
            return Ok((country, number));
        }
        return Err(VatFormatError {
            value: vat_id.into(),
            reason: "invalid format for country XI".into(),
        });
    }

    Err(VatFormatError {
        value: vat_id.into(),
        reason: format!("unknown country code '{country}'"),
    })
}

/// Validate a German Steuernummer (tax number) format.
///
/// Accepts both the unified 13-digit ELSTER format and common
/// display formats with slashes (e.g. "12/345/67890").
/// Returns the cleaned 13-digit number on success.
pub fn validate_steuernummer(stnr: &str) -> Result<String, VatFormatError> {
    // Strip common separators
    let cleaned: String = stnr.chars().filter(|c| c.is_ascii_digit()).collect();

    // The unified format is 13 digits
    if cleaned.len() == 13 {
        // Validate known Bundesland prefixes
        let prefix2: u32 = cleaned[..2].parse().unwrap_or(0);
        let valid_prefixes = [10, 11, 21, 22, 23, 24, 26, 27, 28, 30, 31, 32, 40, 41];
        // NRW uses prefix 5x
        if valid_prefixes.contains(&prefix2) || (50..=60).contains(&prefix2) {
            return Ok(cleaned);
        }
        return Err(VatFormatError {
            value: stnr.into(),
            reason: format!("unknown Bundesland prefix '{}'", &cleaned[..2]),
        });
    }

    // Legacy formats: 10-11 digits (without separators)
    if (10..=11).contains(&cleaned.len()) {
        return Ok(cleaned);
    }

    Err(VatFormatError {
        value: stnr.into(),
        reason: format!(
            "expected 13 digits (ELSTER) or 10-11 digits (legacy), got {}",
            cleaned.len()
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- VAT format ---

    #[test]
    fn valid_de_vat() {
        let (cc, num) = validate_vat_format("DE123456789").unwrap();
        assert_eq!(cc, "DE");
        assert_eq!(num, "123456789");
    }

    #[test]
    fn valid_at_vat() {
        assert!(validate_vat_format("ATU12345678").is_ok());
    }

    #[test]
    fn valid_fr_vat() {
        assert!(validate_vat_format("FR12345678901").is_ok());
    }

    #[test]
    fn valid_nl_vat() {
        assert!(validate_vat_format("NL123456789B01").is_ok());
    }

    #[test]
    fn valid_it_vat() {
        assert!(validate_vat_format("IT12345678901").is_ok());
    }

    #[test]
    fn valid_es_vat() {
        assert!(validate_vat_format("ESX1234567X").is_ok());
    }

    #[test]
    fn valid_pl_vat() {
        assert!(validate_vat_format("PL1234567890").is_ok());
    }

    #[test]
    fn de_vat_leading_zero_rejected() {
        assert!(validate_vat_format("DE012345678").is_err());
    }

    #[test]
    fn de_vat_too_short() {
        assert!(validate_vat_format("DE12345678").is_err());
    }

    #[test]
    fn de_vat_too_long() {
        assert!(validate_vat_format("DE1234567890").is_err());
    }

    #[test]
    fn unknown_country() {
        assert!(validate_vat_format("XX12345678").is_err());
    }

    #[test]
    fn too_short_input() {
        assert!(validate_vat_format("DE").is_err());
    }

    #[test]
    fn whitespace_trimmed() {
        assert!(validate_vat_format("  DE123456789  ").is_ok());
    }

    // --- Steuernummer ---

    #[test]
    fn valid_13digit_berlin() {
        let r = validate_steuernummer("1121081508155").unwrap();
        assert_eq!(r.len(), 13);
    }

    #[test]
    fn valid_13digit_nrw() {
        let r = validate_steuernummer("5133081508159").unwrap();
        assert_eq!(r.len(), 13);
    }

    #[test]
    fn valid_with_slashes() {
        let r = validate_steuernummer("11/210/81508").unwrap();
        // After stripping: 1121081508 = 10 digits → legacy format
        assert_eq!(r, "1121081508");
    }

    #[test]
    fn invalid_prefix() {
        assert!(validate_steuernummer("9900000000000").is_err());
    }

    #[test]
    fn too_few_digits() {
        assert!(validate_steuernummer("123456").is_err());
    }
}
