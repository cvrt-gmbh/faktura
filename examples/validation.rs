use chrono::NaiveDate;
use faktura::core::*;
use rust_decimal_macros::dec;

fn main() {
    // Build a valid invoice
    let invoice = InvoiceBuilder::new("RE-2024-001", NaiveDate::from_ymd_opt(2024, 6, 15).unwrap())
        .tax_point_date(NaiveDate::from_ymd_opt(2024, 6, 15).unwrap())
        .seller(
            PartyBuilder::new(
                "ACME GmbH",
                AddressBuilder::new("Berlin", "10115", "DE")
                    .street("Friedrichstraße 123")
                    .build(),
            )
            .vat_id("DE123456789")
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
        .expect("invoice should be valid");

    // §14 UStG validation
    let errors_14 = validate_14_ustg(&invoice);
    println!("§14 UStG validation: {} errors", errors_14.len());
    for e in &errors_14 {
        println!("  {}", e);
    }

    // EN 16931 validation
    let errors_en = validate_en16931(&invoice);
    println!("EN 16931 validation: {} errors", errors_en.len());
    for e in &errors_en {
        println!("  {}", e);
    }

    // Arithmetic validation
    let errors_arith = validate_arithmetic(&invoice);
    println!("Arithmetic validation: {} errors", errors_arith.len());
    for e in &errors_arith {
        println!("  {}", e);
    }

    // Invoice number sequencing
    let mut seq = InvoiceNumberSequence::new("RE-", 2024);
    println!("\nGenerated numbers:");
    for _ in 0..5 {
        println!("  {}", seq.next_number());
    }
}
