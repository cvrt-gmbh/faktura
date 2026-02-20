use chrono::NaiveDate;
use faktura::core::*;
use rust_decimal_macros::dec;

fn main() {
    // ── 1. Builder error: missing required fields ─────────────────────
    println!("=== Builder Error ===");
    let result = InvoiceBuilder::new("", NaiveDate::from_ymd_opt(2024, 6, 15).unwrap())
        .seller(
            PartyBuilder::new(
                "ACME GmbH",
                AddressBuilder::new("Berlin", "10115", "DE").build(),
            )
            .build(),
        )
        .buyer(
            PartyBuilder::new(
                "Kunde AG",
                AddressBuilder::new("München", "80331", "DE").build(),
            )
            .build(),
        )
        .add_line(
            LineItemBuilder::new("1", "Consulting", dec!(10), "HUR", dec!(150))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .build();

    match result {
        Ok(inv) => {
            // Invoice built — but §14 UStG validation catches the empty number
            let errors = validate_14_ustg(&inv);
            for e in &errors {
                println!("  Validation: {}", e);
            }
        }
        Err(e) => println!("  Build failed: {}", e),
    }

    // ── 2. Validation errors: incomplete invoice ──────────────────────
    println!("\n=== Validation Errors ===");
    let invoice = InvoiceBuilder::new("RE-2024-001", NaiveDate::from_ymd_opt(2024, 6, 15).unwrap())
        // Missing seller VAT ID, missing tax_point_date, missing delivery date
        .seller(
            PartyBuilder::new(
                "ACME GmbH",
                AddressBuilder::new("Berlin", "10115", "DE").build(),
            )
            .build(),
        )
        .buyer(
            PartyBuilder::new(
                "Kunde AG",
                AddressBuilder::new("München", "80331", "DE").build(),
            )
            .build(),
        )
        .add_line(
            LineItemBuilder::new("1", "Consulting", dec!(10), "HUR", dec!(150))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .build()
        .unwrap();

    let errors = validate_14_ustg(&invoice);
    println!("  Found {} validation errors:", errors.len());
    for e in &errors {
        println!("  - {}", e);
    }

    // ── 3. build_strict rejects invalid invoices ──────────────────────
    println!("\n=== build_strict() ===");
    let result = InvoiceBuilder::new("RE-2024-002", NaiveDate::from_ymd_opt(2024, 6, 15).unwrap())
        .seller(
            PartyBuilder::new(
                "ACME GmbH",
                AddressBuilder::new("Berlin", "10115", "DE").build(),
            )
            .build(),
        )
        .buyer(
            PartyBuilder::new(
                "Kunde AG",
                AddressBuilder::new("München", "80331", "DE").build(),
            )
            .build(),
        )
        .add_line(
            LineItemBuilder::new("1", "Consulting", dec!(10), "HUR", dec!(150))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .build_strict();

    match result {
        Ok(_) => println!("  Invoice is fully valid"),
        Err(e) => println!("  Rejected: {}", e),
    }

    // ── 4. XML parse failure ──────────────────────────────────────────
    println!("\n=== XML Parse Errors ===");

    let bad_xml = "<not-an-invoice>hello</not-an-invoice>";
    match faktura::xrechnung::from_xml(bad_xml) {
        Ok(_) => println!("  Parsed successfully (unexpected)"),
        Err(e) => println!("  Parse error: {}", e),
    }

    let empty = "";
    match faktura::xrechnung::from_ubl_xml(empty) {
        Ok(_) => println!("  Parsed successfully (unexpected)"),
        Err(e) => println!("  Empty input: {}", e),
    }
}
