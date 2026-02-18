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
