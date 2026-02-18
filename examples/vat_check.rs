use faktura::vat::*;
use rust_decimal_macros::dec;

fn main() {
    // VAT ID format validation (no network required)
    println!("=== VAT ID Format Validation ===\n");

    let test_ids = [
        "DE123456789",
        "ATU12345678",
        "FR12345678901",
        "NL123456789B01",
        "DE12345678",  // too short
        "XX999999999", // unknown country
    ];

    for id in &test_ids {
        match validate_vat_format(id) {
            Ok((cc, num)) => println!("  {id} => valid (country={cc}, number={num})"),
            Err(e) => println!("  {id} => INVALID: {e}"),
        }
    }

    // German Steuernummer validation
    println!("\n=== Steuernummer Validation ===\n");

    let test_stnr = [
        "1121081508155", // Berlin, 13-digit ELSTER
        "11/210/81508",  // Berlin, with slashes
        "5133081508159", // NRW
        "123456",        // too short
    ];

    for stnr in &test_stnr {
        match validate_steuernummer(stnr) {
            Ok(cleaned) => println!("  {stnr} => valid (cleaned: {cleaned})"),
            Err(e) => println!("  {stnr} => INVALID: {e}"),
        }
    }

    // Kleinunternehmer threshold check
    println!("\n=== Kleinunternehmer §19 UStG Check ===\n");

    let scenarios = [
        ("Below both limits", dec!(20_000), dec!(80_000)),
        ("At exact limits", dec!(25_000), dec!(100_000)),
        ("Previous year over", dec!(26_000), dec!(50_000)),
        ("Current year over", dec!(20_000), dec!(110_000)),
        ("First year, no history", dec!(0), dec!(45_000)),
    ];

    for (label, prev, curr) in &scenarios {
        let status = check_kleinunternehmer(*prev, *curr);
        println!("  {label}:");
        println!("    prev={prev}, curr={curr}");
        println!(
            "    eligible={}, reason={}",
            status.eligible,
            status.reason.as_deref().unwrap_or("—")
        );
    }

    println!("\n  Thresholds: prev year ≤ {KU_PREV_YEAR_LIMIT}, curr year ≤ {KU_CURR_YEAR_LIMIT}");
}
