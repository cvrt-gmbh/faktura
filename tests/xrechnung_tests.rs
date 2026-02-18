#![cfg(feature = "xrechnung")]

use chrono::NaiveDate;
use faktura::core::*;
use faktura::xrechnung;
use rust_decimal_macros::dec;

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

/// Build a fully XRechnung-compliant domestic invoice.
fn xrechnung_invoice() -> Invoice {
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
            LineItemBuilder::new("1", "Softwareentwicklung", dec!(80), "HUR", dec!(120))
                .tax(TaxCategory::StandardRate, dec!(19))
                .description("React Frontend")
                .build(),
        )
        .add_line(
            LineItemBuilder::new("2", "Hosting", dec!(1), "C62", dec!(49.90))
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
        .expect("valid invoice")
}

// ---------------------------------------------------------------------------
// UBL Generation
// ---------------------------------------------------------------------------

#[test]
fn ubl_generation_produces_valid_xml() {
    let inv = xrechnung_invoice();
    let xml = xrechnung::to_ubl_xml(&inv).unwrap();

    assert!(xml.contains("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
    assert!(xml.contains("ubl:Invoice"));
    assert!(xml.contains(xrechnung::XRECHNUNG_CUSTOMIZATION_ID));
    assert!(xml.contains(xrechnung::PEPPOL_PROFILE_ID));
}

#[test]
fn ubl_contains_invoice_metadata() {
    let inv = xrechnung_invoice();
    let xml = xrechnung::to_ubl_xml(&inv).unwrap();

    assert!(xml.contains("<cbc:ID>RE-2024-001</cbc:ID>"));
    assert!(xml.contains("<cbc:IssueDate>2024-06-15</cbc:IssueDate>"));
    assert!(xml.contains("<cbc:DueDate>2024-07-15</cbc:DueDate>"));
    assert!(xml.contains("<cbc:InvoiceTypeCode>380</cbc:InvoiceTypeCode>"));
    assert!(xml.contains("<cbc:DocumentCurrencyCode>EUR</cbc:DocumentCurrencyCode>"));
    assert!(xml.contains("<cbc:BuyerReference>04011000-12345-03</cbc:BuyerReference>"));
}

#[test]
fn ubl_contains_seller_details() {
    let inv = xrechnung_invoice();
    let xml = xrechnung::to_ubl_xml(&inv).unwrap();

    assert!(xml.contains("<cbc:RegistrationName>ACME GmbH</cbc:RegistrationName>"));
    assert!(xml.contains("<cbc:CompanyID>DE123456789</cbc:CompanyID>"));
    assert!(xml.contains("<cbc:StreetName>Friedrichstraße 123</cbc:StreetName>"));
    assert!(xml.contains("<cbc:CityName>Berlin</cbc:CityName>"));
    assert!(xml.contains("<cbc:PostalZone>10115</cbc:PostalZone>"));
    assert!(xml.contains("<cbc:IdentificationCode>DE</cbc:IdentificationCode>"));
    assert!(xml.contains("<cbc:Name>Max Mustermann</cbc:Name>"));
    assert!(xml.contains("<cbc:Telephone>+49 30 12345</cbc:Telephone>"));
    assert!(xml.contains("<cbc:ElectronicMail>max@acme.de</cbc:ElectronicMail>"));
    assert!(xml.contains("schemeID=\"EM\""));
    assert!(xml.contains("seller@acme.de"));
}

#[test]
fn ubl_contains_totals() {
    let inv = xrechnung_invoice();
    let xml = xrechnung::to_ubl_xml(&inv).unwrap();

    // 80*120 + 49.90 = 9649.90
    assert!(xml.contains("9649.90"));
    // VAT: 9649.90 * 0.19 = 1833.481 → 1833.48
    assert!(xml.contains("1833.48"));
    // Gross: 11483.38
    assert!(xml.contains("11483.38"));
}

#[test]
fn ubl_contains_line_items() {
    let inv = xrechnung_invoice();
    let xml = xrechnung::to_ubl_xml(&inv).unwrap();

    assert!(xml.contains("cac:InvoiceLine"));
    assert!(xml.contains("Softwareentwicklung"));
    assert!(xml.contains("unitCode=\"HUR\""));
    assert!(xml.contains("React Frontend"));
    assert!(xml.contains("Hosting"));
}

#[test]
fn ubl_contains_payment_info() {
    let inv = xrechnung_invoice();
    let xml = xrechnung::to_ubl_xml(&inv).unwrap();

    assert!(xml.contains("<cbc:PaymentMeansCode>58</cbc:PaymentMeansCode>"));
    assert!(xml.contains("DE89370400440532013000")); // IBAN
    assert!(xml.contains("COBADEFFXXX")); // BIC
    assert!(xml.contains("Zahlbar innerhalb von 30 Tagen"));
}

// ---------------------------------------------------------------------------
// CII Generation
// ---------------------------------------------------------------------------

#[test]
fn cii_generation_produces_valid_xml() {
    let inv = xrechnung_invoice();
    let xml = xrechnung::to_cii_xml(&inv).unwrap();

    assert!(xml.contains("rsm:CrossIndustryInvoice"));
    assert!(xml.contains(xrechnung::XRECHNUNG_CUSTOMIZATION_ID));
    assert!(xml.contains(xrechnung::PEPPOL_PROFILE_ID));
}

#[test]
fn cii_uses_correct_date_format() {
    let inv = xrechnung_invoice();
    let xml = xrechnung::to_cii_xml(&inv).unwrap();

    // CII uses YYYYMMDD format with format="102"
    assert!(xml.contains("20240615"));
    assert!(xml.contains("format=\"102\""));
}

#[test]
fn cii_contains_seller_tax_registration() {
    let inv = xrechnung_invoice();
    let xml = xrechnung::to_cii_xml(&inv).unwrap();

    assert!(xml.contains("schemeID=\"VA\""));
    assert!(xml.contains("DE123456789"));
}

#[test]
fn cii_contains_monetary_summation() {
    let inv = xrechnung_invoice();
    let xml = xrechnung::to_cii_xml(&inv).unwrap();

    assert!(xml.contains("<ram:LineTotalAmount>9649.90</ram:LineTotalAmount>"));
    assert!(xml.contains("<ram:DuePayableAmount>11483.38</ram:DuePayableAmount>"));
}

// ---------------------------------------------------------------------------
// UBL Roundtrip (generate → parse → compare)
// ---------------------------------------------------------------------------

#[test]
fn ubl_roundtrip() {
    let original = xrechnung_invoice();
    let xml = xrechnung::to_ubl_xml(&original).unwrap();
    let parsed = xrechnung::from_ubl_xml(&xml).unwrap();

    assert_eq!(parsed.number, original.number);
    assert_eq!(parsed.issue_date, original.issue_date);
    assert_eq!(parsed.due_date, original.due_date);
    assert_eq!(parsed.type_code, original.type_code);
    assert_eq!(parsed.currency_code, original.currency_code);
    assert_eq!(parsed.buyer_reference, original.buyer_reference);

    // Seller
    assert_eq!(parsed.seller.name, original.seller.name);
    assert_eq!(parsed.seller.vat_id, original.seller.vat_id);
    assert_eq!(parsed.seller.address.city, original.seller.address.city);
    assert_eq!(
        parsed.seller.address.postal_code,
        original.seller.address.postal_code
    );
    assert_eq!(
        parsed.seller.address.country_code,
        original.seller.address.country_code
    );

    // Buyer
    assert_eq!(parsed.buyer.name, original.buyer.name);
    assert_eq!(parsed.buyer.address.city, original.buyer.address.city);

    // Lines
    assert_eq!(parsed.lines.len(), original.lines.len());
    assert_eq!(parsed.lines[0].item_name, "Softwareentwicklung");
    assert_eq!(parsed.lines[0].quantity, dec!(80));
    assert_eq!(parsed.lines[0].unit_price, dec!(120));
    assert_eq!(parsed.lines[1].item_name, "Hosting");

    // Totals
    let orig_totals = original.totals.as_ref().unwrap();
    let parsed_totals = parsed.totals.as_ref().unwrap();
    assert_eq!(parsed_totals.line_net_total, orig_totals.line_net_total);
    assert_eq!(parsed_totals.vat_total, orig_totals.vat_total);
    assert_eq!(parsed_totals.gross_total, orig_totals.gross_total);
    assert_eq!(parsed_totals.amount_due, orig_totals.amount_due);

    // Payment
    let orig_payment = original.payment.as_ref().unwrap();
    let parsed_payment = parsed.payment.as_ref().unwrap();
    assert_eq!(parsed_payment.means_code, orig_payment.means_code);
    let orig_ct = orig_payment.credit_transfer.as_ref().unwrap();
    let parsed_ct = parsed_payment.credit_transfer.as_ref().unwrap();
    assert_eq!(parsed_ct.iban, orig_ct.iban);
    assert_eq!(parsed_ct.bic, orig_ct.bic);
}

// ---------------------------------------------------------------------------
// CII Roundtrip
// ---------------------------------------------------------------------------

#[test]
fn cii_roundtrip() {
    let original = xrechnung_invoice();
    let xml = xrechnung::to_cii_xml(&original).unwrap();
    let parsed = xrechnung::from_cii_xml(&xml).unwrap();

    assert_eq!(parsed.number, original.number);
    assert_eq!(parsed.issue_date, original.issue_date);
    assert_eq!(parsed.type_code, original.type_code);
    assert_eq!(parsed.currency_code, original.currency_code);
    assert_eq!(parsed.buyer_reference, original.buyer_reference);

    assert_eq!(parsed.seller.name, original.seller.name);
    assert_eq!(parsed.seller.vat_id, original.seller.vat_id);
    assert_eq!(parsed.buyer.name, original.buyer.name);

    assert_eq!(parsed.lines.len(), 2);
    assert_eq!(parsed.lines[0].item_name, "Softwareentwicklung");
    assert_eq!(parsed.lines[0].quantity, dec!(80));

    let parsed_totals = parsed.totals.as_ref().unwrap();
    let orig_totals = original.totals.as_ref().unwrap();
    assert_eq!(parsed_totals.line_net_total, orig_totals.line_net_total);
    assert_eq!(parsed_totals.vat_total, orig_totals.vat_total);
    assert_eq!(parsed_totals.amount_due, orig_totals.amount_due);
}

// ---------------------------------------------------------------------------
// XRechnung Validation (BR-DE rules)
// ---------------------------------------------------------------------------

#[test]
fn xrechnung_valid_invoice_passes() {
    let inv = xrechnung_invoice();
    let errors = xrechnung::validate_xrechnung(&inv);
    assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
}

#[test]
fn xrechnung_missing_buyer_reference() {
    let inv = InvoiceBuilder::new("RE-001", date(2024, 6, 15))
        .tax_point_date(date(2024, 6, 15))
        // No buyer_reference
        .seller(
            PartyBuilder::new(
                "S GmbH",
                AddressBuilder::new("Berlin", "10115", "DE").build(),
            )
            .vat_id("DE123456789")
            .electronic_address("EM", "s@s.de")
            .contact(Some("A".into()), Some("1".into()), Some("a@b.de".into()))
            .build(),
        )
        .buyer(
            PartyBuilder::new(
                "B AG",
                AddressBuilder::new("München", "80331", "DE").build(),
            )
            .electronic_address("EM", "b@b.de")
            .build(),
        )
        .add_line(
            LineItemBuilder::new("1", "Test", dec!(1), "C62", dec!(100))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .payment(PaymentInstructions {
            means_code: PaymentMeansCode::SepaCreditTransfer,
            means_text: None,
            remittance_info: None,
            credit_transfer: Some(CreditTransfer {
                iban: "DE89370400440532013000".into(),
                bic: None,
                account_name: None,
            }),
        })
        .build()
        .unwrap();

    let errors = xrechnung::validate_xrechnung(&inv);
    assert!(errors.iter().any(|e| e.rule.as_deref() == Some("BR-DE-15")));
}

#[test]
fn xrechnung_missing_seller_contact() {
    let inv = InvoiceBuilder::new("RE-001", date(2024, 6, 15))
        .tax_point_date(date(2024, 6, 15))
        .buyer_reference("04011000-12345-03")
        .seller(
            PartyBuilder::new(
                "S GmbH",
                AddressBuilder::new("Berlin", "10115", "DE").build(),
            )
            .vat_id("DE123456789")
            .electronic_address("EM", "s@s.de")
            // No contact
            .build(),
        )
        .buyer(
            PartyBuilder::new(
                "B AG",
                AddressBuilder::new("München", "80331", "DE").build(),
            )
            .electronic_address("EM", "b@b.de")
            .build(),
        )
        .add_line(
            LineItemBuilder::new("1", "Test", dec!(1), "C62", dec!(100))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .payment(PaymentInstructions {
            means_code: PaymentMeansCode::SepaCreditTransfer,
            means_text: None,
            remittance_info: None,
            credit_transfer: Some(CreditTransfer {
                iban: "DE89370400440532013000".into(),
                bic: None,
                account_name: None,
            }),
        })
        .build()
        .unwrap();

    let errors = xrechnung::validate_xrechnung(&inv);
    assert!(errors.iter().any(|e| e.rule.as_deref() == Some("BR-DE-2")));
}

#[test]
fn xrechnung_missing_electronic_addresses() {
    let inv = InvoiceBuilder::new("RE-001", date(2024, 6, 15))
        .tax_point_date(date(2024, 6, 15))
        .buyer_reference("04011000-12345-03")
        .seller(
            PartyBuilder::new(
                "S GmbH",
                AddressBuilder::new("Berlin", "10115", "DE").build(),
            )
            .vat_id("DE123456789")
            // No electronic address
            .contact(Some("A".into()), Some("1".into()), Some("a@b.de".into()))
            .build(),
        )
        .buyer(
            PartyBuilder::new(
                "B AG",
                AddressBuilder::new("München", "80331", "DE").build(),
            )
            // No electronic address
            .build(),
        )
        .add_line(
            LineItemBuilder::new("1", "Test", dec!(1), "C62", dec!(100))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .payment(PaymentInstructions {
            means_code: PaymentMeansCode::SepaCreditTransfer,
            means_text: None,
            remittance_info: None,
            credit_transfer: Some(CreditTransfer {
                iban: "DE89370400440532013000".into(),
                bic: None,
                account_name: None,
            }),
        })
        .build()
        .unwrap();

    let errors = xrechnung::validate_xrechnung(&inv);
    assert!(errors.iter().any(|e| e.rule.as_deref() == Some("BR-DE-26")));
    assert!(errors.iter().any(|e| e.rule.as_deref() == Some("BR-DE-28")));
}

#[test]
fn xrechnung_missing_payment() {
    let inv = InvoiceBuilder::new("RE-001", date(2024, 6, 15))
        .tax_point_date(date(2024, 6, 15))
        .buyer_reference("04011000-12345-03")
        .seller(
            PartyBuilder::new(
                "S GmbH",
                AddressBuilder::new("Berlin", "10115", "DE").build(),
            )
            .vat_id("DE123456789")
            .electronic_address("EM", "s@s.de")
            .contact(Some("A".into()), Some("1".into()), Some("a@b.de".into()))
            .build(),
        )
        .buyer(
            PartyBuilder::new(
                "B AG",
                AddressBuilder::new("München", "80331", "DE").build(),
            )
            .electronic_address("EM", "b@b.de")
            .build(),
        )
        .add_line(
            LineItemBuilder::new("1", "Test", dec!(1), "C62", dec!(100))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        // No payment
        .build()
        .unwrap();

    let errors = xrechnung::validate_xrechnung(&inv);
    assert!(errors.iter().any(|e| e.rule.as_deref() == Some("BR-DE-1")));
}

// ---------------------------------------------------------------------------
// Kleinunternehmer XRechnung
// ---------------------------------------------------------------------------

#[test]
fn ubl_kleinunternehmer_roundtrip() {
    let inv = InvoiceBuilder::new("RE-2024-010", date(2024, 6, 15))
        .vat_scenario(VatScenario::Kleinunternehmer)
        .tax_point_date(date(2024, 6, 15))
        .buyer_reference("04011000-99999-01")
        .note("Kein Ausweis von Umsatzsteuer, da Kleinunternehmer gemäß §19 UStG")
        .seller(
            PartyBuilder::new(
                "Freelancer",
                AddressBuilder::new("Köln", "50667", "DE").build(),
            )
            .tax_number("214/5678/0001")
            .electronic_address("EM", "free@lancer.de")
            .contact(
                Some("F".into()),
                Some("+49 221 0".into()),
                Some("f@l.de".into()),
            )
            .build(),
        )
        .buyer(
            PartyBuilder::new("Amt", AddressBuilder::new("Bonn", "53111", "DE").build())
                .electronic_address("EM", "amt@bonn.de")
                .build(),
        )
        .add_line(
            LineItemBuilder::new("1", "Webdesign", dec!(1), "C62", dec!(2500))
                .tax(TaxCategory::NotSubjectToVat, dec!(0))
                .build(),
        )
        .payment(PaymentInstructions {
            means_code: PaymentMeansCode::SepaCreditTransfer,
            means_text: None,
            remittance_info: None,
            credit_transfer: Some(CreditTransfer {
                iban: "DE89370400440532013000".into(),
                bic: None,
                account_name: None,
            }),
        })
        .build()
        .unwrap();

    let xml = xrechnung::to_ubl_xml(&inv).unwrap();
    assert!(xml.contains("TaxExemptionReason"));

    let parsed = xrechnung::from_ubl_xml(&xml).unwrap();
    assert_eq!(parsed.number, "RE-2024-010");
    assert_eq!(parsed.totals.as_ref().unwrap().vat_total, dec!(0));
    assert_eq!(parsed.totals.as_ref().unwrap().gross_total, dec!(2500));

    // Tax number via FC scheme
    assert!(xml.contains("<cbc:ID>FC</cbc:ID>"));
    assert!(xml.contains("214/5678/0001"));
}

// ---------------------------------------------------------------------------
// Credit Note (type code 381) end-to-end
// ---------------------------------------------------------------------------

fn credit_note_invoice() -> Invoice {
    InvoiceBuilder::new("GS-2024-001", date(2024, 6, 15))
        .type_code(InvoiceTypeCode::CreditNote)
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
            LineItemBuilder::new("1", "Gutschrift Beratung", dec!(5), "HUR", dec!(120))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .payment(PaymentInstructions {
            means_code: PaymentMeansCode::SepaCreditTransfer,
            means_text: None,
            remittance_info: Some("GS-2024-001".into()),
            credit_transfer: Some(CreditTransfer {
                iban: "DE89370400440532013000".into(),
                bic: Some("COBADEFFXXX".into()),
                account_name: Some("ACME GmbH".into()),
            }),
        })
        .payment_terms("Gutschrift wird innerhalb von 14 Tagen erstattet")
        .build()
        .expect("valid credit note")
}

#[test]
fn credit_note_ubl_generation() {
    let inv = credit_note_invoice();
    let xml = xrechnung::to_ubl_xml(&inv).unwrap();

    // Must use CreditNote root element, not Invoice
    assert!(xml.contains("ubl:CreditNote"), "should use CreditNote root");
    assert!(
        !xml.contains("ubl:Invoice"),
        "should not contain Invoice root"
    );
    assert!(
        xml.contains("<cbc:CreditNoteTypeCode>381</cbc:CreditNoteTypeCode>")
            || xml.contains("<cbc:InvoiceTypeCode>381</cbc:InvoiceTypeCode>"),
        "should contain type code 381"
    );
    // Credit notes use CreditedQuantity
    assert!(xml.contains("CreditedQuantity") || xml.contains("InvoicedQuantity"));
    assert!(xml.contains("CreditNoteLine") || xml.contains("InvoiceLine"));
}

#[test]
fn credit_note_ubl_roundtrip() {
    let original = credit_note_invoice();
    let xml = xrechnung::to_ubl_xml(&original).unwrap();
    let parsed = xrechnung::from_ubl_xml(&xml).unwrap();

    assert_eq!(parsed.type_code, InvoiceTypeCode::CreditNote);
    assert_eq!(parsed.type_code.code(), 381);
    assert_eq!(parsed.number, "GS-2024-001");
    assert_eq!(parsed.lines.len(), 1);
    assert_eq!(parsed.lines[0].quantity, dec!(5));
    assert_eq!(parsed.lines[0].unit_price, dec!(120));
    assert_eq!(parsed.seller.name, "ACME GmbH");
}

#[test]
fn credit_note_cii_generation() {
    let inv = credit_note_invoice();
    let xml = xrechnung::to_cii_xml(&inv).unwrap();

    // CII always uses CrossIndustryInvoice root
    assert!(xml.contains("CrossIndustryInvoice"));
    assert!(xml.contains("<ram:TypeCode>381</ram:TypeCode>"));
}

#[test]
fn credit_note_cii_roundtrip() {
    let original = credit_note_invoice();
    let xml = xrechnung::to_cii_xml(&original).unwrap();
    let parsed = xrechnung::from_cii_xml(&xml).unwrap();

    assert_eq!(parsed.type_code, InvoiceTypeCode::CreditNote);
    assert_eq!(parsed.type_code.code(), 381);
    assert_eq!(parsed.number, "GS-2024-001");
    assert_eq!(parsed.lines.len(), 1);
    assert_eq!(parsed.lines[0].quantity, dec!(5));
}

#[test]
fn credit_note_xrechnung_valid() {
    let inv = credit_note_invoice();
    let errors = xrechnung::validate_xrechnung(&inv);
    assert!(
        errors.is_empty(),
        "credit note should pass XRechnung validation, got: {:?}",
        errors
    );
}

// ---------------------------------------------------------------------------
// Document-level allowances/charges (BG-20, BG-21)
// ---------------------------------------------------------------------------

fn invoice_with_allowance_and_charge() -> Invoice {
    InvoiceBuilder::new("RE-2024-020", date(2024, 6, 15))
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
            LineItemBuilder::new("1", "Beratung", dec!(10), "HUR", dec!(100))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .add_allowance(AllowanceCharge {
            is_charge: false,
            amount: dec!(50),
            percentage: None,
            base_amount: None,
            tax_category: TaxCategory::StandardRate,
            tax_rate: dec!(19),
            reason: Some("Treuerabatt".into()),
            reason_code: Some("95".into()),
        })
        .add_charge(AllowanceCharge {
            is_charge: true,
            amount: dec!(25),
            percentage: None,
            base_amount: None,
            tax_category: TaxCategory::StandardRate,
            tax_rate: dec!(19),
            reason: Some("Verpackung".into()),
            reason_code: Some("ABL".into()),
        })
        .payment(PaymentInstructions {
            means_code: PaymentMeansCode::SepaCreditTransfer,
            means_text: None,
            remittance_info: Some("RE-2024-020".into()),
            credit_transfer: Some(CreditTransfer {
                iban: "DE89370400440532013000".into(),
                bic: Some("COBADEFFXXX".into()),
                account_name: Some("ACME GmbH".into()),
            }),
        })
        .payment_terms("Zahlbar innerhalb von 30 Tagen")
        .build()
        .expect("valid invoice with allowance and charge")
}

#[test]
fn allowance_charge_totals_correct() {
    let inv = invoice_with_allowance_and_charge();
    let totals = inv.totals.as_ref().unwrap();

    // Line total: 10 * 100 = 1000
    assert_eq!(totals.line_net_total, dec!(1000));
    // Allowances: 50
    assert_eq!(totals.allowances_total, dec!(50));
    // Charges: 25
    assert_eq!(totals.charges_total, dec!(25));
    // Net total: 1000 - 50 + 25 = 975
    assert_eq!(totals.net_total, dec!(975));
    // VAT: 975 * 0.19 = 185.25
    assert_eq!(totals.vat_total, dec!(185.25));
    // Gross: 975 + 185.25 = 1160.25
    assert_eq!(totals.gross_total, dec!(1160.25));
}

#[test]
fn allowance_charge_ubl_generation() {
    let inv = invoice_with_allowance_and_charge();
    let xml = xrechnung::to_ubl_xml(&inv).unwrap();

    assert!(xml.contains("cac:AllowanceCharge"));
    assert!(xml.contains("<cbc:ChargeIndicator>false</cbc:ChargeIndicator>"));
    assert!(xml.contains("<cbc:ChargeIndicator>true</cbc:ChargeIndicator>"));
    assert!(xml.contains("Treuerabatt"));
    assert!(xml.contains("Verpackung"));
    assert!(xml.contains("AllowanceTotalAmount"));
    assert!(xml.contains("ChargeTotalAmount"));
}

#[test]
fn allowance_charge_ubl_roundtrip() {
    let original = invoice_with_allowance_and_charge();
    let xml = xrechnung::to_ubl_xml(&original).unwrap();
    let parsed = xrechnung::from_ubl_xml(&xml).unwrap();

    assert_eq!(parsed.number, "RE-2024-020");
    let totals = parsed.totals.as_ref().unwrap();
    assert_eq!(totals.allowances_total, dec!(50));
    assert_eq!(totals.charges_total, dec!(25));
    assert_eq!(totals.net_total, dec!(975));
    assert_eq!(totals.amount_due, dec!(1160.25));
}

#[test]
fn allowance_charge_cii_generation() {
    let inv = invoice_with_allowance_and_charge();
    let xml = xrechnung::to_cii_xml(&inv).unwrap();

    assert!(xml.contains("SpecifiedTradeAllowanceCharge"));
    assert!(xml.contains("Treuerabatt"));
    assert!(xml.contains("Verpackung"));
    assert!(xml.contains("AllowanceTotalAmount"));
    assert!(xml.contains("ChargeTotalAmount"));
}

#[test]
fn allowance_charge_cii_roundtrip() {
    let original = invoice_with_allowance_and_charge();
    let xml = xrechnung::to_cii_xml(&original).unwrap();
    let parsed = xrechnung::from_cii_xml(&xml).unwrap();

    assert_eq!(parsed.number, "RE-2024-020");
    let totals = parsed.totals.as_ref().unwrap();
    assert_eq!(totals.allowances_total, dec!(50));
    assert_eq!(totals.charges_total, dec!(25));
    assert_eq!(totals.net_total, dec!(975));
    assert_eq!(totals.amount_due, dec!(1160.25));
}

#[test]
fn allowance_charge_xrechnung_valid() {
    let inv = invoice_with_allowance_and_charge();
    let errors = xrechnung::validate_xrechnung(&inv);
    assert!(
        errors.is_empty(),
        "invoice with allowances/charges should pass XRechnung validation, got: {:?}",
        errors
    );
}

// ---------------------------------------------------------------------------
// Malformed XML input — from_ubl_xml
// ---------------------------------------------------------------------------

#[test]
fn ubl_parse_empty_string() {
    let result = xrechnung::from_ubl_xml("");
    assert!(result.is_err());
}

#[test]
fn ubl_parse_not_xml() {
    let result = xrechnung::from_ubl_xml("this is not xml at all");
    assert!(result.is_err());
}

#[test]
fn ubl_parse_truncated_xml() {
    let result = xrechnung::from_ubl_xml("<?xml version=\"1.0\"?><Invoice><cbc:ID>123</cbc:ID>");
    assert!(result.is_err());
}

#[test]
fn ubl_parse_wrong_root_element() {
    let result = xrechnung::from_ubl_xml("<?xml version=\"1.0\"?><Catalog></Catalog>");
    assert!(result.is_err());
}

#[test]
fn ubl_parse_missing_required_fields() {
    // Valid XML structure but missing all business data
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<ubl:Invoice xmlns:ubl="urn:oasis:names:specification:ubl:schema:module:invoice:2"
             xmlns:cbc="urn:oasis:names:specification:ubl:schema:module:commonbasiccomponents-2"
             xmlns:cac="urn:oasis:names:specification:ubl:schema:module:commonaggregatecomponents-2">
</ubl:Invoice>"#;
    let result = xrechnung::from_ubl_xml(xml);
    // Should either error or produce an invoice with empty required fields
    // The important thing is it doesn't panic
    let _ = result;
}

// ---------------------------------------------------------------------------
// Malformed XML input — from_cii_xml
// ---------------------------------------------------------------------------

#[test]
fn cii_parse_empty_string() {
    let result = xrechnung::from_cii_xml("");
    assert!(result.is_err());
}

#[test]
fn cii_parse_not_xml() {
    let result = xrechnung::from_cii_xml("{\"type\": \"invoice\"}");
    assert!(result.is_err());
}

#[test]
fn cii_parse_truncated_xml() {
    let result =
        xrechnung::from_cii_xml("<?xml version=\"1.0\"?><rsm:CrossIndustryInvoice><rsm:Exch");
    assert!(result.is_err());
}

#[test]
fn cii_parse_ubl_xml_as_cii() {
    // Feed UBL XML to the CII parser — should not panic
    let inv = xrechnung_invoice();
    let ubl_xml = xrechnung::to_ubl_xml(&inv).unwrap();
    let result = xrechnung::from_cii_xml(&ubl_xml);
    // May succeed with empty/wrong data or error — must not panic
    let _ = result;
}

#[test]
fn ubl_parse_cii_xml_as_ubl() {
    // Feed CII XML to the UBL parser — should not panic
    let inv = xrechnung_invoice();
    let cii_xml = xrechnung::to_cii_xml(&inv).unwrap();
    let result = xrechnung::from_ubl_xml(&cii_xml);
    // May succeed with empty/wrong data or error — must not panic
    let _ = result;
}

// ===========================================================================
// Feature 1: Item Attributes (BT-160/BT-161)
// ===========================================================================

fn invoice_with_item_attributes() -> Invoice {
    InvoiceBuilder::new("RE-2024-030", date(2024, 6, 15))
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
            LineItemBuilder::new("1", "Laptop", dec!(2), "C62", dec!(999.99))
                .tax(TaxCategory::StandardRate, dec!(19))
                .add_attribute("Color", "Silver")
                .add_attribute("RAM", "16GB")
                .build(),
        )
        .payment(PaymentInstructions {
            means_code: PaymentMeansCode::SepaCreditTransfer,
            means_text: None,
            remittance_info: Some("RE-2024-030".into()),
            credit_transfer: Some(CreditTransfer {
                iban: "DE89370400440532013000".into(),
                bic: Some("COBADEFFXXX".into()),
                account_name: Some("ACME GmbH".into()),
            }),
        })
        .build()
        .expect("valid invoice with item attributes")
}

#[test]
fn item_attributes_ubl_generation() {
    let inv = invoice_with_item_attributes();
    let xml = xrechnung::to_ubl_xml(&inv).unwrap();

    assert!(xml.contains("cac:AdditionalItemProperty"));
    assert!(xml.contains("<cbc:Name>Color</cbc:Name>"));
    assert!(xml.contains("<cbc:Value>Silver</cbc:Value>"));
    assert!(xml.contains("<cbc:Name>RAM</cbc:Name>"));
    assert!(xml.contains("<cbc:Value>16GB</cbc:Value>"));
}

#[test]
fn item_attributes_ubl_roundtrip() {
    let original = invoice_with_item_attributes();
    let xml = xrechnung::to_ubl_xml(&original).unwrap();
    let parsed = xrechnung::from_ubl_xml(&xml).unwrap();

    assert_eq!(parsed.lines[0].attributes.len(), 2);
    assert_eq!(parsed.lines[0].attributes[0].name, "Color");
    assert_eq!(parsed.lines[0].attributes[0].value, "Silver");
    assert_eq!(parsed.lines[0].attributes[1].name, "RAM");
    assert_eq!(parsed.lines[0].attributes[1].value, "16GB");
}

#[test]
fn item_attributes_cii_generation() {
    let inv = invoice_with_item_attributes();
    let xml = xrechnung::to_cii_xml(&inv).unwrap();

    assert!(xml.contains("ram:ApplicableProductCharacteristic"));
    assert!(xml.contains("<ram:Description>Color</ram:Description>"));
    assert!(xml.contains("<ram:Value>Silver</ram:Value>"));
    assert!(xml.contains("<ram:Description>RAM</ram:Description>"));
    assert!(xml.contains("<ram:Value>16GB</ram:Value>"));
}

#[test]
fn item_attributes_cii_roundtrip() {
    let original = invoice_with_item_attributes();
    let xml = xrechnung::to_cii_xml(&original).unwrap();
    let parsed = xrechnung::from_cii_xml(&xml).unwrap();

    assert_eq!(parsed.lines[0].attributes.len(), 2);
    assert_eq!(parsed.lines[0].attributes[0].name, "Color");
    assert_eq!(parsed.lines[0].attributes[0].value, "Silver");
    assert_eq!(parsed.lines[0].attributes[1].name, "RAM");
    assert_eq!(parsed.lines[0].attributes[1].value, "16GB");
}

// ===========================================================================
// Feature 2: Line-Level Invoicing Period (BG-26)
// ===========================================================================

fn invoice_with_line_period() -> Invoice {
    InvoiceBuilder::new("RE-2024-031", date(2024, 6, 15))
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
            LineItemBuilder::new("1", "Hosting Monat Juni", dec!(1), "C62", dec!(49.90))
                .tax(TaxCategory::StandardRate, dec!(19))
                .invoicing_period(date(2024, 6, 1), date(2024, 6, 30))
                .build(),
        )
        .payment(PaymentInstructions {
            means_code: PaymentMeansCode::SepaCreditTransfer,
            means_text: None,
            remittance_info: None,
            credit_transfer: Some(CreditTransfer {
                iban: "DE89370400440532013000".into(),
                bic: None,
                account_name: None,
            }),
        })
        .build()
        .expect("valid invoice with line period")
}

#[test]
fn line_period_ubl_generation() {
    let inv = invoice_with_line_period();
    let xml = xrechnung::to_ubl_xml(&inv).unwrap();

    // Line-level InvoicePeriod inside InvoiceLine
    assert!(xml.contains("<cbc:StartDate>2024-06-01</cbc:StartDate>"));
    assert!(xml.contains("<cbc:EndDate>2024-06-30</cbc:EndDate>"));
}

#[test]
fn line_period_ubl_roundtrip() {
    let original = invoice_with_line_period();
    let xml = xrechnung::to_ubl_xml(&original).unwrap();
    let parsed = xrechnung::from_ubl_xml(&xml).unwrap();

    let period = parsed.lines[0].invoicing_period.as_ref().unwrap();
    assert_eq!(period.start, date(2024, 6, 1));
    assert_eq!(period.end, date(2024, 6, 30));
}

#[test]
fn line_period_cii_generation() {
    let inv = invoice_with_line_period();
    let xml = xrechnung::to_cii_xml(&inv).unwrap();

    assert!(xml.contains("ram:BillingSpecifiedPeriod"));
    assert!(xml.contains("20240601"));
    assert!(xml.contains("20240630"));
}

#[test]
fn line_period_cii_roundtrip() {
    let original = invoice_with_line_period();
    let xml = xrechnung::to_cii_xml(&original).unwrap();
    let parsed = xrechnung::from_cii_xml(&xml).unwrap();

    let period = parsed.lines[0].invoicing_period.as_ref().unwrap();
    assert_eq!(period.start, date(2024, 6, 1));
    assert_eq!(period.end, date(2024, 6, 30));
}

// ===========================================================================
// Feature 3: Preceding Invoice Reference (BT-25/BT-26)
// ===========================================================================

fn invoice_with_preceding_reference() -> Invoice {
    InvoiceBuilder::new("GS-2024-010", date(2024, 7, 1))
        .type_code(InvoiceTypeCode::CreditNote)
        .due_date(date(2024, 7, 31))
        .tax_point_date(date(2024, 7, 1))
        .buyer_reference("04011000-12345-03")
        .add_preceding_invoice("RE-2024-001", Some(date(2024, 6, 15)))
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
            LineItemBuilder::new("1", "Gutschrift Beratung", dec!(2), "HUR", dec!(120))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .payment(PaymentInstructions {
            means_code: PaymentMeansCode::SepaCreditTransfer,
            means_text: None,
            remittance_info: None,
            credit_transfer: Some(CreditTransfer {
                iban: "DE89370400440532013000".into(),
                bic: None,
                account_name: None,
            }),
        })
        .build()
        .expect("valid credit note with preceding invoice")
}

#[test]
fn preceding_invoice_ubl_generation() {
    let inv = invoice_with_preceding_reference();
    let xml = xrechnung::to_ubl_xml(&inv).unwrap();

    assert!(xml.contains("cac:BillingReference"));
    assert!(xml.contains("cac:InvoiceDocumentReference"));
    assert!(xml.contains("<cbc:ID>RE-2024-001</cbc:ID>"));
    assert!(xml.contains("<cbc:IssueDate>2024-06-15</cbc:IssueDate>"));
}

#[test]
fn preceding_invoice_ubl_roundtrip() {
    let original = invoice_with_preceding_reference();
    let xml = xrechnung::to_ubl_xml(&original).unwrap();
    let parsed = xrechnung::from_ubl_xml(&xml).unwrap();

    assert_eq!(parsed.preceding_invoices.len(), 1);
    assert_eq!(parsed.preceding_invoices[0].number, "RE-2024-001");
    assert_eq!(
        parsed.preceding_invoices[0].issue_date,
        Some(date(2024, 6, 15))
    );
}

#[test]
fn preceding_invoice_cii_generation() {
    let inv = invoice_with_preceding_reference();
    let xml = xrechnung::to_cii_xml(&inv).unwrap();

    assert!(xml.contains("ram:InvoiceReferencedDocument"));
    assert!(xml.contains("<ram:IssuerAssignedID>RE-2024-001</ram:IssuerAssignedID>"));
    assert!(xml.contains("20240615")); // CII date format
}

#[test]
fn preceding_invoice_cii_roundtrip() {
    let original = invoice_with_preceding_reference();
    let xml = xrechnung::to_cii_xml(&original).unwrap();
    let parsed = xrechnung::from_cii_xml(&xml).unwrap();

    assert_eq!(parsed.preceding_invoices.len(), 1);
    assert_eq!(parsed.preceding_invoices[0].number, "RE-2024-001");
    assert_eq!(
        parsed.preceding_invoices[0].issue_date,
        Some(date(2024, 6, 15))
    );
}

// ===========================================================================
// Feature 4: Tax Currency (BT-6 / BT-111)
// ===========================================================================

fn invoice_with_tax_currency() -> Invoice {
    InvoiceBuilder::new("RE-2024-032", date(2024, 6, 15))
        .due_date(date(2024, 7, 15))
        .tax_point_date(date(2024, 6, 15))
        .buyer_reference("04011000-12345-03")
        .currency("EUR")
        .tax_currency("GBP", dec!(158.70))
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
            LineItemBuilder::new("1", "Beratung", dec!(10), "HUR", dec!(100))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .payment(PaymentInstructions {
            means_code: PaymentMeansCode::SepaCreditTransfer,
            means_text: None,
            remittance_info: None,
            credit_transfer: Some(CreditTransfer {
                iban: "DE89370400440532013000".into(),
                bic: None,
                account_name: None,
            }),
        })
        .build()
        .expect("valid invoice with tax currency")
}

#[test]
fn tax_currency_ubl_generation() {
    let inv = invoice_with_tax_currency();
    let xml = xrechnung::to_ubl_xml(&inv).unwrap();

    assert!(xml.contains("<cbc:TaxCurrencyCode>GBP</cbc:TaxCurrencyCode>"));
    // Should have two TaxTotal elements
    assert!(xml.contains("currencyID=\"EUR\""));
    assert!(xml.contains("currencyID=\"GBP\""));
    assert!(xml.contains("158.70"));
}

#[test]
fn tax_currency_ubl_roundtrip() {
    let original = invoice_with_tax_currency();
    let xml = xrechnung::to_ubl_xml(&original).unwrap();
    let parsed = xrechnung::from_ubl_xml(&xml).unwrap();

    assert_eq!(parsed.tax_currency_code, Some("GBP".to_string()));
    let totals = parsed.totals.as_ref().unwrap();
    assert_eq!(totals.vat_total_in_tax_currency, Some(dec!(158.70)));
    assert_eq!(totals.vat_total, dec!(190)); // 1000 * 19%
}

#[test]
fn tax_currency_cii_generation() {
    let inv = invoice_with_tax_currency();
    let xml = xrechnung::to_cii_xml(&inv).unwrap();

    assert!(xml.contains("<ram:TaxCurrencyCode>GBP</ram:TaxCurrencyCode>"));
    assert!(xml.contains("currencyID=\"GBP\""));
    assert!(xml.contains("158.70"));
}

#[test]
fn tax_currency_cii_roundtrip() {
    let original = invoice_with_tax_currency();
    let xml = xrechnung::to_cii_xml(&original).unwrap();
    let parsed = xrechnung::from_cii_xml(&xml).unwrap();

    assert_eq!(parsed.tax_currency_code, Some("GBP".to_string()));
    let totals = parsed.totals.as_ref().unwrap();
    assert_eq!(totals.vat_total_in_tax_currency, Some(dec!(158.70)));
}

// ===========================================================================
// Feature 5: Document Attachments (BG-24)
// ===========================================================================

fn invoice_with_attachment() -> Invoice {
    InvoiceBuilder::new("RE-2024-033", date(2024, 6, 15))
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
            LineItemBuilder::new("1", "Beratung", dec!(1), "HUR", dec!(200))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .add_attachment(DocumentAttachment {
            id: Some("ATT-001".into()),
            description: Some("Timesheet".into()),
            external_uri: None,
            embedded_document: Some(EmbeddedDocument {
                content: "dGVzdA==".into(), // base64("test")
                mime_type: "application/pdf".into(),
                filename: "timesheet.pdf".into(),
            }),
        })
        .add_attachment(DocumentAttachment {
            id: Some("ATT-002".into()),
            description: Some("External spec".into()),
            external_uri: Some("https://example.com/spec.pdf".into()),
            embedded_document: None,
        })
        .payment(PaymentInstructions {
            means_code: PaymentMeansCode::SepaCreditTransfer,
            means_text: None,
            remittance_info: None,
            credit_transfer: Some(CreditTransfer {
                iban: "DE89370400440532013000".into(),
                bic: None,
                account_name: None,
            }),
        })
        .build()
        .expect("valid invoice with attachments")
}

#[test]
fn attachment_ubl_generation() {
    let inv = invoice_with_attachment();
    let xml = xrechnung::to_ubl_xml(&inv).unwrap();

    assert!(xml.contains("cac:AdditionalDocumentReference"));
    assert!(xml.contains("<cbc:ID>ATT-001</cbc:ID>"));
    assert!(xml.contains("<cbc:DocumentDescription>Timesheet</cbc:DocumentDescription>"));
    assert!(xml.contains("mimeCode=\"application/pdf\""));
    assert!(xml.contains("filename=\"timesheet.pdf\""));
    assert!(xml.contains("dGVzdA=="));
    assert!(xml.contains("<cbc:ID>ATT-002</cbc:ID>"));
}

#[test]
fn attachment_ubl_roundtrip() {
    let original = invoice_with_attachment();
    let xml = xrechnung::to_ubl_xml(&original).unwrap();
    let parsed = xrechnung::from_ubl_xml(&xml).unwrap();

    assert_eq!(parsed.attachments.len(), 2);

    let att1 = &parsed.attachments[0];
    assert_eq!(att1.id, Some("ATT-001".to_string()));
    assert_eq!(att1.description, Some("Timesheet".to_string()));
    let emb = att1.embedded_document.as_ref().unwrap();
    assert_eq!(emb.content, "dGVzdA==");
    assert_eq!(emb.mime_type, "application/pdf");
    assert_eq!(emb.filename, "timesheet.pdf");

    let att2 = &parsed.attachments[1];
    assert_eq!(att2.id, Some("ATT-002".to_string()));
    assert_eq!(att2.description, Some("External spec".to_string()));
}

#[test]
fn attachment_cii_generation() {
    let inv = invoice_with_attachment();
    let xml = xrechnung::to_cii_xml(&inv).unwrap();

    assert!(xml.contains("ram:AdditionalReferencedDocument"));
    assert!(xml.contains("<ram:IssuerAssignedID>ATT-001</ram:IssuerAssignedID>"));
    assert!(xml.contains("<ram:TypeCode>916</ram:TypeCode>"));
    assert!(xml.contains("<ram:Name>Timesheet</ram:Name>"));
    assert!(xml.contains("mimeCode=\"application/pdf\""));
    assert!(xml.contains("dGVzdA=="));
}

#[test]
fn attachment_cii_roundtrip() {
    let original = invoice_with_attachment();
    let xml = xrechnung::to_cii_xml(&original).unwrap();
    let parsed = xrechnung::from_cii_xml(&xml).unwrap();

    assert_eq!(parsed.attachments.len(), 2);

    let att1 = &parsed.attachments[0];
    assert_eq!(att1.id, Some("ATT-001".to_string()));
    assert_eq!(att1.description, Some("Timesheet".to_string()));
    let emb = att1.embedded_document.as_ref().unwrap();
    assert_eq!(emb.content, "dGVzdA==");
    assert_eq!(emb.mime_type, "application/pdf");
    assert_eq!(emb.filename, "timesheet.pdf");
}

#[test]
fn attachment_limit_enforced() {
    let mut builder = InvoiceBuilder::new("RE-LIMIT", date(2024, 6, 15))
        .tax_point_date(date(2024, 6, 15))
        .seller(
            PartyBuilder::new(
                "S GmbH",
                AddressBuilder::new("Berlin", "10115", "DE").build(),
            )
            .vat_id("DE123456789")
            .build(),
        )
        .buyer(
            PartyBuilder::new(
                "B AG",
                AddressBuilder::new("München", "80331", "DE").build(),
            )
            .build(),
        )
        .add_line(
            LineItemBuilder::new("1", "Test", dec!(1), "C62", dec!(100))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        );

    for i in 0..101 {
        builder = builder.add_attachment(DocumentAttachment {
            id: Some(format!("ATT-{i}")),
            description: None,
            external_uri: Some("https://example.com".into()),
            embedded_document: None,
        });
    }

    let result = builder.build();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("100 attachments"));
}

// ---------------------------------------------------------------------------
// Snapshot tests (insta)
// ---------------------------------------------------------------------------

#[test]
fn ubl_snapshot() {
    let inv = xrechnung_invoice();
    let xml = xrechnung::to_ubl_xml(&inv).unwrap();
    insta::assert_snapshot!("ubl_domestic_invoice", xml);
}

#[test]
fn cii_snapshot() {
    let inv = xrechnung_invoice();
    let xml = xrechnung::to_cii_xml(&inv).unwrap();
    insta::assert_snapshot!("cii_domestic_invoice", xml);
}
