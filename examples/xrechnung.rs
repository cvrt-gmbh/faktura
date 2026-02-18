use chrono::NaiveDate;
use faktura::core::*;
use rust_decimal_macros::dec;

fn main() {
    let invoice = InvoiceBuilder::new("RE-2024-042", NaiveDate::from_ymd_opt(2024, 6, 15).unwrap())
        .buyer_reference("04011000-12345-67") // Leitweg-ID
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
                "Stadtverwaltung Musterstadt",
                AddressBuilder::new("Musterstadt", "12345", "DE")
                    .street("Rathausplatz 1")
                    .build(),
            )
            .electronic_address("EM", "rechnung@musterstadt.de")
            .build(),
        )
        .add_line(
            LineItemBuilder::new("1", "IT-Beratung", dec!(40), "HUR", dec!(150))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .payment(PaymentInstructions {
            means_code: PaymentMeansCode::SepaCreditTransfer,
            means_text: None,
            remittance_info: Some("RE-2024-042".into()),
            credit_transfer: Some(CreditTransfer {
                iban: "DE89370400440532013000".into(),
                bic: Some("COBADEFFXXX".into()),
                account_name: Some("ACME GmbH".into()),
            }),
        })
        .build()
        .expect("invoice should be valid");

    // Generate UBL XML
    let ubl_xml = faktura::xrechnung::to_ubl_xml(&invoice).expect("UBL generation failed");
    println!("=== XRechnung UBL 2.1 ===");
    println!("{}", &ubl_xml[..500.min(ubl_xml.len())]);
    println!("... ({} bytes total)\n", ubl_xml.len());

    // Generate CII XML
    let cii_xml = faktura::xrechnung::to_cii_xml(&invoice).expect("CII generation failed");
    println!("=== XRechnung CII ===");
    println!("{}", &cii_xml[..500.min(cii_xml.len())]);
    println!("... ({} bytes total)\n", cii_xml.len());

    // Parse UBL back
    let parsed = faktura::xrechnung::from_ubl_xml(&ubl_xml).expect("UBL parsing failed");
    println!(
        "Roundtrip: {} -> {} lines",
        parsed.number,
        parsed.lines.len()
    );
}
