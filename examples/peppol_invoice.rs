use chrono::NaiveDate;
use faktura::core::*;
use faktura::peppol;
use rust_decimal_macros::dec;

fn main() {
    // Build a Peppol-compliant cross-border invoice (DE seller → NL buyer)
    let invoice = InvoiceBuilder::new("PEPP-2024-001", NaiveDate::from_ymd_opt(2024, 6, 15).unwrap())
        .tax_point_date(NaiveDate::from_ymd_opt(2024, 6, 15).unwrap())
        .buyer_reference("PO-2024-4711")
        .seller(
            PartyBuilder::new(
                "ACME GmbH",
                AddressBuilder::new("Berlin", "10115", "DE")
                    .street("Friedrichstraße 123")
                    .build(),
            )
            .vat_id("DE123456789")
            .electronic_address("DE123456789", "9930") // EAS 9930 = DE VAT
            .build(),
        )
        .buyer(
            PartyBuilder::new(
                "Dutch Corp B.V.",
                AddressBuilder::new("Amsterdam", "1012 AB", "NL")
                    .street("Keizersgracht 456")
                    .build(),
            )
            .vat_id("NL123456789B01")
            .electronic_address("NL123456789B01", "9944") // EAS 9944 = NL VAT
            .build(),
        )
        .add_line(
            LineItemBuilder::new("1", "Consulting services", dec!(20), "HUR", dec!(150))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .add_line(
            LineItemBuilder::new("2", "Software license", dec!(1), "C62", dec!(500))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .build()
        .expect("invoice valid");

    // Validate Peppol-specific rules
    println!("=== Peppol Validation ===\n");
    let errors = peppol::validate_peppol(&invoice);
    if errors.is_empty() {
        println!("  No validation errors.\n");
    } else {
        for err in &errors {
            println!("  ERROR: {err}");
        }
        println!();
    }

    // Generate Peppol UBL XML
    let xml = peppol::to_ubl_xml(&invoice).expect("XML generation failed");
    println!("=== Peppol UBL XML (first 15 lines) ===\n");
    for line in xml.lines().take(15) {
        println!("{line}");
    }
    println!("...\n");

    // Show Peppol constants
    println!("=== Peppol Identifiers ===\n");
    println!("  CustomizationID: {}", peppol::PEPPOL_CUSTOMIZATION_ID);
    println!("  ProfileID:       {}", peppol::PEPPOL_PROFILE_ID);

    // EAS scheme lookup
    println!("\n=== EAS Scheme Defaults ===\n");
    for cc in &["DE", "NL", "FR", "IT", "SE"] {
        if let Some(scheme) = peppol::eas_scheme_for_country(cc) {
            println!("  {cc} → {} ({})", scheme.code, scheme.description);
        }
    }
}
