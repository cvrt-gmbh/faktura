use chrono::NaiveDate;
use faktura::core::*;
use faktura::zugferd::{self, ZugferdProfile};
use rust_decimal_macros::dec;

fn main() {
    let invoice = InvoiceBuilder::new("RE-2024-001", NaiveDate::from_ymd_opt(2024, 6, 15).unwrap())
        .seller(
            PartyBuilder::new(
                "ACME GmbH",
                AddressBuilder::new("Berlin", "10115", "DE")
                    .street("Friedrichstraße 123")
                    .build(),
            )
            .vat_id("DE123456789")
            .electronic_address("EM", "billing@acme.de")
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
            LineItemBuilder::new("1", "IT-Beratung", dec!(20), "HUR", dec!(150))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .build()
        .expect("invoice should be valid");

    // Generate ZUGFeRD XML for different profiles
    for profile in [
        ZugferdProfile::Minimum,
        ZugferdProfile::BasicWl,
        ZugferdProfile::Basic,
        ZugferdProfile::EN16931,
        ZugferdProfile::Extended,
        ZugferdProfile::XRechnung,
    ] {
        let xml = zugferd::to_xml(&invoice, profile).expect("ZUGFeRD XML failed");
        println!("{:?}: {} bytes", profile, xml.len());
    }

    // Embedding into a PDF requires a real PDF file
    // Here we just demonstrate the XML generation
    println!("\nTo embed into a PDF:");
    println!("  let pdf_bytes = std::fs::read(\"invoice.pdf\").unwrap();");
    println!(
        "  let result = zugferd::embed_in_pdf(&pdf_bytes, &invoice, ZugferdProfile::EN16931);"
    );
}
