#![cfg(feature = "zugferd")]

use chrono::NaiveDate;
use faktura::core::*;
use faktura::xrechnung;
use faktura::zugferd::{self, FACTURX_FILENAME, ZugferdProfile};
use rust_decimal_macros::dec;

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
        })
        .payment_terms("Zahlbar innerhalb von 30 Tagen")
        .build()
        .unwrap()
}

/// Create a minimal valid PDF in memory using lopdf.
fn minimal_pdf() -> Vec<u8> {
    use lopdf::{Document, Object, Stream, dictionary};

    let mut doc = Document::with_version("1.7");

    // A minimal page with empty content
    let pages_id = doc.new_object_id();
    let font_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
    });
    let resources_id = doc.add_object(dictionary! {
        "Font" => dictionary! {
            "F1" => Object::Reference(font_id),
        },
    });
    let content = Stream::new(
        dictionary! {},
        b"BT /F1 12 Tf 100 700 Td (Invoice) Tj ET".to_vec(),
    );
    let content_id = doc.add_object(content);
    let page_id = doc.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => Object::Reference(pages_id),
        "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
        "Contents" => Object::Reference(content_id),
        "Resources" => Object::Reference(resources_id),
    });
    let pages = dictionary! {
        "Type" => "Pages",
        "Kids" => vec![Object::Reference(page_id)],
        "Count" => 1,
    };
    doc.objects.insert(pages_id, Object::Dictionary(pages));

    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => Object::Reference(pages_id),
    });
    doc.trailer.set("Root", Object::Reference(catalog_id));

    let mut output = Vec::new();
    doc.save_to(&mut output).expect("save minimal PDF");
    output
}

// ---------------------------------------------------------------------------
// XML Generation per Profile
// ---------------------------------------------------------------------------

#[test]
fn xml_generation_en16931_profile() {
    let inv = test_invoice();
    let xml = zugferd::to_xml(&inv, ZugferdProfile::EN16931).unwrap();

    assert!(xml.contains("urn:cen.eu:en16931:2017"));
    assert!(xml.contains("rsm:CrossIndustryInvoice"));
    assert!(xml.contains("RE-2024-001"));
    // Should NOT contain the XRechnung-specific URN
    assert!(!xml.contains("xeinkauf.de"));
}

#[test]
fn xml_generation_minimum_profile() {
    let inv = test_invoice();
    let xml = zugferd::to_xml(&inv, ZugferdProfile::Minimum).unwrap();
    assert!(xml.contains("urn:factur-x.eu:1p0:minimum"));
}

#[test]
fn xml_generation_basic_profile() {
    let inv = test_invoice();
    let xml = zugferd::to_xml(&inv, ZugferdProfile::Basic).unwrap();
    assert!(xml.contains("urn:cen.eu:en16931:2017#compliant#urn:factur-x.eu:1p0:basic"));
}

#[test]
fn xml_generation_extended_profile() {
    let inv = test_invoice();
    let xml = zugferd::to_xml(&inv, ZugferdProfile::Extended).unwrap();
    assert!(xml.contains("urn:cen.eu:en16931:2017#conformant#urn:factur-x.eu:1p0:extended"));
}

#[test]
fn xml_generation_xrechnung_profile() {
    let inv = test_invoice();
    let xml = zugferd::to_xml(&inv, ZugferdProfile::XRechnung).unwrap();
    assert!(xml.contains(xrechnung::XRECHNUNG_CUSTOMIZATION_ID));
}

// ---------------------------------------------------------------------------
// PDF Embedding
// ---------------------------------------------------------------------------

#[test]
fn embed_xml_into_pdf() {
    let inv = test_invoice();
    let xml = zugferd::to_xml(&inv, ZugferdProfile::EN16931).unwrap();
    let pdf = minimal_pdf();

    let result = zugferd::embed_in_pdf(&pdf, &xml, ZugferdProfile::EN16931);
    assert!(result.is_ok(), "embed failed: {:?}", result.err());

    let zugferd_pdf = result.unwrap();
    // Should be larger than the original
    assert!(zugferd_pdf.len() > pdf.len());
    // Should still be valid PDF (starts with %PDF)
    assert!(zugferd_pdf.starts_with(b"%PDF"));
}

#[test]
fn embed_creates_valid_pdf_structure() {
    let inv = test_invoice();
    let xml = zugferd::to_xml(&inv, ZugferdProfile::EN16931).unwrap();
    let pdf = minimal_pdf();
    let zugferd_pdf = zugferd::embed_in_pdf(&pdf, &xml, ZugferdProfile::EN16931).unwrap();

    // Verify we can load the result
    let doc = lopdf::Document::load_mem(&zugferd_pdf).unwrap();
    let catalog = doc.catalog().unwrap();

    // AF array should exist
    assert!(catalog.get(b"AF").is_ok(), "AF array missing from catalog");
    // Names dictionary should exist
    assert!(
        catalog.get(b"Names").is_ok(),
        "Names dict missing from catalog"
    );
    // Metadata should exist
    assert!(
        catalog.get(b"Metadata").is_ok(),
        "Metadata missing from catalog"
    );
}

// ---------------------------------------------------------------------------
// Roundtrip: Embed → Extract
// ---------------------------------------------------------------------------

#[test]
fn embed_extract_roundtrip() {
    let inv = test_invoice();
    let xml = zugferd::to_xml(&inv, ZugferdProfile::EN16931).unwrap();
    let pdf = minimal_pdf();

    // Embed
    let zugferd_pdf = zugferd::embed_in_pdf(&pdf, &xml, ZugferdProfile::EN16931).unwrap();

    // Extract
    let extracted = zugferd::extract_from_pdf(&zugferd_pdf).unwrap();

    // Compare — the extracted XML should match what we embedded
    assert_eq!(extracted, xml);
}

#[test]
fn roundtrip_preserves_invoice_data() {
    let inv = test_invoice();
    let xml = zugferd::to_xml(&inv, ZugferdProfile::EN16931).unwrap();
    let pdf = minimal_pdf();

    let zugferd_pdf = zugferd::embed_in_pdf(&pdf, &xml, ZugferdProfile::EN16931).unwrap();
    let extracted_xml = zugferd::extract_from_pdf(&zugferd_pdf).unwrap();

    // Parse the extracted XML back to an invoice
    let parsed = xrechnung::from_cii_xml(&extracted_xml).unwrap();
    assert_eq!(parsed.number, "RE-2024-001");
    assert_eq!(parsed.seller.name, "ACME GmbH");
    assert_eq!(parsed.buyer.name, "Kunde AG");
    assert_eq!(parsed.lines.len(), 1);
    assert_eq!(parsed.lines[0].item_name, "Beratung");

    let totals = parsed.totals.as_ref().unwrap();
    assert_eq!(totals.line_net_total, dec!(1500));
    assert_eq!(totals.vat_total, dec!(285));
    assert_eq!(totals.gross_total, dec!(1785));
}

// ---------------------------------------------------------------------------
// Extract from non-ZUGFeRD PDF
// ---------------------------------------------------------------------------

#[test]
fn extract_from_plain_pdf_fails() {
    let pdf = minimal_pdf();
    let result = zugferd::extract_from_pdf(&pdf);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// XMP Metadata
// ---------------------------------------------------------------------------

#[test]
fn embed_contains_xmp_metadata() {
    let inv = test_invoice();
    let xml = zugferd::to_xml(&inv, ZugferdProfile::EN16931).unwrap();
    let pdf = minimal_pdf();
    let zugferd_pdf = zugferd::embed_in_pdf(&pdf, &xml, ZugferdProfile::EN16931).unwrap();

    // Check the raw bytes contain XMP markers
    let pdf_str = String::from_utf8_lossy(&zugferd_pdf);
    assert!(
        pdf_str.contains("pdfaid:part"),
        "missing PDF/A identification"
    );
    assert!(pdf_str.contains("EN 16931"), "missing conformance level");
    assert!(
        pdf_str.contains(FACTURX_FILENAME),
        "missing filename in XMP"
    );
    assert!(pdf_str.contains("urn:factur-x:pdfa:CrossIndustryDocument:invoice:1p0#"));
}

#[test]
fn embed_xrechnung_profile_xmp() {
    let inv = test_invoice();
    let xml = zugferd::to_xml(&inv, ZugferdProfile::XRechnung).unwrap();
    let pdf = minimal_pdf();
    let zugferd_pdf = zugferd::embed_in_pdf(&pdf, &xml, ZugferdProfile::XRechnung).unwrap();

    let pdf_str = String::from_utf8_lossy(&zugferd_pdf);
    assert!(
        pdf_str.contains("XRECHNUNG"),
        "XMP should have XRECHNUNG conformance level"
    );
}

// ---------------------------------------------------------------------------
// Profile properties
// ---------------------------------------------------------------------------

#[test]
fn profile_urns() {
    assert_eq!(ZugferdProfile::Minimum.urn(), "urn:factur-x.eu:1p0:minimum");
    assert_eq!(ZugferdProfile::EN16931.urn(), "urn:cen.eu:en16931:2017");
    assert_eq!(
        ZugferdProfile::XRechnung.urn(),
        xrechnung::XRECHNUNG_CUSTOMIZATION_ID,
    );
}

#[test]
fn profile_af_relationships() {
    assert_eq!(ZugferdProfile::Minimum.af_relationship(), "Data");
    assert_eq!(ZugferdProfile::BasicWl.af_relationship(), "Data");
    assert_eq!(ZugferdProfile::Basic.af_relationship(), "Alternative");
    assert_eq!(ZugferdProfile::EN16931.af_relationship(), "Alternative");
    assert_eq!(ZugferdProfile::Extended.af_relationship(), "Alternative");
    assert_eq!(ZugferdProfile::XRechnung.af_relationship(), "Alternative");
}

#[test]
fn profile_conformance_levels() {
    assert_eq!(ZugferdProfile::Minimum.conformance_level(), "MINIMUM");
    assert_eq!(ZugferdProfile::BasicWl.conformance_level(), "BASIC WL");
    assert_eq!(ZugferdProfile::Basic.conformance_level(), "BASIC");
    assert_eq!(ZugferdProfile::EN16931.conformance_level(), "EN 16931");
    assert_eq!(ZugferdProfile::Extended.conformance_level(), "EXTENDED");
    assert_eq!(ZugferdProfile::XRechnung.conformance_level(), "XRECHNUNG");
}
