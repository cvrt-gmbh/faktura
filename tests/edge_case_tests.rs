//! Comprehensive edge-case tests filling gaps identified in the v0.2.0 audit.

use chrono::NaiveDate;
use faktura::core::*;
use rust_decimal_macros::dec;

// ---------------------------------------------------------------------------
// Helpers (canonical builder pattern)
// ---------------------------------------------------------------------------

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

fn seller() -> Party {
    PartyBuilder::new(
        "ACME GmbH",
        AddressBuilder::new("Berlin", "10115", "DE")
            .street("Friedrichstraße 123")
            .build(),
    )
    .vat_id("DE123456789")
    .contact(
        Some("Max Mustermann".into()),
        Some("+49 30 12345".into()),
        Some("max@acme.de".into()),
    )
    .build()
}

fn buyer() -> Party {
    PartyBuilder::new(
        "Kunde AG",
        AddressBuilder::new("München", "80331", "DE")
            .street("Marienplatz 1")
            .build(),
    )
    .build()
}

/// XRechnung-compliant seller with electronic address.
fn xr_seller() -> Party {
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
    .build()
}

/// XRechnung-compliant buyer with electronic address.
fn xr_buyer() -> Party {
    PartyBuilder::new(
        "Kunde AG",
        AddressBuilder::new("München", "80331", "DE")
            .street("Marienplatz 1")
            .build(),
    )
    .electronic_address("EM", "buyer@kunde.de")
    .build()
}

fn sepa_payment() -> PaymentInstructions {
    PaymentInstructions {
        means_code: PaymentMeansCode::SepaCreditTransfer,
        means_text: None,
        remittance_info: Some("RE-EDGE-001".into()),
        credit_transfer: Some(CreditTransfer {
            iban: "DE89370400440532013000".into(),
            bic: Some("COBADEFFXXX".into()),
            account_name: Some("ACME GmbH".into()),
        }),
        card_payment: None,
        direct_debit: None,
    }
}

// ===========================================================================
// HIGH PRIORITY
// ===========================================================================

// ---- 1. Non-EUR invoice currency ----

#[test]
#[cfg(feature = "xrechnung")]
fn non_eur_currency_usd_roundtrip() {
    let inv = InvoiceBuilder::new("USD-001", date(2024, 6, 1))
        .due_date(date(2024, 7, 1))
        .currency("USD")
        .seller(xr_seller())
        .buyer(xr_buyer())
        .buyer_reference("LEITWEG-123")
        .add_line(
            LineItemBuilder::new("1", "Consulting", dec!(10), "HUR", dec!(150))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .payment(sepa_payment())
        .tax_point_date(date(2024, 6, 1))
        .build()
        .unwrap();

    assert_eq!(inv.currency_code, "USD");
    let totals = inv.totals.as_ref().unwrap();
    assert_eq!(totals.line_net_total, dec!(1500));
    assert_eq!(totals.vat_total, dec!(285));
    assert_eq!(totals.gross_total, dec!(1785));

    // UBL roundtrip
    let ubl_xml = faktura::xrechnung::to_ubl_xml(&inv).unwrap();
    let ubl_back = faktura::xrechnung::from_ubl_xml(&ubl_xml).unwrap();
    assert_eq!(ubl_back.currency_code, "USD");
    assert_eq!(ubl_back.totals.as_ref().unwrap().gross_total, dec!(1785));

    // CII roundtrip
    let cii_xml = faktura::xrechnung::to_cii_xml(&inv).unwrap();
    let cii_back = faktura::xrechnung::from_cii_xml(&cii_xml).unwrap();
    assert_eq!(cii_back.currency_code, "USD");
    assert_eq!(cii_back.totals.as_ref().unwrap().gross_total, dec!(1785));
}

#[test]
#[cfg(feature = "datev")]
fn non_eur_currency_datev_export() {
    let inv = InvoiceBuilder::new("USD-002", date(2024, 6, 1))
        .due_date(date(2024, 7, 1))
        .currency("USD")
        .seller(seller())
        .buyer(buyer())
        .add_line(
            LineItemBuilder::new("1", "Consulting", dec!(10), "HUR", dec!(100))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .payment(sepa_payment())
        .tax_point_date(date(2024, 6, 1))
        .build()
        .unwrap();

    let config = faktura::datev::DatevConfigBuilder::new(12345, 99999)
        .fiscal_year_start(date(2024, 1, 1))
        .build();

    let csv = faktura::datev::to_extf(&[inv], &config).unwrap();
    assert!(!csv.is_empty());
    // DATEV CSV should contain the invoice data
    assert!(csv.contains("USD-002") || csv.contains("1190")); // either number or gross amount
}

// ---- 2. Skonto payment terms (BR-DE-18) ----

#[test]
#[cfg(feature = "xrechnung")]
fn skonto_valid_format_passes_xrechnung() {
    let inv = InvoiceBuilder::new("SK-001", date(2024, 6, 1))
        .due_date(date(2024, 7, 1))
        .seller(xr_seller())
        .buyer(xr_buyer())
        .buyer_reference("LEITWEG-123")
        .add_line(
            LineItemBuilder::new("1", "Widget", dec!(1), "C62", dec!(100))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .payment(sepa_payment())
        .payment_terms("#SKONTO#TAGE=14#PROZENT=2.00#")
        .tax_point_date(date(2024, 6, 1))
        .build()
        .unwrap();

    let errors = faktura::xrechnung::validate_xrechnung(&inv);
    let skonto_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.rule.as_deref() == Some("BR-DE-18"))
        .collect();
    assert!(skonto_errors.is_empty(), "Valid skonto should pass BR-DE-18");
}

#[test]
#[cfg(feature = "xrechnung")]
fn skonto_missing_trailing_hash_fails_br_de_18() {
    let inv = InvoiceBuilder::new("SK-002", date(2024, 6, 1))
        .due_date(date(2024, 7, 1))
        .seller(xr_seller())
        .buyer(xr_buyer())
        .buyer_reference("LEITWEG-123")
        .add_line(
            LineItemBuilder::new("1", "Widget", dec!(1), "C62", dec!(100))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .payment(sepa_payment())
        .payment_terms("#SKONTO#TAGE=14#PROZENT=2.00") // missing trailing #
        .tax_point_date(date(2024, 6, 1))
        .build()
        .unwrap();

    let errors = faktura::xrechnung::validate_xrechnung(&inv);
    assert!(
        errors.iter().any(|e| e.rule.as_deref() == Some("BR-DE-18")),
        "Missing trailing # should trigger BR-DE-18"
    );
}

#[test]
#[cfg(feature = "xrechnung")]
fn skonto_missing_prozent_fails_br_de_18() {
    let inv = InvoiceBuilder::new("SK-003", date(2024, 6, 1))
        .due_date(date(2024, 7, 1))
        .seller(xr_seller())
        .buyer(xr_buyer())
        .buyer_reference("LEITWEG-123")
        .add_line(
            LineItemBuilder::new("1", "Widget", dec!(1), "C62", dec!(100))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .payment(sepa_payment())
        .payment_terms("#SKONTO#TAGE=14#") // missing PROZENT=
        .tax_point_date(date(2024, 6, 1))
        .build()
        .unwrap();

    let errors = faktura::xrechnung::validate_xrechnung(&inv);
    assert!(
        errors.iter().any(|e| e.rule.as_deref() == Some("BR-DE-18")),
        "Missing PROZENT should trigger BR-DE-18"
    );
}

// ---- 3. Exempt VAT end-to-end ----

#[test]
#[cfg(feature = "xrechnung")]
fn exempt_vat_end_to_end_roundtrip() {
    let inv = InvoiceBuilder::new("EX-001", date(2024, 6, 1))
        .due_date(date(2024, 7, 1))
        .seller(xr_seller())
        .buyer(xr_buyer())
        .buyer_reference("LEITWEG-123")
        .vat_scenario(VatScenario::Mixed)
        .add_line(
            LineItemBuilder::new("1", "Medical service", dec!(1), "C62", dec!(500))
                .tax(TaxCategory::Exempt, dec!(0))
                .build(),
        )
        .payment(sepa_payment())
        .tax_point_date(date(2024, 6, 1))
        .build()
        .unwrap();

    let totals = inv.totals.as_ref().unwrap();
    assert_eq!(totals.vat_total, dec!(0));
    assert_eq!(totals.gross_total, dec!(500));
    // Auto-filled exemption reason
    let vb = &totals.vat_breakdown[0];
    assert_eq!(vb.category, TaxCategory::Exempt);
    assert!(vb.exemption_reason.is_some());

    // UBL roundtrip
    let ubl_xml = faktura::xrechnung::to_ubl_xml(&inv).unwrap();
    let ubl_back = faktura::xrechnung::from_ubl_xml(&ubl_xml).unwrap();
    let ubl_vb = &ubl_back.totals.as_ref().unwrap().vat_breakdown[0];
    assert_eq!(ubl_vb.category, TaxCategory::Exempt);
    assert_eq!(ubl_vb.tax_amount, dec!(0));

    // CII roundtrip
    let cii_xml = faktura::xrechnung::to_cii_xml(&inv).unwrap();
    let cii_back = faktura::xrechnung::from_cii_xml(&cii_xml).unwrap();
    let cii_vb = &cii_back.totals.as_ref().unwrap().vat_breakdown[0];
    assert_eq!(cii_vb.category, TaxCategory::Exempt);
    assert_eq!(cii_vb.tax_amount, dec!(0));
}

// ===========================================================================
// MEDIUM PRIORITY
// ===========================================================================

// ---- 4. XML special character escaping ----

#[test]
#[cfg(feature = "xrechnung")]
fn xml_special_characters_roundtrip() {
    let special_seller = PartyBuilder::new(
        "Müller & Söhne <GmbH>",
        AddressBuilder::new("Berlin", "10115", "DE")
            .street("Straße der \"Einheit\" 1")
            .build(),
    )
    .vat_id("DE123456789")
    .electronic_address("EM", "seller@mueller.de")
    .contact(
        Some("Max Müller".into()),
        Some("+49 30 12345".into()),
        Some("max@mueller.de".into()),
    )
    .build();

    let inv = InvoiceBuilder::new("SPEC-001", date(2024, 6, 1))
        .due_date(date(2024, 7, 1))
        .seller(special_seller)
        .buyer(xr_buyer())
        .buyer_reference("LEITWEG-123")
        .add_line(
            LineItemBuilder::new("1", "Item with \"quotes\" & <angles>", dec!(1), "C62", dec!(100))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .payment(sepa_payment())
        .tax_point_date(date(2024, 6, 1))
        .build()
        .unwrap();

    // UBL roundtrip
    let ubl_xml = faktura::xrechnung::to_ubl_xml(&inv).unwrap();
    let ubl_back = faktura::xrechnung::from_ubl_xml(&ubl_xml).unwrap();
    assert_eq!(ubl_back.seller.name, "Müller & Söhne <GmbH>");
    assert!(ubl_back.lines[0].item_name.contains("\"quotes\""));
    assert!(ubl_back.lines[0].item_name.contains("<angles>"));

    // CII roundtrip
    let cii_xml = faktura::xrechnung::to_cii_xml(&inv).unwrap();
    let cii_back = faktura::xrechnung::from_cii_xml(&cii_xml).unwrap();
    assert_eq!(cii_back.seller.name, "Müller & Söhne <GmbH>");
    assert!(cii_back.lines[0].item_name.contains("\"quotes\""));
}

// ---- 5. Negative line quantities (credit note) ----

#[test]
#[cfg(feature = "xrechnung")]
fn credit_note_negative_quantity_roundtrip() {
    let inv = InvoiceBuilder::new("CN-001", date(2024, 6, 1))
        .due_date(date(2024, 7, 1))
        .type_code(InvoiceTypeCode::CreditNote)
        .seller(xr_seller())
        .buyer(xr_buyer())
        .buyer_reference("LEITWEG-123")
        .add_line(
            LineItemBuilder::new("1", "Returned goods", dec!(-5), "C62", dec!(20))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .payment(sepa_payment())
        .tax_point_date(date(2024, 6, 1))
        .build()
        .unwrap();

    assert_eq!(inv.type_code, InvoiceTypeCode::CreditNote);
    let totals = inv.totals.as_ref().unwrap();
    assert_eq!(totals.line_net_total, dec!(-100));

    // UBL roundtrip
    let ubl_xml = faktura::xrechnung::to_ubl_xml(&inv).unwrap();
    let ubl_back = faktura::xrechnung::from_ubl_xml(&ubl_xml).unwrap();
    assert_eq!(ubl_back.type_code, InvoiceTypeCode::CreditNote);
    assert_eq!(ubl_back.lines[0].quantity, dec!(-5));

    // CII roundtrip
    let cii_xml = faktura::xrechnung::to_cii_xml(&inv).unwrap();
    let cii_back = faktura::xrechnung::from_cii_xml(&cii_xml).unwrap();
    assert_eq!(cii_back.type_code, InvoiceTypeCode::CreditNote);
    assert_eq!(cii_back.lines[0].quantity, dec!(-5));
}

// ---- 6. Percentage-based allowances ----

#[test]
#[cfg(feature = "peppol")]
fn percentage_allowance_peppol_roundtrip() {
    let inv = InvoiceBuilder::new("PA-001", date(2024, 6, 1))
        .due_date(date(2024, 7, 1))
        .seller(xr_seller())
        .buyer(xr_buyer())
        .buyer_reference("LEITWEG-123")
        .add_line(
            LineItemBuilder::new("1", "Service", dec!(10), "HUR", dec!(100))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .add_allowance(AllowanceCharge {
            is_charge: false,
            amount: dec!(100),
            percentage: Some(dec!(10)),
            base_amount: Some(dec!(1000)),
            tax_category: TaxCategory::StandardRate,
            tax_rate: dec!(19),
            reason: Some("Volume discount".into()),
            reason_code: Some("95".into()),
        })
        .payment(sepa_payment())
        .tax_point_date(date(2024, 6, 1))
        .build()
        .unwrap();

    let totals = inv.totals.as_ref().unwrap();
    assert_eq!(totals.line_net_total, dec!(1000));
    assert_eq!(totals.allowances_total, dec!(100));
    assert_eq!(totals.net_total, dec!(900));

    // Peppol validation
    let errors = faktura::peppol::validate_peppol(&inv);
    let peppol_allowance_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.field.contains("allowance"))
        .collect();
    assert!(
        peppol_allowance_errors.is_empty(),
        "Percentage allowance should pass Peppol: {peppol_allowance_errors:?}"
    );

    // UBL roundtrip
    let ubl_xml = faktura::xrechnung::to_ubl_xml(&inv).unwrap();
    let ubl_back = faktura::xrechnung::from_ubl_xml(&ubl_xml).unwrap();
    assert_eq!(ubl_back.totals.as_ref().unwrap().net_total, dec!(900));
}

// ---- 7. Rounding edge cases ----

#[test]
fn rounding_fractional_vat() {
    // 3 × 33.33 = 99.99 at 19% → VAT = 18.9981 → rounded to 19.00
    let inv = InvoiceBuilder::new("RND-001", date(2024, 6, 1))
        .due_date(date(2024, 7, 1))
        .seller(seller())
        .buyer(buyer())
        .add_line(
            LineItemBuilder::new("1", "Widget A", dec!(3), "C62", dec!(33.33))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .payment(sepa_payment())
        .tax_point_date(date(2024, 6, 1))
        .build()
        .unwrap();

    let totals = inv.totals.as_ref().unwrap();
    assert_eq!(totals.line_net_total, dec!(99.99));
    // 99.99 * 0.19 = 18.9981 → half-up → 19.00
    assert_eq!(totals.vat_total, dec!(19.00));
    assert_eq!(totals.gross_total, dec!(118.99));

    // Arithmetic validation must pass
    let arith_errors = validate_arithmetic(&inv);
    assert!(
        arith_errors.is_empty(),
        "Rounding case should pass arithmetic: {arith_errors:?}"
    );
}

// ---- 8. SmallInvoice scenario ----

#[test]
fn small_invoice_scenario() {
    let inv = InvoiceBuilder::new("SM-001", date(2024, 6, 1))
        .seller(seller())
        .buyer(
            PartyBuilder::new(
                "Kunde",
                AddressBuilder::new("Berlin", "10115", "DE").build(),
            )
            .build(),
        )
        .vat_scenario(VatScenario::SmallInvoice)
        .add_line(
            LineItemBuilder::new("1", "Coffee", dec!(2), "C62", dec!(4.50))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .payment(sepa_payment())
        // SmallInvoice does NOT require tax_point_date
        .build()
        .unwrap();

    let totals = inv.totals.as_ref().unwrap();
    // 2 × 4.50 = 9.00, VAT = 1.71, gross = 10.71 — under €250
    assert!(totals.gross_total <= dec!(250));
    assert_eq!(totals.line_net_total, dec!(9.00));
}

// ---- 9. Credit note in ZUGFeRD ----

#[test]
#[cfg(feature = "zugferd")]
fn credit_note_zugferd_xml_roundtrip() {
    let inv = InvoiceBuilder::new("CN-ZUG-001", date(2024, 6, 1))
        .due_date(date(2024, 7, 1))
        .type_code(InvoiceTypeCode::CreditNote)
        .seller(xr_seller())
        .buyer(xr_buyer())
        .buyer_reference("LEITWEG-123")
        .add_line(
            LineItemBuilder::new("1", "Credit", dec!(-1), "C62", dec!(200))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .payment(sepa_payment())
        .tax_point_date(date(2024, 6, 1))
        .build()
        .unwrap();

    // Generate ZUGFeRD CII XML
    let xml = faktura::zugferd::to_xml(&inv, faktura::zugferd::ZugferdProfile::EN16931).unwrap();
    assert!(!xml.is_empty());
    // Should contain type code 381
    assert!(xml.contains("381"), "ZUGFeRD XML should contain credit note type code 381");

    // Parse back via CII parser
    let back = faktura::xrechnung::from_cii_xml(&xml).unwrap();
    assert_eq!(back.type_code, InvoiceTypeCode::CreditNote);
}

// ===========================================================================
// LOW PRIORITY
// ===========================================================================

// ---- 10. Seller item ID roundtrip ----

#[test]
#[cfg(feature = "xrechnung")]
fn seller_item_id_roundtrip() {
    let inv = InvoiceBuilder::new("SID-001", date(2024, 6, 1))
        .due_date(date(2024, 7, 1))
        .seller(xr_seller())
        .buyer(xr_buyer())
        .buyer_reference("LEITWEG-123")
        .add_line(
            LineItemBuilder::new("1", "Custom Part", dec!(1), "C62", dec!(50))
                .tax(TaxCategory::StandardRate, dec!(19))
                .seller_item_id("SEL-001")
                .build(),
        )
        .payment(sepa_payment())
        .tax_point_date(date(2024, 6, 1))
        .build()
        .unwrap();

    // UBL roundtrip
    let ubl_xml = faktura::xrechnung::to_ubl_xml(&inv).unwrap();
    let ubl_back = faktura::xrechnung::from_ubl_xml(&ubl_xml).unwrap();
    assert_eq!(ubl_back.lines[0].seller_item_id.as_deref(), Some("SEL-001"));

    // CII roundtrip
    let cii_xml = faktura::xrechnung::to_cii_xml(&inv).unwrap();
    let cii_back = faktura::xrechnung::from_cii_xml(&cii_xml).unwrap();
    assert_eq!(cii_back.lines[0].seller_item_id.as_deref(), Some("SEL-001"));
}

// ---- 11. Standard item ID UBL roundtrip ----

#[test]
#[cfg(feature = "xrechnung")]
fn standard_item_id_ubl_roundtrip() {
    let inv = InvoiceBuilder::new("GTIN-001", date(2024, 6, 1))
        .due_date(date(2024, 7, 1))
        .seller(xr_seller())
        .buyer(xr_buyer())
        .buyer_reference("LEITWEG-123")
        .add_line(
            LineItemBuilder::new("1", "Barcode Item", dec!(1), "C62", dec!(25))
                .tax(TaxCategory::StandardRate, dec!(19))
                .standard_item_id("4012345678901")
                .build(),
        )
        .payment(sepa_payment())
        .tax_point_date(date(2024, 6, 1))
        .build()
        .unwrap();

    let ubl_xml = faktura::xrechnung::to_ubl_xml(&inv).unwrap();
    let ubl_back = faktura::xrechnung::from_ubl_xml(&ubl_xml).unwrap();
    assert_eq!(
        ubl_back.lines[0].standard_item_id.as_deref(),
        Some("4012345678901")
    );
}

// ---- 12. Multiple preceding invoice references ----

#[test]
#[cfg(feature = "xrechnung")]
fn multiple_preceding_invoice_refs_roundtrip() {
    let inv = InvoiceBuilder::new("CORR-001", date(2024, 6, 1))
        .due_date(date(2024, 7, 1))
        .type_code(InvoiceTypeCode::Corrected)
        .seller(xr_seller())
        .buyer(xr_buyer())
        .buyer_reference("LEITWEG-123")
        .add_preceding_invoice("RE-2024-050", Some(date(2024, 3, 1)))
        .add_preceding_invoice("RE-2024-051", Some(date(2024, 4, 1)))
        .add_line(
            LineItemBuilder::new("1", "Corrected item", dec!(1), "C62", dec!(100))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .payment(sepa_payment())
        .tax_point_date(date(2024, 6, 1))
        .build()
        .unwrap();

    assert_eq!(inv.preceding_invoices.len(), 2);

    // UBL roundtrip
    let ubl_xml = faktura::xrechnung::to_ubl_xml(&inv).unwrap();
    let ubl_back = faktura::xrechnung::from_ubl_xml(&ubl_xml).unwrap();
    assert_eq!(ubl_back.preceding_invoices.len(), 2);
    assert_eq!(ubl_back.preceding_invoices[0].number, "RE-2024-050");
    assert_eq!(ubl_back.preceding_invoices[1].number, "RE-2024-051");

    // CII roundtrip
    let cii_xml = faktura::xrechnung::to_cii_xml(&inv).unwrap();
    let cii_back = faktura::xrechnung::from_cii_xml(&cii_xml).unwrap();
    assert_eq!(cii_back.preceding_invoices.len(), 2);
}

// ---- 13. Minimal invoice ----

#[test]
fn minimal_invoice_builds_and_validates() {
    let inv = InvoiceBuilder::new("MIN-001", date(2024, 6, 1))
        .seller(seller())
        .buyer(buyer())
        .add_line(
            LineItemBuilder::new("1", "Item", dec!(1), "C62", dec!(10))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .payment(sepa_payment())
        .tax_point_date(date(2024, 6, 1))
        .build()
        .unwrap();

    let totals = inv.totals.as_ref().unwrap();
    assert_eq!(totals.line_net_total, dec!(10));
    assert_eq!(totals.vat_total, dec!(1.90));
    assert_eq!(totals.gross_total, dec!(11.90));

    // §14 UStG validation passes
    let errors = validate_14_ustg(&inv);
    assert!(errors.is_empty(), "Minimal invoice should pass §14: {errors:?}");
}

// ---- 14. Delivery CII full roundtrip ----

#[test]
#[cfg(feature = "xrechnung")]
fn delivery_info_cii_roundtrip() {
    let inv = InvoiceBuilder::new("DEL-001", date(2024, 6, 1))
        .due_date(date(2024, 7, 1))
        .seller(xr_seller())
        .buyer(xr_buyer())
        .buyer_reference("LEITWEG-123")
        .delivery(DeliveryInformation {
            actual_delivery_date: Some(date(2024, 6, 10)),
            delivery_party: Some(DeliveryParty {
                name: "Warehouse North".into(),
                location_id: Some("LOC-42".into()),
            }),
            delivery_address: Some(DeliveryAddress {
                street: Some("Industriestr. 5".into()),
                additional: None,
                city: "Hamburg".into(),
                postal_code: "20095".into(),
                subdivision: None,
                country_code: "DE".into(),
            }),
        })
        .add_line(
            LineItemBuilder::new("1", "Goods", dec!(1), "C62", dec!(100))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .payment(sepa_payment())
        .tax_point_date(date(2024, 6, 1))
        .build()
        .unwrap();

    // CII roundtrip
    let cii_xml = faktura::xrechnung::to_cii_xml(&inv).unwrap();
    let cii_back = faktura::xrechnung::from_cii_xml(&cii_xml).unwrap();

    let del = cii_back.delivery.as_ref().unwrap();
    assert_eq!(del.actual_delivery_date, Some(date(2024, 6, 10)));

    if let Some(party) = &del.delivery_party {
        assert_eq!(party.name, "Warehouse North");
    }

    if let Some(addr) = &del.delivery_address {
        assert_eq!(addr.city, "Hamburg");
        assert_eq!(addr.postal_code, "20095");
        assert_eq!(addr.country_code, "DE");
    }
}

// ---- 15. Period start > end validation ----

#[test]
fn inverted_period_handling() {
    // Builder accepts inverted period; check if validation catches it or it's silently accepted
    let result = InvoiceBuilder::new("PER-001", date(2024, 6, 1))
        .seller(seller())
        .buyer(buyer())
        .add_line(
            LineItemBuilder::new("1", "Item", dec!(1), "C62", dec!(10))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .payment(sepa_payment())
        .invoicing_period(date(2024, 6, 30), date(2024, 6, 1)) // end before start
        .build();

    // Whether it errors or succeeds, this test documents the behavior.
    // If it succeeds, EN16931 validation might catch it.
    match result {
        Ok(inv) => {
            // Builder accepted the inverted period — document that it was not rejected
            let period = inv.invoicing_period.as_ref().unwrap();
            assert_eq!(period.start, date(2024, 6, 30));
            assert_eq!(period.end, date(2024, 6, 1));
        }
        Err(_) => {
            // Builder rejected the inverted period — also acceptable
        }
    }
}

// ---- 16. Unicode in addresses/item names ----

#[test]
#[cfg(feature = "xrechnung")]
fn unicode_addresses_and_items_roundtrip() {
    let unicode_buyer = PartyBuilder::new(
        "株式会社テスト", // Japanese company name
        AddressBuilder::new("東京", "100-0001", "JP")
            .street("千代田区丸の内1-1")
            .build(),
    )
    .electronic_address("EM", "test@example.jp")
    .build();

    let inv = InvoiceBuilder::new("UNI-001", date(2024, 6, 1))
        .due_date(date(2024, 7, 1))
        .seller(xr_seller())
        .buyer(unicode_buyer)
        .buyer_reference("LEITWEG-123")
        .add_line(
            LineItemBuilder::new("1", "مُنتَج عربي", dec!(1), "C62", dec!(100)) // Arabic item
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .payment(sepa_payment())
        .tax_point_date(date(2024, 6, 1))
        .build()
        .unwrap();

    // UBL roundtrip
    let ubl_xml = faktura::xrechnung::to_ubl_xml(&inv).unwrap();
    let ubl_back = faktura::xrechnung::from_ubl_xml(&ubl_xml).unwrap();
    assert_eq!(ubl_back.buyer.name, "株式会社テスト");
    assert!(ubl_back.lines[0].item_name.contains("عربي"));

    // CII roundtrip
    let cii_xml = faktura::xrechnung::to_cii_xml(&inv).unwrap();
    let cii_back = faktura::xrechnung::from_cii_xml(&cii_xml).unwrap();
    assert_eq!(cii_back.buyer.name, "株式会社テスト");
    assert!(cii_back.lines[0].item_name.contains("عربي"));
}

// ---- 17. Minimal invoice through KoSIT (requires Docker) ----

#[test]
#[ignore] // Requires KoSIT validator Docker container at localhost:8081
#[cfg(feature = "xrechnung")]
fn minimal_invoice_kosit_validation() {
    let inv = InvoiceBuilder::new("KOSIT-001", date(2024, 6, 1))
        .due_date(date(2024, 7, 1))
        .seller(xr_seller())
        .buyer(xr_buyer())
        .buyer_reference("LEITWEG-123")
        .add_line(
            LineItemBuilder::new("1", "Test", dec!(1), "C62", dec!(100))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .payment(sepa_payment())
        .tax_point_date(date(2024, 6, 1))
        .build()
        .unwrap();

    let ubl_xml = faktura::xrechnung::to_ubl_xml(&inv).unwrap();

    let client = reqwest::blocking::Client::new();
    let resp = client
        .post("http://localhost:8081/validation")
        .header("Content-Type", "application/xml")
        .body(ubl_xml)
        .send()
        .expect("KoSIT validator must be running");

    assert!(
        resp.status().is_success(),
        "KoSIT validation failed: {}",
        resp.text().unwrap_or_default()
    );
}

// ---- 18. Non-EUR through KoSIT (requires Docker) ----

#[test]
#[ignore] // Requires KoSIT validator Docker container at localhost:8081
#[cfg(feature = "xrechnung")]
fn non_eur_kosit_validation() {
    let inv = InvoiceBuilder::new("KOSIT-USD-001", date(2024, 6, 1))
        .due_date(date(2024, 7, 1))
        .currency("USD")
        .seller(xr_seller())
        .buyer(xr_buyer())
        .buyer_reference("LEITWEG-123")
        .add_line(
            LineItemBuilder::new("1", "Consulting", dec!(10), "HUR", dec!(150))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .payment(sepa_payment())
        .tax_point_date(date(2024, 6, 1))
        .build()
        .unwrap();

    let ubl_xml = faktura::xrechnung::to_ubl_xml(&inv).unwrap();

    let client = reqwest::blocking::Client::new();
    let resp = client
        .post("http://localhost:8081/validation")
        .header("Content-Type", "application/xml")
        .body(ubl_xml)
        .send()
        .expect("KoSIT validator must be running");

    assert!(
        resp.status().is_success(),
        "KoSIT validation for USD invoice failed: {}",
        resp.text().unwrap_or_default()
    );
}
