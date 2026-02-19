#![cfg(feature = "zugferd")]

//! Integration tests against reference ZUGFeRD/Factur-X PDFs from
//! <https://github.com/ZUGFeRD/mustangproject>.

use chrono::NaiveDate;
use faktura::core::*;
use faktura::xrechnung;
use faktura::zugferd::{self, ZugferdProfile};
use rust_decimal_macros::dec;
use std::fs;
use std::path::Path;

fn fixtures_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/zugferd-pdfs")
}

// ---------------------------------------------------------------------------
// Extract from reference PDFs
// ---------------------------------------------------------------------------

#[test]
fn extract_from_en16931_einfach() {
    let path = fixtures_dir().join("EN16931_Einfach.pdf");
    if !path.exists() {
        eprintln!("skipping: EN16931_Einfach.pdf not found");
        return;
    }
    let pdf_bytes = fs::read(&path).unwrap();
    let xml = zugferd::extract_from_pdf(&pdf_bytes).unwrap();

    // Should be valid CII XML
    assert!(
        xml.contains("CrossIndustryInvoice"),
        "extracted XML should be CII format"
    );

    // Parse it
    let inv = xrechnung::from_cii_xml(&xml).unwrap();
    assert!(!inv.number.is_empty(), "invoice number should not be empty");
    assert!(!inv.lines.is_empty(), "should have at least one line");
}

#[test]
fn extract_from_mustang_beispiel() {
    let path = fixtures_dir().join("MustangBeispiel20221026.pdf");
    if !path.exists() {
        eprintln!("skipping: MustangBeispiel20221026.pdf not found");
        return;
    }
    let pdf_bytes = fs::read(&path).unwrap();
    let xml = zugferd::extract_from_pdf(&pdf_bytes).unwrap();

    assert!(xml.contains("CrossIndustryInvoice"));

    let inv = xrechnung::from_cii_xml(&xml).unwrap();
    assert!(!inv.number.is_empty());
    assert!(!inv.seller.name.is_empty());
}

#[test]
fn extract_from_extended_pdfa3a() {
    let path = fixtures_dir().join("zugferd_2p1_EXTENDED_PDFA-3A.pdf");
    if !path.exists() {
        eprintln!("skipping: zugferd_2p1_EXTENDED_PDFA-3A.pdf not found");
        return;
    }
    let pdf_bytes = fs::read(&path).unwrap();
    let xml = zugferd::extract_from_pdf(&pdf_bytes).unwrap();

    assert!(xml.contains("CrossIndustryInvoice"));

    let inv = xrechnung::from_cii_xml(&xml).unwrap();
    assert!(!inv.number.is_empty());
}

// ---------------------------------------------------------------------------
// Embed into reference PDFs (multi-page input test)
// ---------------------------------------------------------------------------

#[test]
fn embed_into_extended_pdf_preserves_pages() {
    let path = fixtures_dir().join("zugferd_2p1_EXTENDED_PDFA-3A.pdf");
    if !path.exists() {
        eprintln!("skipping: zugferd_2p1_EXTENDED_PDFA-3A.pdf not found");
        return;
    }
    let pdf_bytes = fs::read(&path).unwrap();

    // Build a test invoice
    let inv = test_invoice();
    let xml = zugferd::to_xml(&inv, ZugferdProfile::EN16931).unwrap();

    // Embed into the real PDF
    let result = zugferd::embed_in_pdf(&pdf_bytes, &xml, ZugferdProfile::EN16931);
    assert!(
        result.is_ok(),
        "embed into real PDF failed: {:?}",
        result.err()
    );

    let output = result.unwrap();
    assert!(output.starts_with(b"%PDF"), "output should be valid PDF");
    assert!(output.len() > pdf_bytes.len(), "output should be larger");

    // Verify we can extract the XML back
    let extracted = zugferd::extract_from_pdf(&output).unwrap();
    assert_eq!(extracted, xml);

    // Verify page count preserved
    let original_doc = lopdf::Document::load_mem(&pdf_bytes).unwrap();
    let output_doc = lopdf::Document::load_mem(&output).unwrap();
    let original_pages = original_doc.get_pages().len();
    let output_pages = output_doc.get_pages().len();
    assert_eq!(
        original_pages, output_pages,
        "page count should be preserved: original={}, output={}",
        original_pages, output_pages
    );
}

#[test]
fn embed_into_mustang_beispiel() {
    let path = fixtures_dir().join("MustangBeispiel20221026.pdf");
    if !path.exists() {
        eprintln!("skipping: MustangBeispiel20221026.pdf not found");
        return;
    }
    let pdf_bytes = fs::read(&path).unwrap();

    let inv = test_invoice();
    let xml = zugferd::to_xml(&inv, ZugferdProfile::EN16931).unwrap();

    let result = zugferd::embed_in_pdf(&pdf_bytes, &xml, ZugferdProfile::EN16931);
    assert!(result.is_ok(), "embed failed: {:?}", result.err());

    let output = result.unwrap();

    // Roundtrip extract
    let extracted = zugferd::extract_from_pdf(&output).unwrap();
    let parsed = xrechnung::from_cii_xml(&extracted).unwrap();
    assert_eq!(parsed.number, "RE-2024-001");
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

fn test_invoice() -> Invoice {
    InvoiceBuilder::new("RE-2024-001", date(2024, 6, 15))
        .due_date(date(2024, 7, 15))
        .tax_point_date(date(2024, 6, 15))
        .buyer_reference("04011000-12345-03")
        .seller(
            PartyBuilder::new(
                "ACME GmbH",
                AddressBuilder::new("Berlin", "10115", "DE")
                    .street("Friedrichstraße 123")
                    .build(),
            )
            .vat_id("DE123456789")
            .electronic_address("EM", "seller@acme.de")
            .contact(
                Some("Max Mustermann".into()),
                Some("+49 30 12345".into()),
                Some("max@acme.de".into()),
            )
            .build(),
        )
        .buyer(
            PartyBuilder::new(
                "Kunde AG",
                AddressBuilder::new("München", "80331", "DE")
                    .street("Marienplatz 1")
                    .build(),
            )
            .electronic_address("EM", "buyer@kunde.de")
            .build(),
        )
        .add_line(
            LineItemBuilder::new("1", "Beratung", dec!(10), "HUR", dec!(150))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .payment(PaymentInstructions {
            means_code: PaymentMeansCode::SepaCreditTransfer,
            means_text: None,
            remittance_info: Some("RE-2024-001".into()),
            credit_transfer: Some(CreditTransfer {
                iban: "DE89370400440532013000".into(),
                bic: Some("COBADEFFXXX".into()),
                account_name: Some("ACME GmbH".into()),
            }),
            card_payment: None,
            direct_debit: None,
        })
        .payment_terms("Zahlbar innerhalb von 30 Tagen")
        .build()
        .unwrap()
}
