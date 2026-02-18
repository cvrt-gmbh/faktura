#![cfg(feature = "peppol")]

use chrono::NaiveDate;
use faktura::core::*;
use faktura::peppol::*;
use rust_decimal_macros::dec;

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

fn peppol_invoice() -> Invoice {
    InvoiceBuilder::new("PEPP-001", date(2024, 6, 15))
        .buyer_reference("BR-123")
        .seller(
            PartyBuilder::new(
                "Seller GmbH",
                AddressBuilder::new("Berlin", "10115", "DE").build(),
            )
            .vat_id("DE123456789")
            .electronic_address("EM", "seller@peppol.eu")
            .contact(
                Some("Max Mustermann".into()),
                Some("+49 30 12345".into()),
                Some("max@seller.de".into()),
            )
            .build(),
        )
        .buyer(
            PartyBuilder::new(
                "Buyer AG",
                AddressBuilder::new("München", "80331", "DE").build(),
            )
            .electronic_address("EM", "buyer@peppol.eu")
            .build(),
        )
        .due_date(date(2024, 7, 15))
        .payment(PaymentInstructions {
            means_code: PaymentMeansCode::SepaCreditTransfer,
            means_text: None,
            remittance_info: Some("PEPP-001".into()),
            credit_transfer: Some(CreditTransfer {
                iban: "DE89370400440532013000".into(),
                bic: None,
                account_name: None,
            }),
        })
        .add_line(
            LineItemBuilder::new("1", "Consulting services", dec!(10), "HUR", dec!(150))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .add_line(
            LineItemBuilder::new("2", "Documentation", dec!(5), "HUR", dec!(100))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .build()
        .unwrap()
}

// ---------------------------------------------------------------------------
// UBL Generation
// ---------------------------------------------------------------------------

#[test]
fn peppol_ubl_contains_customization_id() {
    let xml = to_ubl_xml(&peppol_invoice()).unwrap();
    assert!(xml.contains(PEPPOL_CUSTOMIZATION_ID));
    // Must NOT contain the XRechnung customization ID
    assert!(
        !xml.contains("urn:cen.eu:en16931:2017#compliant#urn:xoev-de:kosit:standard:xrechnung_3.0")
    );
}

#[test]
fn peppol_ubl_contains_profile_id() {
    let xml = to_ubl_xml(&peppol_invoice()).unwrap();
    assert!(xml.contains(PEPPOL_PROFILE_ID));
}

#[test]
fn peppol_ubl_is_valid_xml() {
    let xml = to_ubl_xml(&peppol_invoice()).unwrap();
    assert!(xml.starts_with("<?xml"));
    assert!(xml.contains("<cbc:ID>PEPP-001</cbc:ID>"));
}

#[test]
fn peppol_ubl_contains_seller_endpoint() {
    let xml = to_ubl_xml(&peppol_invoice()).unwrap();
    assert!(xml.contains("seller@peppol.eu"));
}

#[test]
fn peppol_ubl_contains_buyer_endpoint() {
    let xml = to_ubl_xml(&peppol_invoice()).unwrap();
    assert!(xml.contains("buyer@peppol.eu"));
}

#[test]
fn peppol_ubl_contains_line_items() {
    let xml = to_ubl_xml(&peppol_invoice()).unwrap();
    assert!(xml.contains("Consulting services"));
    assert!(xml.contains("Documentation"));
}

// ---------------------------------------------------------------------------
// Roundtrip
// ---------------------------------------------------------------------------

#[test]
fn peppol_roundtrip() {
    let original = peppol_invoice();
    let xml = to_ubl_xml(&original).unwrap();
    let parsed = from_ubl_xml(&xml).unwrap();

    assert_eq!(parsed.number, original.number);
    assert_eq!(parsed.issue_date, original.issue_date);
    assert_eq!(parsed.seller.name, original.seller.name);
    assert_eq!(parsed.buyer.name, original.buyer.name);
    assert_eq!(parsed.lines.len(), original.lines.len());
    assert_eq!(parsed.lines[0].item_name, "Consulting services");
    assert_eq!(parsed.lines[1].item_name, "Documentation");
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

#[test]
fn valid_peppol_invoice_passes_all_checks() {
    let errors = validate_peppol(&peppol_invoice());
    assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
}

#[test]
fn missing_buyer_reference_fails_r003() {
    let mut inv = peppol_invoice();
    inv.buyer_reference = None;
    inv.order_reference = None;
    let errors = validate_peppol(&inv);
    assert!(
        errors
            .iter()
            .any(|e| e.rule.as_deref() == Some("PEPPOL-EN16931-R003"))
    );
}

#[test]
fn order_reference_satisfies_r003() {
    let mut inv = peppol_invoice();
    inv.buyer_reference = None;
    inv.order_reference = Some("PO-456".into());
    let errors = validate_peppol(&inv);
    assert!(
        !errors
            .iter()
            .any(|e| e.rule.as_deref() == Some("PEPPOL-EN16931-R003"))
    );
}

#[test]
fn missing_seller_electronic_address_fails_r020() {
    let mut inv = peppol_invoice();
    inv.seller.electronic_address = None;
    let errors = validate_peppol(&inv);
    assert!(
        errors
            .iter()
            .any(|e| e.rule.as_deref() == Some("PEPPOL-EN16931-R020"))
    );
}

#[test]
fn missing_buyer_electronic_address_fails_r010() {
    let mut inv = peppol_invoice();
    inv.buyer.electronic_address = None;
    let errors = validate_peppol(&inv);
    assert!(
        errors
            .iter()
            .any(|e| e.rule.as_deref() == Some("PEPPOL-EN16931-R010"))
    );
}

#[test]
fn empty_seller_name_fails_r008() {
    let mut inv = peppol_invoice();
    inv.seller.name = String::new();
    let errors = validate_peppol(&inv);
    assert!(
        errors
            .iter()
            .any(|e| e.rule.as_deref() == Some("PEPPOL-EN16931-R008"))
    );
}

#[test]
fn empty_buyer_name_fails_r008() {
    let mut inv = peppol_invoice();
    inv.buyer.name = String::new();
    let errors = validate_peppol(&inv);
    assert!(
        errors
            .iter()
            .any(|e| e.rule.as_deref() == Some("PEPPOL-EN16931-R008"))
    );
}

#[test]
fn empty_invoice_number_fails_r008() {
    let mut inv = peppol_invoice();
    inv.number = String::new();
    let errors = validate_peppol(&inv);
    assert!(
        errors
            .iter()
            .any(|e| e.rule.as_deref() == Some("PEPPOL-EN16931-R008"))
    );
}

#[test]
fn partial_invoice_non_german_buyer_fails_p0112() {
    let mut inv = peppol_invoice();
    inv.type_code = InvoiceTypeCode::Partial;
    inv.buyer.address.country_code = "FR".into();
    let errors = validate_peppol(&inv);
    assert!(
        errors
            .iter()
            .any(|e| e.rule.as_deref() == Some("PEPPOL-EN16931-P0112"))
    );
}

#[test]
fn corrected_invoice_both_german_passes_p0112() {
    let mut inv = peppol_invoice();
    inv.type_code = InvoiceTypeCode::Corrected;
    // Both seller and buyer are DE by default
    let errors = validate_peppol(&inv);
    assert!(
        !errors
            .iter()
            .any(|e| e.rule.as_deref() == Some("PEPPOL-EN16931-P0112"))
    );
}

#[test]
fn zero_quantity_line_fails_r121() {
    let mut inv = peppol_invoice();
    inv.lines[0].quantity = dec!(0);
    let errors = validate_peppol(&inv);
    assert!(
        errors
            .iter()
            .any(|e| e.rule.as_deref() == Some("PEPPOL-EN16931-R121"))
    );
}

#[test]
fn negative_quantity_line_fails_r121() {
    let mut inv = peppol_invoice();
    inv.lines[0].quantity = dec!(-5);
    let errors = validate_peppol(&inv);
    assert!(
        errors
            .iter()
            .any(|e| e.rule.as_deref() == Some("PEPPOL-EN16931-R121"))
    );
}

// ---------------------------------------------------------------------------
// EAS Schemes
// ---------------------------------------------------------------------------

#[test]
fn eas_de_default_leitweg() {
    let s = eas_scheme_for_country("DE").unwrap();
    assert_eq!(s.code, "0204");
    assert_eq!(s.description, "Leitweg-ID");
}

#[test]
fn eas_de_vat_constant() {
    assert_eq!(EasScheme::DE_VAT.code, "9930");
}

#[test]
fn eas_gln_constant() {
    assert_eq!(EasScheme::GLN.code, "0088");
}

#[test]
fn eas_multiple_countries() {
    assert_eq!(eas_scheme_for_country("AT").unwrap().code, "9914");
    assert_eq!(eas_scheme_for_country("NL").unwrap().code, "0190");
    assert_eq!(eas_scheme_for_country("FR").unwrap().code, "9957");
    assert_eq!(eas_scheme_for_country("IT").unwrap().code, "0210");
    assert_eq!(eas_scheme_for_country("SE").unwrap().code, "0007");
    assert_eq!(eas_scheme_for_country("NO").unwrap().code, "0192");
}

#[test]
fn eas_unknown_country() {
    assert!(eas_scheme_for_country("XX").is_none());
}

#[test]
fn eas_case_insensitive() {
    assert!(eas_scheme_for_country("de").is_some());
    assert!(eas_scheme_for_country("De").is_some());
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

#[test]
fn peppol_constants_are_correct() {
    assert!(PEPPOL_CUSTOMIZATION_ID.contains("peppol.eu"));
    assert!(PEPPOL_PROFILE_ID.contains("peppol.eu"));
    assert!(PEPPOL_INVOICE_DOCTYPE.contains("Invoice"));
    assert!(PEPPOL_CREDIT_NOTE_DOCTYPE.contains("CreditNote"));
}
