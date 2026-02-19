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
            card_payment: None,
            direct_debit: None,
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
            card_payment: None,
            direct_debit: None,
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
            card_payment: None,
            direct_debit: None,
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
            card_payment: None,
            direct_debit: None,
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
// BR-DE-17: Type code 877 allowed
// ---------------------------------------------------------------------------

#[test]
fn xrechnung_type_code_877_allowed() {
    let mut inv = xrechnung_invoice();
    inv.type_code = InvoiceTypeCode::Other(877);
    let errors = xrechnung::validate_xrechnung(&inv);
    assert!(
        !errors.iter().any(|e| e.rule.as_deref() == Some("BR-DE-17")),
        "877 should be allowed, got: {:?}",
        errors
    );
}

// ---------------------------------------------------------------------------
// BR-DE-23/24/25: Payment means group exclusion
// ---------------------------------------------------------------------------

#[test]
fn xrechnung_sepa_transfer_requires_credit_transfer() {
    let mut inv = xrechnung_invoice();
    inv.payment = Some(PaymentInstructions {
        means_code: PaymentMeansCode::SepaCreditTransfer,
        means_text: None,
        remittance_info: None,
        credit_transfer: None, // missing!
        card_payment: None,
        direct_debit: None,
    });
    let errors = xrechnung::validate_xrechnung(&inv);
    assert!(
        errors.iter().any(|e| e.rule.as_deref() == Some("BR-DE-23")),
        "Should require credit transfer for code 58"
    );
}

#[test]
fn xrechnung_card_code_requires_card_payment() {
    let mut inv = xrechnung_invoice();
    inv.payment = Some(PaymentInstructions {
        means_code: PaymentMeansCode::BankCard,
        means_text: None,
        remittance_info: None,
        credit_transfer: None,
        card_payment: None, // missing!
        direct_debit: None,
    });
    let errors = xrechnung::validate_xrechnung(&inv);
    assert!(
        errors.iter().any(|e| e.rule.as_deref() == Some("BR-DE-24")),
        "Should require card payment for code 48"
    );
}

#[test]
fn xrechnung_direct_debit_requires_debit_info() {
    let mut inv = xrechnung_invoice();
    inv.payment = Some(PaymentInstructions {
        means_code: PaymentMeansCode::SepaDirectDebit,
        means_text: None,
        remittance_info: None,
        credit_transfer: None,
        card_payment: None,
        direct_debit: None, // missing!
    });
    let errors = xrechnung::validate_xrechnung(&inv);
    assert!(
        errors.iter().any(|e| e.rule.as_deref() == Some("BR-DE-25")),
        "Should require direct debit for code 59"
    );
}

#[test]
fn xrechnung_card_code_rejects_credit_transfer() {
    let mut inv = xrechnung_invoice();
    inv.payment = Some(PaymentInstructions {
        means_code: PaymentMeansCode::BankCard,
        means_text: None,
        remittance_info: None,
        credit_transfer: Some(CreditTransfer {
            iban: "DE89370400440532013000".into(),
            bic: None,
            account_name: None,
        }),
        card_payment: Some(CardPayment {
            account_number: "1234".into(),
            holder_name: None,
        }),
        direct_debit: None,
    });
    let errors = xrechnung::validate_xrechnung(&inv);
    assert!(
        errors
            .iter()
            .any(|e| e.rule.as_deref() == Some("BR-DE-24") && e.message.contains("must not")),
        "Card code should reject credit transfer"
    );
}

// ---------------------------------------------------------------------------
// BR-DE-27: Telephone format
// ---------------------------------------------------------------------------

#[test]
fn xrechnung_phone_needs_digits() {
    let mut inv = xrechnung_invoice();
    inv.seller.contact = Some(Contact {
        name: Some("Test".into()),
        phone: Some("ab".into()), // no digits
        email: Some("a@b.de".into()),
    });
    let errors = xrechnung::validate_xrechnung(&inv);
    assert!(
        errors.iter().any(|e| e.rule.as_deref() == Some("BR-DE-27")),
        "Phone with <3 digits should trigger BR-DE-27"
    );
}

// ---------------------------------------------------------------------------
// BR-DE-28: Email format
// ---------------------------------------------------------------------------

#[test]
fn xrechnung_email_needs_at_sign() {
    let mut inv = xrechnung_invoice();
    inv.seller.contact = Some(Contact {
        name: Some("Test".into()),
        phone: Some("+49 30 12345".into()),
        email: Some("not-an-email".into()),
    });
    let errors = xrechnung::validate_xrechnung(&inv);
    assert!(
        errors
            .iter()
            .any(|e| e.rule.as_deref() == Some("BR-DE-28") && e.field == "seller.contact.email"),
        "Email without @ should trigger BR-DE-28"
    );
}

// ---------------------------------------------------------------------------
// BR-DE-19: IBAN format
// ---------------------------------------------------------------------------

#[test]
fn xrechnung_invalid_iban_format() {
    let mut inv = xrechnung_invoice();
    inv.payment = Some(PaymentInstructions {
        means_code: PaymentMeansCode::SepaCreditTransfer,
        means_text: None,
        remittance_info: None,
        credit_transfer: Some(CreditTransfer {
            iban: "12345".into(), // not a valid IBAN format
            bic: None,
            account_name: None,
        }),
        card_payment: None,
        direct_debit: None,
    });
    let errors = xrechnung::validate_xrechnung(&inv);
    assert!(
        errors.iter().any(|e| e.rule.as_deref() == Some("BR-DE-19")),
        "Invalid IBAN format should trigger BR-DE-19"
    );
}

// ---------------------------------------------------------------------------
// BR-DE-30/31: Direct debit requires creditor ID + debited account
// ---------------------------------------------------------------------------

#[test]
fn xrechnung_direct_debit_needs_creditor_and_account() {
    let mut inv = xrechnung_invoice();
    inv.payment = Some(PaymentInstructions {
        means_code: PaymentMeansCode::SepaDirectDebit,
        means_text: None,
        remittance_info: None,
        credit_transfer: None,
        card_payment: None,
        direct_debit: Some(DirectDebit {
            mandate_id: Some("MANDATE-001".into()),
            creditor_id: None,        // missing
            debited_account_id: None, // missing
        }),
    });
    let errors = xrechnung::validate_xrechnung(&inv);
    assert!(
        errors.iter().any(|e| e.rule.as_deref() == Some("BR-DE-30")),
        "Missing creditor ID should trigger BR-DE-30"
    );
    assert!(
        errors.iter().any(|e| e.rule.as_deref() == Some("BR-DE-31")),
        "Missing debited account should trigger BR-DE-31"
    );
}

// ---------------------------------------------------------------------------
// BR-DE-26: Corrected invoice should reference preceding
// ---------------------------------------------------------------------------

#[test]
fn xrechnung_corrected_invoice_needs_preceding() {
    let mut inv = xrechnung_invoice();
    inv.type_code = InvoiceTypeCode::Corrected;
    inv.preceding_invoices = vec![];
    let errors = xrechnung::validate_xrechnung(&inv);
    assert!(
        errors
            .iter()
            .any(|e| e.rule.as_deref() == Some("BR-DE-26") && e.field == "preceding_invoices"),
        "Corrected invoice (384) without preceding reference should trigger BR-DE-26"
    );
}

// ---------------------------------------------------------------------------
// BR-DE-22: Unique attachment filenames
// ---------------------------------------------------------------------------

#[test]
fn xrechnung_duplicate_attachment_filenames() {
    let mut inv = xrechnung_invoice();
    inv.attachments = vec![
        DocumentAttachment {
            id: Some("ATT-1".into()),
            description: None,
            external_uri: None,
            embedded_document: Some(EmbeddedDocument {
                content: "AQID".into(),
                mime_type: "application/pdf".into(),
                filename: "doc.pdf".into(),
            }),
        },
        DocumentAttachment {
            id: Some("ATT-2".into()),
            description: None,
            external_uri: None,
            embedded_document: Some(EmbeddedDocument {
                content: "BAUG".into(),
                mime_type: "application/pdf".into(),
                filename: "doc.pdf".into(), // duplicate!
            }),
        },
    ];
    let errors = xrechnung::validate_xrechnung(&inv);
    assert!(
        errors.iter().any(|e| e.rule.as_deref() == Some("BR-DE-22")),
        "Duplicate attachment filenames should trigger BR-DE-22"
    );
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
            card_payment: None,
            direct_debit: None,
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
            card_payment: None,
            direct_debit: None,
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
            card_payment: None,
            direct_debit: None,
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
            card_payment: None,
            direct_debit: None,
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
            card_payment: None,
            direct_debit: None,
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
            card_payment: None,
            direct_debit: None,
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
            card_payment: None,
            direct_debit: None,
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
            card_payment: None,
            direct_debit: None,
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
// BG-29: Price discount (gross price) tests
// ---------------------------------------------------------------------------

fn invoice_with_gross_price() -> Invoice {
    InvoiceBuilder::new("RE-GP-001", date(2024, 6, 15))
        .tax_point_date(date(2024, 6, 15))
        .buyer_reference("04011000-12345-03")
        .seller(
            PartyBuilder::new(
                "Seller GmbH",
                AddressBuilder::new("Berlin", "10115", "DE").build(),
            )
            .vat_id("DE123456789")
            .electronic_address("EM", "seller@test.de")
            .build(),
        )
        .buyer(
            PartyBuilder::new(
                "Buyer AG",
                AddressBuilder::new("München", "80331", "DE").build(),
            )
            .electronic_address("EM", "buyer@test.de")
            .build(),
        )
        .add_line(
            LineItemBuilder::new("1", "Widget", dec!(5), "C62", dec!(80))
                .gross_price(dec!(100))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .build()
        .unwrap()
}

#[test]
fn gross_price_ubl_generation() {
    let inv = invoice_with_gross_price();
    let xml = xrechnung::to_ubl_xml(&inv).unwrap();
    assert!(xml.contains("<cbc:PriceAmount"));
    assert!(xml.contains("<cbc:BaseAmount"));
    assert!(xml.contains("<cbc:ChargeIndicator>false</cbc:ChargeIndicator>"));
    assert!(xml.contains("100"));
}

#[test]
fn gross_price_ubl_roundtrip() {
    let inv = invoice_with_gross_price();
    let xml = xrechnung::to_ubl_xml(&inv).unwrap();
    let parsed = xrechnung::from_ubl_xml(&xml).unwrap();
    assert_eq!(parsed.lines[0].gross_price, Some(dec!(100)));
    assert_eq!(parsed.lines[0].unit_price, dec!(80));
}

#[test]
fn gross_price_cii_generation() {
    let inv = invoice_with_gross_price();
    let xml = xrechnung::to_cii_xml(&inv).unwrap();
    assert!(xml.contains("GrossPriceProductTradePrice"));
    assert!(xml.contains("AppliedTradeAllowanceCharge"));
    assert!(xml.contains("NetPriceProductTradePrice"));
}

#[test]
fn gross_price_cii_roundtrip() {
    let inv = invoice_with_gross_price();
    let xml = xrechnung::to_cii_xml(&inv).unwrap();
    let parsed = xrechnung::from_cii_xml(&xml).unwrap();
    assert_eq!(parsed.lines[0].gross_price, Some(dec!(100)));
    assert_eq!(parsed.lines[0].unit_price, dec!(80));
}

// ---------------------------------------------------------------------------
// BG-27/BG-28: Line-level allowances and charges tests
// ---------------------------------------------------------------------------

fn invoice_with_line_charges() -> Invoice {
    InvoiceBuilder::new("RE-LC-001", date(2024, 6, 15))
        .tax_point_date(date(2024, 6, 15))
        .buyer_reference("04011000-12345-03")
        .seller(
            PartyBuilder::new(
                "Seller GmbH",
                AddressBuilder::new("Berlin", "10115", "DE").build(),
            )
            .vat_id("DE123456789")
            .electronic_address("EM", "seller@test.de")
            .build(),
        )
        .buyer(
            PartyBuilder::new(
                "Buyer AG",
                AddressBuilder::new("München", "80331", "DE").build(),
            )
            .electronic_address("EM", "buyer@test.de")
            .build(),
        )
        .add_line(
            LineItemBuilder::new("1", "Service", dec!(10), "HUR", dec!(100))
                .tax(TaxCategory::StandardRate, dec!(19))
                .add_charge(AllowanceCharge {
                    is_charge: true,
                    amount: dec!(50),
                    percentage: None,
                    base_amount: None,
                    tax_category: TaxCategory::StandardRate,
                    tax_rate: dec!(19),
                    reason: Some("Rush fee".into()),
                    reason_code: Some("FC".into()),
                })
                .add_allowance(AllowanceCharge {
                    is_charge: false,
                    amount: dec!(100),
                    percentage: None,
                    base_amount: None,
                    tax_category: TaxCategory::StandardRate,
                    tax_rate: dec!(19),
                    reason: Some("Volume discount".into()),
                    reason_code: Some("95".into()),
                })
                .build(),
        )
        .build()
        .unwrap()
}

#[test]
fn line_charges_totals_correct() {
    let inv = invoice_with_line_charges();
    assert_eq!(inv.lines[0].line_amount, Some(dec!(950)));
    let totals = inv.totals.as_ref().unwrap();
    assert_eq!(totals.line_net_total, dec!(950));
}

#[test]
fn line_charges_ubl_generation() {
    let inv = invoice_with_line_charges();
    let xml = xrechnung::to_ubl_xml(&inv).unwrap();
    assert!(xml.contains("<cbc:ChargeIndicator>true</cbc:ChargeIndicator>"));
    assert!(xml.contains("<cbc:ChargeIndicator>false</cbc:ChargeIndicator>"));
    assert!(xml.contains("Rush fee"));
    assert!(xml.contains("Volume discount"));
}

#[test]
fn line_charges_ubl_roundtrip() {
    let inv = invoice_with_line_charges();
    let xml = xrechnung::to_ubl_xml(&inv).unwrap();
    let parsed = xrechnung::from_ubl_xml(&xml).unwrap();
    assert_eq!(parsed.lines[0].charges.len(), 1);
    assert_eq!(parsed.lines[0].allowances.len(), 1);
    assert_eq!(parsed.lines[0].charges[0].amount, dec!(50));
    assert_eq!(
        parsed.lines[0].charges[0].reason.as_deref(),
        Some("Rush fee")
    );
    assert_eq!(parsed.lines[0].allowances[0].amount, dec!(100));
    assert_eq!(
        parsed.lines[0].allowances[0].reason.as_deref(),
        Some("Volume discount")
    );
}

#[test]
fn line_charges_cii_generation() {
    let inv = invoice_with_line_charges();
    let xml = xrechnung::to_cii_xml(&inv).unwrap();
    assert!(xml.contains("SpecifiedTradeAllowanceCharge"));
    assert!(xml.contains("Rush fee"));
    assert!(xml.contains("Volume discount"));
}

#[test]
fn line_charges_cii_roundtrip() {
    let inv = invoice_with_line_charges();
    let xml = xrechnung::to_cii_xml(&inv).unwrap();
    let parsed = xrechnung::from_cii_xml(&xml).unwrap();
    assert_eq!(parsed.lines[0].charges.len(), 1);
    assert_eq!(parsed.lines[0].allowances.len(), 1);
    assert_eq!(parsed.lines[0].charges[0].amount, dec!(50));
    assert_eq!(parsed.lines[0].allowances[0].amount, dec!(100));
}

// ---------------------------------------------------------------------------
// BG-20/BG-21: Document-level allowances/charges roundtrip tests
// ---------------------------------------------------------------------------

#[test]
fn doc_allowances_charges_ubl_roundtrip() {
    let inv = xrechnung_invoice();
    let xml = xrechnung::to_ubl_xml(&inv).unwrap();
    let parsed = xrechnung::from_ubl_xml(&xml).unwrap();
    assert_eq!(parsed.allowances.len(), inv.allowances.len());
    assert_eq!(parsed.charges.len(), inv.charges.len());
    for (orig, parsed) in inv.allowances.iter().zip(parsed.allowances.iter()) {
        assert_eq!(orig.amount, parsed.amount);
    }
    for (orig, parsed) in inv.charges.iter().zip(parsed.charges.iter()) {
        assert_eq!(orig.amount, parsed.amount);
    }
}

#[test]
fn doc_allowances_charges_cii_roundtrip() {
    let inv = xrechnung_invoice();
    let xml = xrechnung::to_cii_xml(&inv).unwrap();
    let parsed = xrechnung::from_cii_xml(&xml).unwrap();
    assert_eq!(parsed.allowances.len(), inv.allowances.len());
    assert_eq!(parsed.charges.len(), inv.charges.len());
    for (orig, parsed) in inv.allowances.iter().zip(parsed.allowances.iter()) {
        assert_eq!(orig.amount, parsed.amount);
    }
    for (orig, parsed) in inv.charges.iter().zip(parsed.charges.iter()) {
        assert_eq!(orig.amount, parsed.amount);
    }
}

// ---------------------------------------------------------------------------
// Delivery information tests (BG-13/BG-14/BG-15)
// ---------------------------------------------------------------------------

#[test]
fn delivery_full_ubl_roundtrip() {
    // Test roundtrip: serialize to UBL → deserialize → verify all fields present
    let delivery = DeliveryInformation {
        actual_delivery_date: Some(NaiveDate::from_ymd_opt(2024, 6, 15).unwrap()),
        delivery_party: Some(DeliveryParty {
            name: "Warehouse Hamburg".to_string(),
            location_id: Some("DE-WH-001".to_string()),
        }),
        delivery_address: Some(DeliveryAddress {
            street: Some("Hafenstrasse 42".to_string()),
            additional: Some("Building C".to_string()),
            city: "Hamburg".to_string(),
            postal_code: "20095".to_string(),
            subdivision: Some("Hamburg".to_string()),
            country_code: "DE".to_string(),
        }),
    };

    let mut inv = InvoiceBuilder::new("INV-001", date(2024, 6, 15))
        .tax_point_date(date(2024, 6, 15))
        .seller(
            PartyBuilder::new(
                "Seller Ltd",
                AddressBuilder::new("Berlin", "10115", "DE").build(),
            )
            .vat_id("DE100000000")
            .build(),
        )
        .buyer(
            PartyBuilder::new(
                "Buyer Inc",
                AddressBuilder::new("Munich", "80331", "DE").build(),
            )
            .build(),
        )
        .add_line(
            LineItemBuilder::new("1", "Service", dec!(1), "C62", dec!(100))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .build()
        .unwrap();

    inv.delivery = Some(delivery.clone());

    // Serialize to UBL XML
    let xml = xrechnung::to_ubl_xml(&inv).unwrap();

    // Deserialize back
    let parsed = xrechnung::from_ubl_xml(&xml).unwrap();

    // Verify delivery information roundtrips correctly
    assert!(
        parsed.delivery.is_some(),
        "Delivery information should be present"
    );
    let parsed_delivery = parsed.delivery.unwrap();

    assert_eq!(
        parsed_delivery.actual_delivery_date, delivery.actual_delivery_date,
        "Actual delivery date mismatch"
    );

    assert!(
        parsed_delivery.delivery_party.is_some(),
        "Delivery party should be present"
    );
    let parsed_party = parsed_delivery.delivery_party.unwrap();
    assert_eq!(
        parsed_party.name,
        delivery.delivery_party.as_ref().unwrap().name,
        "Delivery party name mismatch"
    );
    assert_eq!(
        parsed_party.location_id,
        delivery.delivery_party.as_ref().unwrap().location_id,
        "Delivery party location ID mismatch"
    );

    assert!(
        parsed_delivery.delivery_address.is_some(),
        "Delivery address should be present"
    );
    let parsed_addr = parsed_delivery.delivery_address.unwrap();
    let expected_addr = delivery.delivery_address.as_ref().unwrap();

    assert_eq!(parsed_addr.street, expected_addr.street, "Street mismatch");
    assert_eq!(
        parsed_addr.additional, expected_addr.additional,
        "Additional street mismatch"
    );
    assert_eq!(parsed_addr.city, expected_addr.city, "City mismatch");
    assert_eq!(
        parsed_addr.postal_code, expected_addr.postal_code,
        "Postal code mismatch"
    );
    assert_eq!(
        parsed_addr.subdivision, expected_addr.subdivision,
        "Subdivision mismatch"
    );
    assert_eq!(
        parsed_addr.country_code, expected_addr.country_code,
        "Country code mismatch"
    );
}

#[test]
fn delivery_minimal_cii_generation() {
    // Test minimal delivery: only actual_delivery_date (no party/address)
    let delivery = DeliveryInformation {
        actual_delivery_date: Some(NaiveDate::from_ymd_opt(2024, 6, 15).unwrap()),
        delivery_party: None,
        delivery_address: None,
    };

    let mut inv = InvoiceBuilder::new("INV-002", date(2024, 6, 15))
        .tax_point_date(date(2024, 6, 15))
        .seller(
            PartyBuilder::new(
                "Seller Ltd",
                AddressBuilder::new("Berlin", "10115", "DE").build(),
            )
            .vat_id("DE100000000")
            .build(),
        )
        .buyer(
            PartyBuilder::new(
                "Buyer Inc",
                AddressBuilder::new("Munich", "80331", "DE").build(),
            )
            .build(),
        )
        .add_line(
            LineItemBuilder::new("1", "Service", dec!(1), "C62", dec!(100))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .build()
        .unwrap();

    inv.delivery = Some(delivery);

    // Should generate CII XML without errors
    let xml = xrechnung::to_cii_xml(&inv).unwrap();
    assert!(xml.contains("ApplicableHeaderTradeDelivery"));
}

#[test]
fn delivery_full_cii_generation() {
    // Test full delivery with party and address in CII format
    let delivery = DeliveryInformation {
        actual_delivery_date: Some(NaiveDate::from_ymd_opt(2024, 6, 15).unwrap()),
        delivery_party: Some(DeliveryParty {
            name: "Logistics Partner".to_string(),
            location_id: Some("DE-LOG-999".to_string()),
        }),
        delivery_address: Some(DeliveryAddress {
            street: Some("Logistikstrasse 99".to_string()),
            additional: Some("Warehouse East".to_string()),
            city: "Frankfurt".to_string(),
            postal_code: "60311".to_string(),
            subdivision: Some("Hesse".to_string()),
            country_code: "DE".to_string(),
        }),
    };

    let mut inv = InvoiceBuilder::new("INV-003", date(2024, 6, 15))
        .tax_point_date(date(2024, 6, 15))
        .seller(
            PartyBuilder::new(
                "Seller Ltd",
                AddressBuilder::new("Berlin", "10115", "DE").build(),
            )
            .vat_id("DE100000000")
            .build(),
        )
        .buyer(
            PartyBuilder::new(
                "Buyer Inc",
                AddressBuilder::new("Munich", "80331", "DE").build(),
            )
            .build(),
        )
        .add_line(
            LineItemBuilder::new("1", "Service", dec!(1), "C62", dec!(100))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .build()
        .unwrap();

    inv.delivery = Some(delivery);

    // Should generate CII XML without errors
    let xml = xrechnung::to_cii_xml(&inv).unwrap();
    assert!(xml.contains("ApplicableHeaderTradeDelivery"));
    assert!(xml.contains("LogisticsPartner") || xml.contains("Logistics Partner"));
}

#[test]
fn delivery_address_only_ubl_roundtrip() {
    // Test delivery with address only (no party)
    let delivery = DeliveryInformation {
        actual_delivery_date: None,
        delivery_party: None,
        delivery_address: Some(DeliveryAddress {
            street: Some("Shipment St. 1".to_string()),
            additional: None,
            city: "Cologne".to_string(),
            postal_code: "50667".to_string(),
            subdivision: None,
            country_code: "DE".to_string(),
        }),
    };

    let mut inv = InvoiceBuilder::new("INV-004", date(2024, 6, 15))
        .tax_point_date(date(2024, 6, 15))
        .seller(
            PartyBuilder::new(
                "Seller Ltd",
                AddressBuilder::new("Berlin", "10115", "DE").build(),
            )
            .vat_id("DE100000000")
            .build(),
        )
        .buyer(
            PartyBuilder::new(
                "Buyer Inc",
                AddressBuilder::new("Munich", "80331", "DE").build(),
            )
            .build(),
        )
        .add_line(
            LineItemBuilder::new("1", "Service", dec!(1), "C62", dec!(100))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .build()
        .unwrap();

    inv.delivery = Some(delivery.clone());

    // Serialize to UBL
    let xml = xrechnung::to_ubl_xml(&inv).unwrap();

    // Deserialize back
    let parsed = xrechnung::from_ubl_xml(&xml).unwrap();

    // Verify address roundtrips
    assert!(parsed.delivery.is_some());
    let parsed_delivery = parsed.delivery.unwrap();
    assert!(parsed_delivery.delivery_address.is_some());

    let parsed_addr = parsed_delivery.delivery_address.unwrap();
    let expected_addr = delivery.delivery_address.unwrap();

    assert_eq!(parsed_addr.street, expected_addr.street);
    assert_eq!(parsed_addr.city, expected_addr.city);
    assert_eq!(parsed_addr.postal_code, expected_addr.postal_code);
    assert_eq!(parsed_addr.country_code, expected_addr.country_code);
}

#[test]
fn delivery_no_delivery_info() {
    // Test invoice without any delivery information (should not break)
    let inv = InvoiceBuilder::new("INV-005", date(2024, 6, 15))
        .tax_point_date(date(2024, 6, 15))
        .seller(
            PartyBuilder::new(
                "Seller Ltd",
                AddressBuilder::new("Berlin", "10115", "DE").build(),
            )
            .vat_id("DE100000000")
            .build(),
        )
        .buyer(
            PartyBuilder::new(
                "Buyer Inc",
                AddressBuilder::new("Munich", "80331", "DE").build(),
            )
            .build(),
        )
        .add_line(
            LineItemBuilder::new("1", "Service", dec!(1), "C62", dec!(100))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .build()
        .unwrap();

    // Should serialize without errors
    let xml = xrechnung::to_ubl_xml(&inv).unwrap();
    assert!(xml.contains("Invoice"));

    // Should deserialize without errors
    let _parsed = xrechnung::from_ubl_xml(&xml).unwrap();
}

// ---------------------------------------------------------------------------
// BG-10: Payee party tests
// ---------------------------------------------------------------------------

fn invoice_with_payee() -> Invoice {
    InvoiceBuilder::new("RE-PAYEE-001", date(2024, 6, 15))
        .tax_point_date(date(2024, 6, 15))
        .buyer_reference("04011000-12345-03")
        .seller(
            PartyBuilder::new(
                "Seller GmbH",
                AddressBuilder::new("Berlin", "10115", "DE").build(),
            )
            .vat_id("DE123456789")
            .electronic_address("EM", "seller@test.de")
            .contact(None, Some("+49 30 12345".into()), Some("s@test.de".into()))
            .build(),
        )
        .buyer(
            PartyBuilder::new(
                "Buyer AG",
                AddressBuilder::new("München", "80331", "DE").build(),
            )
            .electronic_address("EM", "buyer@test.de")
            .build(),
        )
        .payee(Payee {
            name: "Payment Receiver Ltd".into(),
            identifier: Some("PAYEE-123".into()),
            legal_registration_id: Some("HRB 98765".into()),
        })
        .add_line(
            LineItemBuilder::new("1", "Service", dec!(1), "C62", dec!(100))
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
            card_payment: None,
            direct_debit: None,
        })
        .build()
        .unwrap()
}

#[test]
fn payee_ubl_generation() {
    let inv = invoice_with_payee();
    let xml = xrechnung::to_ubl_xml(&inv).unwrap();
    assert!(xml.contains("cac:PayeeParty"));
    assert!(xml.contains("Payment Receiver Ltd"));
    assert!(xml.contains("PAYEE-123"));
    assert!(xml.contains("HRB 98765"));
}

#[test]
fn payee_ubl_roundtrip() {
    let inv = invoice_with_payee();
    let xml = xrechnung::to_ubl_xml(&inv).unwrap();
    let parsed = xrechnung::from_ubl_xml(&xml).unwrap();
    let payee = parsed.payee.unwrap();
    assert_eq!(payee.name, "Payment Receiver Ltd");
    assert_eq!(payee.identifier.as_deref(), Some("PAYEE-123"));
    assert_eq!(payee.legal_registration_id.as_deref(), Some("HRB 98765"));
}

#[test]
fn payee_cii_generation() {
    let inv = invoice_with_payee();
    let xml = xrechnung::to_cii_xml(&inv).unwrap();
    assert!(xml.contains("ram:PayeeTradeParty"));
    assert!(xml.contains("Payment Receiver Ltd"));
}

#[test]
fn payee_cii_roundtrip() {
    let inv = invoice_with_payee();
    let xml = xrechnung::to_cii_xml(&inv).unwrap();
    let parsed = xrechnung::from_cii_xml(&xml).unwrap();
    let payee = parsed.payee.unwrap();
    assert_eq!(payee.name, "Payment Receiver Ltd");
    assert_eq!(payee.identifier.as_deref(), Some("PAYEE-123"));
    assert_eq!(payee.legal_registration_id.as_deref(), Some("HRB 98765"));
}

// ---------------------------------------------------------------------------
// BG-11: Seller tax representative tests
// ---------------------------------------------------------------------------

fn invoice_with_tax_rep() -> Invoice {
    InvoiceBuilder::new("RE-TAXREP-001", date(2024, 6, 15))
        .tax_point_date(date(2024, 6, 15))
        .buyer_reference("04011000-12345-03")
        .seller(
            PartyBuilder::new(
                "Seller GmbH",
                AddressBuilder::new("Berlin", "10115", "DE").build(),
            )
            .vat_id("DE123456789")
            .electronic_address("EM", "seller@test.de")
            .contact(None, Some("+49 30 12345".into()), Some("s@test.de".into()))
            .build(),
        )
        .buyer(
            PartyBuilder::new(
                "Buyer AG",
                AddressBuilder::new("München", "80331", "DE").build(),
            )
            .electronic_address("EM", "buyer@test.de")
            .build(),
        )
        .tax_representative(TaxRepresentative {
            name: "Tax Rep GmbH".into(),
            vat_id: "DE987654321".into(),
            address: AddressBuilder::new("Hamburg", "20095", "DE")
                .street("Jungfernstieg 1")
                .subdivision("Hamburg")
                .build(),
        })
        .add_line(
            LineItemBuilder::new("1", "Service", dec!(1), "C62", dec!(100))
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
            card_payment: None,
            direct_debit: None,
        })
        .build()
        .unwrap()
}

#[test]
fn tax_representative_ubl_generation() {
    let inv = invoice_with_tax_rep();
    let xml = xrechnung::to_ubl_xml(&inv).unwrap();
    assert!(xml.contains("cac:TaxRepresentativeParty"));
    assert!(xml.contains("Tax Rep GmbH"));
    assert!(xml.contains("DE987654321"));
    assert!(xml.contains("Jungfernstieg 1"));
}

#[test]
fn tax_representative_ubl_roundtrip() {
    let inv = invoice_with_tax_rep();
    let xml = xrechnung::to_ubl_xml(&inv).unwrap();
    let parsed = xrechnung::from_ubl_xml(&xml).unwrap();
    let tr = parsed.tax_representative.unwrap();
    assert_eq!(tr.name, "Tax Rep GmbH");
    assert_eq!(tr.vat_id, "DE987654321");
    assert_eq!(tr.address.city, "Hamburg");
    assert_eq!(tr.address.subdivision.as_deref(), Some("Hamburg"));
}

#[test]
fn tax_representative_cii_generation() {
    let inv = invoice_with_tax_rep();
    let xml = xrechnung::to_cii_xml(&inv).unwrap();
    assert!(xml.contains("ram:SellerTaxRepresentativeTradeParty"));
    assert!(xml.contains("Tax Rep GmbH"));
    assert!(xml.contains("DE987654321"));
}

#[test]
fn tax_representative_cii_roundtrip() {
    let inv = invoice_with_tax_rep();
    let xml = xrechnung::to_cii_xml(&inv).unwrap();
    let parsed = xrechnung::from_cii_xml(&xml).unwrap();
    let tr = parsed.tax_representative.unwrap();
    assert_eq!(tr.name, "Tax Rep GmbH");
    assert_eq!(tr.vat_id, "DE987654321");
    assert_eq!(tr.address.city, "Hamburg");
    assert_eq!(tr.address.subdivision.as_deref(), Some("Hamburg"));
}

// ---------------------------------------------------------------------------
// BG-18: Card payment tests
// ---------------------------------------------------------------------------

fn invoice_with_card_payment() -> Invoice {
    InvoiceBuilder::new("RE-CARD-001", date(2024, 6, 15))
        .tax_point_date(date(2024, 6, 15))
        .buyer_reference("04011000-12345-03")
        .seller(
            PartyBuilder::new(
                "Seller GmbH",
                AddressBuilder::new("Berlin", "10115", "DE").build(),
            )
            .vat_id("DE123456789")
            .electronic_address("EM", "seller@test.de")
            .contact(None, Some("+49 30 12345".into()), Some("s@test.de".into()))
            .build(),
        )
        .buyer(
            PartyBuilder::new(
                "Buyer AG",
                AddressBuilder::new("München", "80331", "DE").build(),
            )
            .electronic_address("EM", "buyer@test.de")
            .build(),
        )
        .add_line(
            LineItemBuilder::new("1", "Service", dec!(1), "C62", dec!(100))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .payment(PaymentInstructions {
            means_code: PaymentMeansCode::from_code(48), // card payment
            means_text: Some("Visa ending 4242".into()),
            remittance_info: Some("Payment for RE-CARD-001".into()),
            credit_transfer: None,
            card_payment: Some(CardPayment {
                account_number: "4242".into(),
                holder_name: Some("Max Mustermann".into()),
            }),
            direct_debit: None,
        })
        .build()
        .unwrap()
}

#[test]
fn card_payment_ubl_generation() {
    let inv = invoice_with_card_payment();
    let xml = xrechnung::to_ubl_xml(&inv).unwrap();
    assert!(xml.contains("cac:CardAccount"));
    assert!(xml.contains("4242"));
    assert!(xml.contains("Max Mustermann"));
}

#[test]
fn card_payment_ubl_roundtrip() {
    let inv = invoice_with_card_payment();
    let xml = xrechnung::to_ubl_xml(&inv).unwrap();
    let parsed = xrechnung::from_ubl_xml(&xml).unwrap();
    let payment = parsed.payment.unwrap();
    assert_eq!(payment.means_text.as_deref(), Some("Visa ending 4242"));
    assert_eq!(
        payment.remittance_info.as_deref(),
        Some("Payment for RE-CARD-001")
    );
    let card = payment.card_payment.unwrap();
    assert_eq!(card.account_number, "4242");
    assert_eq!(card.holder_name.as_deref(), Some("Max Mustermann"));
}

#[test]
fn card_payment_cii_generation() {
    let inv = invoice_with_card_payment();
    let xml = xrechnung::to_cii_xml(&inv).unwrap();
    assert!(xml.contains("ram:ApplicableTradeSettlementFinancialCard"));
    assert!(xml.contains("4242"));
    assert!(xml.contains("Max Mustermann"));
    assert!(xml.contains("ram:Information"));
}

#[test]
fn card_payment_cii_roundtrip() {
    let inv = invoice_with_card_payment();
    let xml = xrechnung::to_cii_xml(&inv).unwrap();
    let parsed = xrechnung::from_cii_xml(&xml).unwrap();
    let payment = parsed.payment.unwrap();
    assert_eq!(payment.means_text.as_deref(), Some("Visa ending 4242"));
    let card = payment.card_payment.unwrap();
    assert_eq!(card.account_number, "4242");
    assert_eq!(card.holder_name.as_deref(), Some("Max Mustermann"));
}

// ---------------------------------------------------------------------------
// BG-19: Direct debit tests
// ---------------------------------------------------------------------------

fn invoice_with_direct_debit() -> Invoice {
    InvoiceBuilder::new("RE-DD-001", date(2024, 6, 15))
        .tax_point_date(date(2024, 6, 15))
        .buyer_reference("04011000-12345-03")
        .seller(
            PartyBuilder::new(
                "Seller GmbH",
                AddressBuilder::new("Berlin", "10115", "DE").build(),
            )
            .vat_id("DE123456789")
            .electronic_address("EM", "seller@test.de")
            .contact(None, Some("+49 30 12345".into()), Some("s@test.de".into()))
            .build(),
        )
        .buyer(
            PartyBuilder::new(
                "Buyer AG",
                AddressBuilder::new("München", "80331", "DE").build(),
            )
            .electronic_address("EM", "buyer@test.de")
            .build(),
        )
        .add_line(
            LineItemBuilder::new("1", "Service", dec!(1), "C62", dec!(100))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .payment(PaymentInstructions {
            means_code: PaymentMeansCode::SepaDirectDebit,
            means_text: None,
            remittance_info: None,
            credit_transfer: None,
            card_payment: None,
            direct_debit: Some(DirectDebit {
                mandate_id: Some("MANDATE-001".into()),
                debited_account_id: Some("DE02120300000000202051".into()),
                creditor_id: Some("DE98ZZZ09999999999".into()),
            }),
        })
        .build()
        .unwrap()
}

#[test]
fn direct_debit_ubl_generation() {
    let inv = invoice_with_direct_debit();
    let xml = xrechnung::to_ubl_xml(&inv).unwrap();
    assert!(xml.contains("cac:PaymentMandate"));
    assert!(xml.contains("MANDATE-001"));
    assert!(xml.contains("DE02120300000000202051"));
}

#[test]
fn direct_debit_ubl_roundtrip() {
    let inv = invoice_with_direct_debit();
    let xml = xrechnung::to_ubl_xml(&inv).unwrap();
    let parsed = xrechnung::from_ubl_xml(&xml).unwrap();
    let dd = parsed.payment.unwrap().direct_debit.unwrap();
    assert_eq!(dd.mandate_id.as_deref(), Some("MANDATE-001"));
    assert_eq!(
        dd.debited_account_id.as_deref(),
        Some("DE02120300000000202051")
    );
}

#[test]
fn direct_debit_cii_generation() {
    let inv = invoice_with_direct_debit();
    let xml = xrechnung::to_cii_xml(&inv).unwrap();
    assert!(xml.contains("ram:DirectDebitMandateID"));
    assert!(xml.contains("MANDATE-001"));
    assert!(xml.contains("ram:PayerPartyDebtorFinancialAccount"));
}

#[test]
fn direct_debit_cii_roundtrip() {
    let inv = invoice_with_direct_debit();
    let xml = xrechnung::to_cii_xml(&inv).unwrap();
    let parsed = xrechnung::from_cii_xml(&xml).unwrap();
    let dd = parsed.payment.unwrap().direct_debit.unwrap();
    assert_eq!(dd.mandate_id.as_deref(), Some("MANDATE-001"));
    assert_eq!(
        dd.debited_account_id.as_deref(),
        Some("DE02120300000000202051")
    );
    assert_eq!(dd.creditor_id.as_deref(), Some("DE98ZZZ09999999999"));
}

// ---------------------------------------------------------------------------
// BT-82/BT-83: Payment means text and remittance info
// ---------------------------------------------------------------------------

#[test]
fn payment_means_text_ubl_roundtrip() {
    let inv = invoice_with_card_payment();
    let xml = xrechnung::to_ubl_xml(&inv).unwrap();
    let parsed = xrechnung::from_ubl_xml(&xml).unwrap();
    assert_eq!(
        parsed.payment.unwrap().means_text.as_deref(),
        Some("Visa ending 4242")
    );
}

#[test]
fn payment_means_text_cii_roundtrip() {
    let inv = invoice_with_card_payment();
    let xml = xrechnung::to_cii_xml(&inv).unwrap();
    let parsed = xrechnung::from_cii_xml(&xml).unwrap();
    assert_eq!(
        parsed.payment.unwrap().means_text.as_deref(),
        Some("Visa ending 4242")
    );
}

// ---------------------------------------------------------------------------
// BT-127: Line note tests
// ---------------------------------------------------------------------------

fn invoice_with_line_note() -> Invoice {
    InvoiceBuilder::new("RE-NOTE-001", date(2024, 6, 15))
        .tax_point_date(date(2024, 6, 15))
        .buyer_reference("04011000-12345-03")
        .seller(
            PartyBuilder::new(
                "Seller GmbH",
                AddressBuilder::new("Berlin", "10115", "DE").build(),
            )
            .vat_id("DE123456789")
            .electronic_address("EM", "seller@test.de")
            .contact(None, Some("+49 30 12345".into()), Some("s@test.de".into()))
            .build(),
        )
        .buyer(
            PartyBuilder::new(
                "Buyer AG",
                AddressBuilder::new("München", "80331", "DE").build(),
            )
            .electronic_address("EM", "buyer@test.de")
            .build(),
        )
        .add_line(
            LineItemBuilder::new("1", "Service", dec!(10), "HUR", dec!(100))
                .tax(TaxCategory::StandardRate, dec!(19))
                .note("Rush delivery surcharge applies")
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
            card_payment: None,
            direct_debit: None,
        })
        .build()
        .unwrap()
}

#[test]
fn line_note_ubl_roundtrip() {
    let inv = invoice_with_line_note();
    let xml = xrechnung::to_ubl_xml(&inv).unwrap();
    assert!(xml.contains("<cbc:Note>Rush delivery surcharge applies</cbc:Note>"));
    let parsed = xrechnung::from_ubl_xml(&xml).unwrap();
    assert_eq!(
        parsed.lines[0].note.as_deref(),
        Some("Rush delivery surcharge applies")
    );
}

#[test]
fn line_note_cii_roundtrip() {
    let inv = invoice_with_line_note();
    let xml = xrechnung::to_cii_xml(&inv).unwrap();
    assert!(xml.contains("ram:IncludedNote"));
    let parsed = xrechnung::from_cii_xml(&xml).unwrap();
    assert_eq!(
        parsed.lines[0].note.as_deref(),
        Some("Rush delivery surcharge applies")
    );
}

// ---------------------------------------------------------------------------
// BT-149/BT-150: Base quantity tests
// ---------------------------------------------------------------------------

fn invoice_with_base_quantity() -> Invoice {
    InvoiceBuilder::new("RE-BQ-001", date(2024, 6, 15))
        .tax_point_date(date(2024, 6, 15))
        .buyer_reference("04011000-12345-03")
        .seller(
            PartyBuilder::new(
                "Seller GmbH",
                AddressBuilder::new("Berlin", "10115", "DE").build(),
            )
            .vat_id("DE123456789")
            .electronic_address("EM", "seller@test.de")
            .contact(None, Some("+49 30 12345".into()), Some("s@test.de".into()))
            .build(),
        )
        .buyer(
            PartyBuilder::new(
                "Buyer AG",
                AddressBuilder::new("München", "80331", "DE").build(),
            )
            .electronic_address("EM", "buyer@test.de")
            .build(),
        )
        .add_line(
            LineItemBuilder::new("1", "Paint", dec!(5), "LTR", dec!(12.50))
                .tax(TaxCategory::StandardRate, dec!(19))
                .base_quantity(dec!(10), Some("LTR".into()))
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
            card_payment: None,
            direct_debit: None,
        })
        .build()
        .unwrap()
}

#[test]
fn base_quantity_ubl_roundtrip() {
    let inv = invoice_with_base_quantity();
    let xml = xrechnung::to_ubl_xml(&inv).unwrap();
    assert!(xml.contains("cbc:BaseQuantity"));
    let parsed = xrechnung::from_ubl_xml(&xml).unwrap();
    assert_eq!(parsed.lines[0].base_quantity, Some(dec!(10)));
    assert_eq!(parsed.lines[0].base_quantity_unit.as_deref(), Some("LTR"));
}

#[test]
fn base_quantity_cii_roundtrip() {
    let inv = invoice_with_base_quantity();
    let xml = xrechnung::to_cii_xml(&inv).unwrap();
    assert!(xml.contains("ram:BasisQuantity"));
    let parsed = xrechnung::from_cii_xml(&xml).unwrap();
    assert_eq!(parsed.lines[0].base_quantity, Some(dec!(10)));
    assert_eq!(parsed.lines[0].base_quantity_unit.as_deref(), Some("LTR"));
}

// ---------------------------------------------------------------------------
// BT-156: Buyer's item identifier tests
// ---------------------------------------------------------------------------

fn invoice_with_buyer_item_id() -> Invoice {
    InvoiceBuilder::new("RE-BII-001", date(2024, 6, 15))
        .tax_point_date(date(2024, 6, 15))
        .buyer_reference("04011000-12345-03")
        .seller(
            PartyBuilder::new(
                "Seller GmbH",
                AddressBuilder::new("Berlin", "10115", "DE").build(),
            )
            .vat_id("DE123456789")
            .electronic_address("EM", "seller@test.de")
            .contact(None, Some("+49 30 12345".into()), Some("s@test.de".into()))
            .build(),
        )
        .buyer(
            PartyBuilder::new(
                "Buyer AG",
                AddressBuilder::new("München", "80331", "DE").build(),
            )
            .electronic_address("EM", "buyer@test.de")
            .build(),
        )
        .add_line(
            LineItemBuilder::new("1", "Widget", dec!(5), "C62", dec!(10))
                .tax(TaxCategory::StandardRate, dec!(19))
                .buyer_item_id("BUYER-SKU-789")
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
            card_payment: None,
            direct_debit: None,
        })
        .build()
        .unwrap()
}

#[test]
fn buyer_item_id_ubl_roundtrip() {
    let inv = invoice_with_buyer_item_id();
    let xml = xrechnung::to_ubl_xml(&inv).unwrap();
    assert!(xml.contains("cac:BuyersItemIdentification"));
    assert!(xml.contains("BUYER-SKU-789"));
    let parsed = xrechnung::from_ubl_xml(&xml).unwrap();
    assert_eq!(
        parsed.lines[0].buyer_item_id.as_deref(),
        Some("BUYER-SKU-789")
    );
}

#[test]
fn buyer_item_id_cii_roundtrip() {
    let inv = invoice_with_buyer_item_id();
    let xml = xrechnung::to_cii_xml(&inv).unwrap();
    assert!(xml.contains("ram:BuyerAssignedID"));
    assert!(xml.contains("BUYER-SKU-789"));
    let parsed = xrechnung::from_cii_xml(&xml).unwrap();
    assert_eq!(
        parsed.lines[0].buyer_item_id.as_deref(),
        Some("BUYER-SKU-789")
    );
}

// ---------------------------------------------------------------------------
// BT-159: Item country of origin tests
// ---------------------------------------------------------------------------

fn invoice_with_origin_country() -> Invoice {
    InvoiceBuilder::new("RE-OC-001", date(2024, 6, 15))
        .tax_point_date(date(2024, 6, 15))
        .buyer_reference("04011000-12345-03")
        .seller(
            PartyBuilder::new(
                "Seller GmbH",
                AddressBuilder::new("Berlin", "10115", "DE").build(),
            )
            .vat_id("DE123456789")
            .electronic_address("EM", "seller@test.de")
            .contact(None, Some("+49 30 12345".into()), Some("s@test.de".into()))
            .build(),
        )
        .buyer(
            PartyBuilder::new(
                "Buyer AG",
                AddressBuilder::new("München", "80331", "DE").build(),
            )
            .electronic_address("EM", "buyer@test.de")
            .build(),
        )
        .add_line(
            LineItemBuilder::new("1", "Imported Wine", dec!(12), "C62", dec!(15))
                .tax(TaxCategory::StandardRate, dec!(19))
                .origin_country("FR")
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
            card_payment: None,
            direct_debit: None,
        })
        .build()
        .unwrap()
}

#[test]
fn origin_country_ubl_roundtrip() {
    let inv = invoice_with_origin_country();
    let xml = xrechnung::to_ubl_xml(&inv).unwrap();
    assert!(xml.contains("cac:OriginCountry"));
    assert!(xml.contains(">FR<"));
    let parsed = xrechnung::from_ubl_xml(&xml).unwrap();
    assert_eq!(parsed.lines[0].origin_country.as_deref(), Some("FR"));
}

#[test]
fn origin_country_cii_roundtrip() {
    let inv = invoice_with_origin_country();
    let xml = xrechnung::to_cii_xml(&inv).unwrap();
    assert!(xml.contains("ram:OriginTradeCountry"));
    let parsed = xrechnung::from_cii_xml(&xml).unwrap();
    assert_eq!(parsed.lines[0].origin_country.as_deref(), Some("FR"));
}

// ---------------------------------------------------------------------------
// Seller subdivision roundtrip
// ---------------------------------------------------------------------------

fn invoice_with_seller_subdivision() -> Invoice {
    InvoiceBuilder::new("RE-SUB-001", date(2024, 6, 15))
        .tax_point_date(date(2024, 6, 15))
        .buyer_reference("04011000-12345-03")
        .seller(
            PartyBuilder::new(
                "Seller GmbH",
                AddressBuilder::new("Berlin", "10115", "DE")
                    .street("Friedrichstraße 123")
                    .subdivision("Berlin")
                    .build(),
            )
            .vat_id("DE123456789")
            .electronic_address("EM", "seller@test.de")
            .contact(None, Some("+49 30 12345".into()), Some("s@test.de".into()))
            .build(),
        )
        .buyer(
            PartyBuilder::new(
                "Buyer AG",
                AddressBuilder::new("München", "80331", "DE").build(),
            )
            .electronic_address("EM", "buyer@test.de")
            .build(),
        )
        .add_line(
            LineItemBuilder::new("1", "Service", dec!(1), "C62", dec!(100))
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
            card_payment: None,
            direct_debit: None,
        })
        .build()
        .unwrap()
}

#[test]
fn seller_subdivision_ubl_roundtrip() {
    let inv = invoice_with_seller_subdivision();
    let xml = xrechnung::to_ubl_xml(&inv).unwrap();
    let parsed = xrechnung::from_ubl_xml(&xml).unwrap();
    assert_eq!(parsed.seller.address.subdivision.as_deref(), Some("Berlin"));
}

#[test]
fn seller_subdivision_cii_roundtrip() {
    let inv = invoice_with_seller_subdivision();
    let xml = xrechnung::to_cii_xml(&inv).unwrap();
    let parsed = xrechnung::from_cii_xml(&xml).unwrap();
    assert_eq!(parsed.seller.address.subdivision.as_deref(), Some("Berlin"));
}

// ---------------------------------------------------------------------------
// Standard item ID (BT-157) CII roundtrip
// ---------------------------------------------------------------------------

#[test]
fn standard_item_id_cii_roundtrip() {
    let inv = InvoiceBuilder::new("RE-SII-001", date(2024, 6, 15))
        .tax_point_date(date(2024, 6, 15))
        .buyer_reference("04011000-12345-03")
        .seller(
            PartyBuilder::new(
                "Seller GmbH",
                AddressBuilder::new("Berlin", "10115", "DE").build(),
            )
            .vat_id("DE123456789")
            .electronic_address("EM", "seller@test.de")
            .contact(None, Some("+49 30 12345".into()), Some("s@test.de".into()))
            .build(),
        )
        .buyer(
            PartyBuilder::new(
                "Buyer AG",
                AddressBuilder::new("München", "80331", "DE").build(),
            )
            .electronic_address("EM", "buyer@test.de")
            .build(),
        )
        .add_line(
            LineItemBuilder::new("1", "Widget", dec!(5), "C62", dec!(10))
                .tax(TaxCategory::StandardRate, dec!(19))
                .standard_item_id("1234567890123")
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
            card_payment: None,
            direct_debit: None,
        })
        .build()
        .unwrap();
    let xml = xrechnung::to_cii_xml(&inv).unwrap();
    assert!(xml.contains("ram:GlobalID"));
    assert!(xml.contains("1234567890123"));
    let parsed = xrechnung::from_cii_xml(&xml).unwrap();
    assert_eq!(
        parsed.lines[0].standard_item_id.as_deref(),
        Some("1234567890123")
    );
}

// ---------------------------------------------------------------------------
// Buyer contact/reg_id CII roundtrip
// ---------------------------------------------------------------------------

#[test]
fn buyer_contact_cii_roundtrip() {
    let inv = InvoiceBuilder::new("RE-BC-001", date(2024, 6, 15))
        .tax_point_date(date(2024, 6, 15))
        .buyer_reference("04011000-12345-03")
        .seller(
            PartyBuilder::new(
                "Seller GmbH",
                AddressBuilder::new("Berlin", "10115", "DE").build(),
            )
            .vat_id("DE123456789")
            .electronic_address("EM", "seller@test.de")
            .contact(None, Some("+49 30 12345".into()), Some("s@test.de".into()))
            .build(),
        )
        .buyer(
            PartyBuilder::new(
                "Buyer AG",
                AddressBuilder::new("München", "80331", "DE").build(),
            )
            .electronic_address("EM", "buyer@test.de")
            .registration_id("HRB 12345")
            .trading_name("Buyer Trading")
            .contact(
                Some("Anna Schmidt".into()),
                Some("+49 89 54321".into()),
                Some("anna@buyer.de".into()),
            )
            .build(),
        )
        .add_line(
            LineItemBuilder::new("1", "Service", dec!(1), "C62", dec!(100))
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
            card_payment: None,
            direct_debit: None,
        })
        .build()
        .unwrap();
    let xml = xrechnung::to_cii_xml(&inv).unwrap();
    let parsed = xrechnung::from_cii_xml(&xml).unwrap();
    assert_eq!(parsed.buyer.trading_name.as_deref(), Some("Buyer Trading"));
    assert_eq!(parsed.buyer.registration_id.as_deref(), Some("HRB 12345"));
    let contact = parsed.buyer.contact.unwrap();
    assert_eq!(contact.name.as_deref(), Some("Anna Schmidt"));
    assert_eq!(contact.phone.as_deref(), Some("+49 89 54321"));
    assert_eq!(contact.email.as_deref(), Some("anna@buyer.de"));
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

// ===== Tax representative exemption (BR-CO-06) =====

#[test]
fn tax_representative_exempts_seller_vat_requirement() {
    // Seller has no VAT ID or tax number, but tax representative is set.
    // This should be valid per BR-CO-06.
    let inv = InvoiceBuilder::new("TR-001", date(2024, 6, 15))
        .tax_point_date(date(2024, 6, 15))
        .buyer_reference("04011000-12345-03")
        .seller(
            PartyBuilder::new(
                "Foreign Co",
                AddressBuilder::new("Paris", "75001", "FR").build(),
            )
            .electronic_address("EM", "seller@foreign.com")
            .contact(
                Some("Jean".into()),
                Some("+33 1 234".into()),
                Some("j@foreign.com".into()),
            )
            .build(),
        )
        .buyer(
            PartyBuilder::new(
                "Kunde AG",
                AddressBuilder::new("Berlin", "10115", "DE").build(),
            )
            .electronic_address("EM", "buyer@kunde.de")
            .build(),
        )
        .tax_representative(TaxRepresentative {
            name: "Steuerberater GmbH".into(),
            vat_id: "DE987654321".into(),
            address: AddressBuilder::new("Berlin", "10115", "DE").build(),
        })
        .add_line(
            LineItemBuilder::new("1", "Service", dec!(1), "C62", dec!(1000))
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
            card_payment: None,
            direct_debit: None,
        })
        .build();

    assert!(inv.is_ok(), "expected OK, got: {:?}", inv.unwrap_err());
}

// ===== Duplicate line ID validation (BR-CO-04) =====

#[test]
fn validate_en16931_catches_duplicate_line_ids() {
    use faktura::core::validate_en16931;

    let inv = InvoiceBuilder::new("DUP-001", date(2024, 6, 15))
        .tax_point_date(date(2024, 6, 15))
        .seller(
            PartyBuilder::new(
                "ACME GmbH",
                AddressBuilder::new("Berlin", "10115", "DE").build(),
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
            LineItemBuilder::new("1", "Item A", dec!(1), "C62", dec!(100))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .add_line(
            LineItemBuilder::new("1", "Item B", dec!(2), "C62", dec!(200))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .build()
        .unwrap();

    let errors = validate_en16931(&inv);
    assert!(
        errors.iter().any(|e| e.rule.as_deref() == Some("BR-CO-04")),
        "expected BR-CO-04 error for duplicate line IDs, got: {:?}",
        errors
    );
}

// ===== ZeroRated e2e test =====

#[test]
fn zero_rated_invoice_ubl_roundtrip() {
    let inv = InvoiceBuilder::new("ZR-001", date(2024, 6, 15))
        .tax_point_date(date(2024, 6, 15))
        .buyer_reference("04011000-12345-03")
        .vat_scenario(VatScenario::Mixed)
        .seller(
            PartyBuilder::new(
                "ACME GmbH",
                AddressBuilder::new("Berlin", "10115", "DE").build(),
            )
            .vat_id("DE123456789")
            .electronic_address("EM", "seller@acme.de")
            .contact(
                Some("Max".into()),
                Some("+49 30 123".into()),
                Some("max@acme.de".into()),
            )
            .build(),
        )
        .buyer(
            PartyBuilder::new(
                "Kunde AG",
                AddressBuilder::new("München", "80331", "DE").build(),
            )
            .electronic_address("EM", "buyer@kunde.de")
            .build(),
        )
        .add_line(
            LineItemBuilder::new("1", "Zero-rated item", dec!(5), "C62", dec!(100))
                .tax(TaxCategory::ZeroRated, dec!(0))
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
            card_payment: None,
            direct_debit: None,
        })
        .build()
        .unwrap();

    let xml = xrechnung::to_ubl_xml(&inv).unwrap();
    let parsed = xrechnung::from_ubl_xml(&xml).unwrap();
    assert_eq!(parsed.lines[0].tax_category, TaxCategory::ZeroRated);
    assert_eq!(parsed.lines[0].tax_rate, dec!(0));
    assert_eq!(parsed.totals.unwrap().vat_total, dec!(0));
}

// ===== CII buyer Steuernummer roundtrip =====

#[test]
fn cii_buyer_tax_number_roundtrip() {
    let inv = InvoiceBuilder::new("BTN-001", date(2024, 6, 15))
        .tax_point_date(date(2024, 6, 15))
        .buyer_reference("04011000-12345-03")
        .seller(
            PartyBuilder::new(
                "ACME GmbH",
                AddressBuilder::new("Berlin", "10115", "DE").build(),
            )
            .vat_id("DE123456789")
            .electronic_address("EM", "seller@acme.de")
            .contact(
                Some("Max".into()),
                Some("+49 30 123".into()),
                Some("max@acme.de".into()),
            )
            .tax_number("1234567890")
            .build(),
        )
        .buyer(
            PartyBuilder::new(
                "Kunde AG",
                AddressBuilder::new("München", "80331", "DE").build(),
            )
            .electronic_address("EM", "buyer@kunde.de")
            .build(),
        )
        .add_line(
            LineItemBuilder::new("1", "Service", dec!(1), "C62", dec!(100))
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
            card_payment: None,
            direct_debit: None,
        })
        .build()
        .unwrap();

    // CII roundtrip
    let xml = xrechnung::to_cii_xml(&inv).unwrap();
    let parsed = xrechnung::from_cii_xml(&xml).unwrap();
    assert_eq!(
        parsed.seller.tax_number.as_deref(),
        Some("1234567890"),
        "seller tax_number should roundtrip through CII"
    );
}

// ===== UBL creditor_id roundtrip =====

#[test]
fn ubl_creditor_id_roundtrip() {
    let inv = InvoiceBuilder::new("DD-002", date(2024, 6, 15))
        .tax_point_date(date(2024, 6, 15))
        .buyer_reference("04011000-12345-03")
        .seller(
            PartyBuilder::new(
                "ACME GmbH",
                AddressBuilder::new("Berlin", "10115", "DE").build(),
            )
            .vat_id("DE123456789")
            .electronic_address("EM", "seller@acme.de")
            .contact(
                Some("Max".into()),
                Some("+49 30 123".into()),
                Some("max@acme.de".into()),
            )
            .build(),
        )
        .buyer(
            PartyBuilder::new(
                "Kunde AG",
                AddressBuilder::new("München", "80331", "DE").build(),
            )
            .electronic_address("EM", "buyer@kunde.de")
            .build(),
        )
        .add_line(
            LineItemBuilder::new("1", "Service", dec!(1), "C62", dec!(100))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .payment(PaymentInstructions {
            means_code: PaymentMeansCode::SepaDirectDebit,
            means_text: None,
            remittance_info: Some("DD-002".into()),
            credit_transfer: None,
            card_payment: None,
            direct_debit: Some(DirectDebit {
                mandate_id: Some("MANDATE-123".into()),
                creditor_id: Some("DE98ZZZ09999999999".into()),
                debited_account_id: Some("DE89370400440532013000".into()),
            }),
        })
        .build()
        .unwrap();

    let xml = xrechnung::to_ubl_xml(&inv).unwrap();
    let parsed = xrechnung::from_ubl_xml(&xml).unwrap();
    let dd = parsed.payment.unwrap().direct_debit.unwrap();
    assert_eq!(dd.creditor_id.as_deref(), Some("DE98ZZZ09999999999"));
    assert_eq!(dd.mandate_id.as_deref(), Some("MANDATE-123"));
    assert_eq!(
        dd.debited_account_id.as_deref(),
        Some("DE89370400440532013000")
    );
}

// ===== New BT references roundtrip =====

#[test]
fn contract_project_sales_order_references_ubl_roundtrip() {
    let inv = InvoiceBuilder::new("REF-001", date(2024, 6, 15))
        .tax_point_date(date(2024, 6, 15))
        .buyer_reference("04011000-12345-03")
        .contract_reference("CONTRACT-2024-42")
        .project_reference("PROJECT-ALPHA")
        .order_reference("PO-2024-100")
        .sales_order_reference("SO-2024-200")
        .buyer_accounting_reference("COST-CENTER-99")
        .seller(
            PartyBuilder::new(
                "ACME GmbH",
                AddressBuilder::new("Berlin", "10115", "DE").build(),
            )
            .vat_id("DE123456789")
            .electronic_address("EM", "seller@acme.de")
            .contact(
                Some("Max".into()),
                Some("+49 30 123".into()),
                Some("max@acme.de".into()),
            )
            .build(),
        )
        .buyer(
            PartyBuilder::new(
                "Kunde AG",
                AddressBuilder::new("München", "80331", "DE").build(),
            )
            .electronic_address("EM", "buyer@kunde.de")
            .build(),
        )
        .add_line(
            LineItemBuilder::new("1", "Service", dec!(1), "C62", dec!(100))
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
            card_payment: None,
            direct_debit: None,
        })
        .build()
        .unwrap();

    // UBL roundtrip
    let xml = xrechnung::to_ubl_xml(&inv).unwrap();
    let parsed = xrechnung::from_ubl_xml(&xml).unwrap();
    assert_eq!(
        parsed.contract_reference.as_deref(),
        Some("CONTRACT-2024-42")
    );
    assert_eq!(parsed.project_reference.as_deref(), Some("PROJECT-ALPHA"));
    assert_eq!(parsed.order_reference.as_deref(), Some("PO-2024-100"));
    assert_eq!(parsed.sales_order_reference.as_deref(), Some("SO-2024-200"));
    assert_eq!(
        parsed.buyer_accounting_reference.as_deref(),
        Some("COST-CENTER-99")
    );
}

#[test]
fn contract_project_sales_order_references_cii_roundtrip() {
    let inv = InvoiceBuilder::new("REF-002", date(2024, 6, 15))
        .tax_point_date(date(2024, 6, 15))
        .buyer_reference("04011000-12345-03")
        .contract_reference("CONTRACT-2024-42")
        .project_reference("PROJECT-ALPHA")
        .order_reference("PO-2024-100")
        .sales_order_reference("SO-2024-200")
        .buyer_accounting_reference("COST-CENTER-99")
        .seller(
            PartyBuilder::new(
                "ACME GmbH",
                AddressBuilder::new("Berlin", "10115", "DE").build(),
            )
            .vat_id("DE123456789")
            .electronic_address("EM", "seller@acme.de")
            .contact(
                Some("Max".into()),
                Some("+49 30 123".into()),
                Some("max@acme.de".into()),
            )
            .build(),
        )
        .buyer(
            PartyBuilder::new(
                "Kunde AG",
                AddressBuilder::new("München", "80331", "DE").build(),
            )
            .electronic_address("EM", "buyer@kunde.de")
            .build(),
        )
        .add_line(
            LineItemBuilder::new("1", "Service", dec!(1), "C62", dec!(100))
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
            card_payment: None,
            direct_debit: None,
        })
        .build()
        .unwrap();

    // CII roundtrip
    let xml = xrechnung::to_cii_xml(&inv).unwrap();
    let parsed = xrechnung::from_cii_xml(&xml).unwrap();
    assert_eq!(
        parsed.contract_reference.as_deref(),
        Some("CONTRACT-2024-42")
    );
    assert_eq!(parsed.project_reference.as_deref(), Some("PROJECT-ALPHA"));
    assert_eq!(parsed.order_reference.as_deref(), Some("PO-2024-100"));
    assert_eq!(parsed.sales_order_reference.as_deref(), Some("SO-2024-200"));
    assert_eq!(
        parsed.buyer_accounting_reference.as_deref(),
        Some("COST-CENTER-99")
    );
}

// ---------- from_xml() auto-detect ----------

#[test]
fn from_xml_auto_detects_ubl() {
    let inv = xrechnung_invoice();
    let ubl_xml = xrechnung::to_ubl_xml(&inv).unwrap();
    let (parsed, syntax) = xrechnung::from_xml(&ubl_xml).unwrap();
    assert_eq!(syntax, xrechnung::XmlSyntax::Ubl);
    assert_eq!(parsed.number, inv.number);
}

#[test]
fn from_xml_auto_detects_cii() {
    let inv = xrechnung_invoice();
    let cii_xml = xrechnung::to_cii_xml(&inv).unwrap();
    let (parsed, syntax) = xrechnung::from_xml(&cii_xml).unwrap();
    assert_eq!(syntax, xrechnung::XmlSyntax::Cii);
    assert_eq!(parsed.number, inv.number);
}

#[test]
fn from_xml_rejects_unknown_root() {
    let xml = r#"<?xml version="1.0"?><SomeOtherDocument/>"#;
    let result = xrechnung::from_xml(xml);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("cannot detect"));
}

// ---------- build_strict() ----------

#[test]
fn build_strict_passes_valid_invoice() {
    let inv = InvoiceBuilder::new("STRICT-001", date(2024, 6, 15))
        .due_date(date(2024, 7, 15))
        .tax_point_date(date(2024, 6, 15))
        .seller(
            PartyBuilder::new(
                "ACME GmbH",
                AddressBuilder::new("Berlin", "10115", "DE").build(),
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
            LineItemBuilder::new("1", "Beratung", dec!(10), "HUR", dec!(150))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .build_strict();

    assert!(inv.is_ok(), "build_strict should pass for valid invoice");
}

#[test]
fn build_strict_rejects_duplicate_line_ids() {
    let result = InvoiceBuilder::new("STRICT-002", date(2024, 6, 15))
        .tax_point_date(date(2024, 6, 15))
        .seller(
            PartyBuilder::new(
                "ACME GmbH",
                AddressBuilder::new("Berlin", "10115", "DE").build(),
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
            LineItemBuilder::new("1", "Item A", dec!(1), "C62", dec!(100))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .add_line(
            LineItemBuilder::new("1", "Item B", dec!(2), "C62", dec!(200))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .build_strict();

    assert!(
        result.is_err(),
        "build_strict should reject duplicate line IDs"
    );
    assert!(result.unwrap_err().to_string().contains("duplicate"));
}

// ---------- unit code validation ----------

#[test]
fn build_strict_rejects_unknown_unit_code() {
    let result = InvoiceBuilder::new("UNIT-001", date(2024, 6, 15))
        .tax_point_date(date(2024, 6, 15))
        .seller(
            PartyBuilder::new(
                "ACME GmbH",
                AddressBuilder::new("Berlin", "10115", "DE").build(),
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
            LineItemBuilder::new("1", "Widget", dec!(5), "PIECE", dec!(10))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .build_strict();

    assert!(
        result.is_err(),
        "build_strict should reject unknown unit code"
    );
    assert!(result.unwrap_err().to_string().contains("PIECE"));
}

#[test]
fn known_unit_codes_pass_validation() {
    assert!(faktura::core::is_known_unit_code("C62"));
    assert!(faktura::core::is_known_unit_code("HUR"));
    assert!(faktura::core::is_known_unit_code("KGM"));
    assert!(!faktura::core::is_known_unit_code("INVALID"));
    assert!(!faktura::core::is_known_unit_code(""));
}
