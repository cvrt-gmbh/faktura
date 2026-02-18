#![cfg(feature = "vat")]

use faktura::vat::*;
use rust_decimal_macros::dec;

// ---------------------------------------------------------------------------
// VAT Format Validation — Germany
// ---------------------------------------------------------------------------

#[test]
fn de_valid() {
    let (cc, num) = validate_vat_format("DE123456789").unwrap();
    assert_eq!(cc, "DE");
    assert_eq!(num, "123456789");
}

#[test]
fn de_leading_zero_rejected() {
    assert!(validate_vat_format("DE012345678").is_err());
}

#[test]
fn de_too_short() {
    assert!(validate_vat_format("DE12345678").is_err());
}

#[test]
fn de_too_long() {
    assert!(validate_vat_format("DE1234567890").is_err());
}

#[test]
fn de_with_letters_rejected() {
    assert!(validate_vat_format("DE12345678A").is_err());
}

// ---------------------------------------------------------------------------
// VAT Format Validation — Other EU Countries
// ---------------------------------------------------------------------------

#[test]
fn at_valid() {
    assert!(validate_vat_format("ATU12345678").is_ok());
}

#[test]
fn at_missing_u_prefix() {
    assert!(validate_vat_format("AT12345678").is_err());
}

#[test]
fn fr_valid() {
    let (cc, num) = validate_vat_format("FR12345678901").unwrap();
    assert_eq!(cc, "FR");
    assert_eq!(num, "12345678901");
}

#[test]
fn fr_alpha_key() {
    // France allows 2 alphanumeric chars as key
    assert!(validate_vat_format("FRAB123456789").is_ok());
}

#[test]
fn nl_valid() {
    assert!(validate_vat_format("NL123456789B01").is_ok());
}

#[test]
fn nl_missing_b() {
    assert!(validate_vat_format("NL123456789A01").is_err());
}

#[test]
fn it_valid() {
    assert!(validate_vat_format("IT12345678901").is_ok());
}

#[test]
fn es_valid() {
    assert!(validate_vat_format("ESA12345678").is_ok());
    assert!(validate_vat_format("ESX1234567X").is_ok());
}

#[test]
fn be_valid() {
    assert!(validate_vat_format("BE0123456789").is_ok());
}

#[test]
fn pl_valid() {
    assert!(validate_vat_format("PL1234567890").is_ok());
}

#[test]
fn cz_valid_8_digits() {
    assert!(validate_vat_format("CZ12345678").is_ok());
}

#[test]
fn cz_valid_10_digits() {
    assert!(validate_vat_format("CZ1234567890").is_ok());
}

#[test]
fn se_valid() {
    assert!(validate_vat_format("SE123456789012").is_ok());
}

#[test]
fn dk_valid() {
    assert!(validate_vat_format("DK12345678").is_ok());
}

#[test]
fn fi_valid() {
    assert!(validate_vat_format("FI12345678").is_ok());
}

#[test]
fn lu_valid() {
    assert!(validate_vat_format("LU12345678").is_ok());
}

#[test]
fn pt_valid() {
    assert!(validate_vat_format("PT123456789").is_ok());
}

#[test]
fn ie_valid() {
    assert!(validate_vat_format("IE1234567A").is_ok());
}

#[test]
fn hr_valid() {
    assert!(validate_vat_format("HR12345678901").is_ok());
}

#[test]
fn hu_valid() {
    assert!(validate_vat_format("HU12345678").is_ok());
}

#[test]
fn ro_valid() {
    assert!(validate_vat_format("RO12345678").is_ok());
    assert!(validate_vat_format("RO12").is_ok()); // Romania allows 2-10 digits
}

#[test]
fn si_valid() {
    assert!(validate_vat_format("SI12345678").is_ok());
}

#[test]
fn sk_valid() {
    assert!(validate_vat_format("SK1234567890").is_ok());
}

#[test]
fn bg_valid() {
    assert!(validate_vat_format("BG123456789").is_ok());
    assert!(validate_vat_format("BG1234567890").is_ok());
}

#[test]
fn ee_valid() {
    assert!(validate_vat_format("EE123456789").is_ok());
}

#[test]
fn lt_valid() {
    assert!(validate_vat_format("LT123456789").is_ok());
    assert!(validate_vat_format("LT123456789012").is_ok());
}

#[test]
fn lv_valid() {
    assert!(validate_vat_format("LV12345678901").is_ok());
}

#[test]
fn mt_valid() {
    assert!(validate_vat_format("MT12345678").is_ok());
}

#[test]
fn cy_valid() {
    assert!(validate_vat_format("CY12345678A").is_ok());
}

#[test]
fn el_greece_valid() {
    assert!(validate_vat_format("EL123456789").is_ok());
}

#[test]
fn xi_northern_ireland() {
    assert!(validate_vat_format("XI123456789").is_ok());
}

// ---------------------------------------------------------------------------
// Edge Cases
// ---------------------------------------------------------------------------

#[test]
fn unknown_country_rejected() {
    assert!(validate_vat_format("XX12345678").is_err());
}

#[test]
fn empty_string_rejected() {
    assert!(validate_vat_format("").is_err());
}

#[test]
fn whitespace_trimmed() {
    assert!(validate_vat_format("  DE123456789  ").is_ok());
}

#[test]
fn error_display() {
    let err = validate_vat_format("DE12").unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("DE12"));
    assert!(msg.contains("invalid"));
}

// ---------------------------------------------------------------------------
// Steuernummer Validation
// ---------------------------------------------------------------------------

#[test]
fn steuernummer_13_digit_berlin() {
    let r = validate_steuernummer("1121081508155").unwrap();
    assert_eq!(r, "1121081508155");
}

#[test]
fn steuernummer_13_digit_nrw() {
    let r = validate_steuernummer("5133081508159").unwrap();
    assert_eq!(r, "5133081508159");
}

#[test]
fn steuernummer_13_digit_bavaria() {
    // Bavaria has no known 2-digit prefix in the standard set,
    // so we test a valid one: Baden-Württemberg = 28
    let r = validate_steuernummer("2812081508155").unwrap();
    assert_eq!(r, "2812081508155");
}

#[test]
fn steuernummer_with_slashes() {
    // 11/210/81508 → 1121081508 (10 digits, legacy format)
    let r = validate_steuernummer("11/210/81508").unwrap();
    assert_eq!(r, "1121081508");
}

#[test]
fn steuernummer_with_spaces() {
    let r = validate_steuernummer("11 210 81508").unwrap();
    assert_eq!(r, "1121081508");
}

#[test]
fn steuernummer_invalid_prefix() {
    assert!(validate_steuernummer("9900000000000").is_err());
}

#[test]
fn steuernummer_too_short() {
    assert!(validate_steuernummer("12345").is_err());
}

#[test]
fn steuernummer_too_long() {
    assert!(validate_steuernummer("12345678901234").is_err());
}

// ---------------------------------------------------------------------------
// Kleinunternehmer Checks
// ---------------------------------------------------------------------------

#[test]
fn ku_eligible_below_limits() {
    let s = check_kleinunternehmer(dec!(20_000), dec!(80_000));
    assert!(s.eligible);
    assert!(s.reason.is_none());
}

#[test]
fn ku_eligible_at_exact_limits() {
    let s = check_kleinunternehmer(dec!(25_000), dec!(100_000));
    assert!(s.eligible);
}

#[test]
fn ku_ineligible_prev_year_over() {
    let s = check_kleinunternehmer(dec!(25_001), dec!(50_000));
    assert!(!s.eligible);
    assert!(s.reason.as_ref().unwrap().contains("previous year"));
}

#[test]
fn ku_ineligible_curr_year_over() {
    let s = check_kleinunternehmer(dec!(20_000), dec!(100_001));
    assert!(!s.eligible);
    assert!(s.reason.as_ref().unwrap().contains("current year"));
}

#[test]
fn ku_both_over() {
    // Previous year check happens first
    let s = check_kleinunternehmer(dec!(30_000), dec!(200_000));
    assert!(!s.eligible);
    assert!(s.reason.as_ref().unwrap().contains("previous year"));
}

#[test]
fn ku_zero_revenue() {
    let s = check_kleinunternehmer(dec!(0), dec!(0));
    assert!(s.eligible);
}

#[test]
fn ku_first_year_business() {
    let s = check_kleinunternehmer(dec!(0), dec!(90_000));
    assert!(s.eligible);
}

#[test]
fn ku_threshold_constants() {
    assert_eq!(KU_PREV_YEAR_LIMIT, dec!(25_000));
    assert_eq!(KU_CURR_YEAR_LIMIT, dec!(100_000));
}

// ---------------------------------------------------------------------------
// VIES (unit tests only — no network calls)
// ---------------------------------------------------------------------------

#[test]
fn vies_result_struct() {
    let r = ViesResult {
        valid: true,
        request_date: Some("2024-06-15".into()),
        name: Some("ACME GmbH".into()),
        address: None,
    };
    assert!(r.valid);
    assert_eq!(r.name.as_deref(), Some("ACME GmbH"));
}

#[test]
fn vies_error_display() {
    let e = ViesError::Network("timeout".into());
    assert!(e.to_string().contains("timeout"));

    let e = ViesError::ApiError("MS_UNAVAILABLE".into());
    assert!(e.to_string().contains("MS_UNAVAILABLE"));

    let e = ViesError::ParseError("invalid json".into());
    assert!(e.to_string().contains("invalid json"));
}
