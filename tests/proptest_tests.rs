//! Property-based tests and edge case tests for the faktura crate.
//!
//! Run with: `cargo test --features all --test proptest_tests`

#![cfg(feature = "xrechnung")]

use chrono::NaiveDate;
use faktura::core::*;
use proptest::prelude::*;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

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
    .electronic_address("EM", "billing@acme.de")
    .contact(
        Some("Max Mustermann".into()),
        Some("+49 30 12345678".into()),
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
    .electronic_address("EM", "rechnung@kunde.de")
    .build()
}

fn payment() -> PaymentInstructions {
    PaymentInstructions {
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
    }
}

/// Build a valid domestic invoice with the given lines.
fn build_domestic(lines: Vec<LineItem>) -> Invoice {
    let mut builder = InvoiceBuilder::new("RE-2024-PROP", date(2024, 6, 15))
        .buyer_reference("BUYER-REF")
        .due_date(date(2024, 7, 15))
        .tax_point_date(date(2024, 6, 15))
        .seller(seller())
        .buyer(buyer())
        .payment(payment());
    for line in lines {
        builder = builder.add_line(line);
    }
    builder.build().unwrap()
}

// ── Proptest Strategies ─────────────────────────────────────────────────────

/// Generate a reasonable price (0.01 to 99999.99).
fn arb_price() -> impl Strategy<Value = Decimal> {
    (1u64..10_000_000u64).prop_map(|cents| Decimal::new(cents as i64, 2))
}

/// Generate a reasonable quantity (1 to 100).
fn arb_quantity() -> impl Strategy<Value = Decimal> {
    (1u32..=100u32).prop_map(Decimal::from)
}

/// Generate a valid (tax_category, tax_rate) pair.
/// BR-S-05 requires non-zero rate for StandardRate, so 0% uses ZeroRatedGoods.
fn arb_tax() -> impl Strategy<Value = (TaxCategory, Decimal)> {
    prop_oneof![
        Just((TaxCategory::ZeroRated, dec!(0))),
        Just((TaxCategory::StandardRate, dec!(7))),
        Just((TaxCategory::StandardRate, dec!(19))),
    ]
}

/// Generate a valid line item.
fn arb_line(idx: usize) -> impl Strategy<Value = LineItem> {
    (arb_quantity(), arb_price(), arb_tax()).prop_map(move |(qty, price, (cat, rate))| {
        LineItemBuilder::new(
            format!("{}", idx + 1),
            format!("Item {}", idx + 1),
            qty,
            "C62",
            price,
        )
        .tax(cat, rate)
        .build()
    })
}

/// Generate 1-5 valid line items.
fn arb_lines() -> impl Strategy<Value = Vec<LineItem>> {
    prop::collection::vec(arb_line(0), 1..=5).prop_map(|mut lines| {
        for (i, line) in lines.iter_mut().enumerate() {
            line.id = format!("{}", i + 1);
            line.item_name = format!("Item {}", i + 1);
        }
        lines
    })
}

// ── Property Tests ──────────────────────────────────────────────────────────

proptest! {
    /// build() → to_ubl_xml() → from_ubl_xml() preserves key fields.
    #[test]
    fn ubl_roundtrip_preserves_fields(lines in arb_lines()) {
        let inv = build_domestic(lines);
        let xml = faktura::xrechnung::to_ubl_xml(&inv).unwrap();
        let parsed = faktura::xrechnung::from_ubl_xml(&xml).unwrap();

        prop_assert_eq!(&parsed.number, &inv.number);
        prop_assert_eq!(parsed.issue_date, inv.issue_date);
        prop_assert_eq!(parsed.type_code, inv.type_code);
        prop_assert_eq!(&parsed.currency_code, &inv.currency_code);
        prop_assert_eq!(parsed.lines.len(), inv.lines.len());
        prop_assert_eq!(&parsed.seller.name, &inv.seller.name);
        prop_assert_eq!(&parsed.buyer.name, &inv.buyer.name);

        // Verify totals match
        let orig_totals = inv.totals.as_ref().unwrap();
        let parsed_totals = parsed.totals.as_ref().unwrap();
        prop_assert_eq!(parsed_totals.line_net_total, orig_totals.line_net_total);
        prop_assert_eq!(parsed_totals.vat_total, orig_totals.vat_total);
        prop_assert_eq!(parsed_totals.gross_total, orig_totals.gross_total);
        prop_assert_eq!(parsed_totals.amount_due, orig_totals.amount_due);
    }

    /// build() → to_cii_xml() → from_cii_xml() preserves key fields.
    #[test]
    fn cii_roundtrip_preserves_fields(lines in arb_lines()) {
        let inv = build_domestic(lines);
        let xml = faktura::xrechnung::to_cii_xml(&inv).unwrap();
        let parsed = faktura::xrechnung::from_cii_xml(&xml).unwrap();

        prop_assert_eq!(&parsed.number, &inv.number);
        prop_assert_eq!(parsed.issue_date, inv.issue_date);
        prop_assert_eq!(parsed.type_code, inv.type_code);
        prop_assert_eq!(&parsed.currency_code, &inv.currency_code);
        prop_assert_eq!(parsed.lines.len(), inv.lines.len());
        prop_assert_eq!(&parsed.seller.name, &inv.seller.name);
        prop_assert_eq!(&parsed.buyer.name, &inv.buyer.name);

        let orig_totals = inv.totals.as_ref().unwrap();
        let parsed_totals = parsed.totals.as_ref().unwrap();
        prop_assert_eq!(parsed_totals.line_net_total, orig_totals.line_net_total);
        prop_assert_eq!(parsed_totals.vat_total, orig_totals.vat_total);
        prop_assert_eq!(parsed_totals.gross_total, orig_totals.gross_total);
        prop_assert_eq!(parsed_totals.amount_due, orig_totals.amount_due);
    }

    /// calculate_totals() output always satisfies validate_arithmetic().
    #[test]
    fn totals_satisfy_arithmetic(lines in arb_lines()) {
        let inv = build_domestic(lines);
        let errors = faktura::core::validate_arithmetic(&inv);
        prop_assert!(errors.is_empty(), "arithmetic errors: {:?}", errors);
    }

    /// Any invoice passing validate_14_ustg() has all mandatory fields non-empty.
    #[test]
    fn ustg_valid_implies_fields_present(lines in arb_lines()) {
        let inv = build_domestic(lines);
        let errors = faktura::core::validate_14_ustg(&inv);
        if errors.is_empty() {
            prop_assert!(!inv.number.trim().is_empty());
            prop_assert!(!inv.seller.name.trim().is_empty());
            prop_assert!(!inv.buyer.name.trim().is_empty());
            prop_assert!(!inv.lines.is_empty());
            for line in &inv.lines {
                prop_assert!(!line.item_name.trim().is_empty());
            }
        }
    }
}

// ── Edge Case Tests ─────────────────────────────────────────────────────────

// --- Unicode names ---

#[test]
fn unicode_seller_buyer_names() {
    let scenarios = [
        ("日本語会社", "東京株式会社"),        // CJK
        ("Ünternehmen GmbH", "Kundé & Söhne"), // Umlauts
        ("شركة عربية", "عميل عربي"),           // RTL Arabic
        ("Compañía S.L.", "José García"),      // Spanish
        ("Ça va Cie", "François Müller"),      // French + combining
    ];

    for (seller_name, buyer_name) in scenarios {
        let inv = InvoiceBuilder::new("RE-UNICODE", date(2024, 6, 15))
            .buyer_reference("BR-UNI")
            .tax_point_date(date(2024, 6, 15))
            .seller(
                PartyBuilder::new(
                    seller_name,
                    AddressBuilder::new("Berlin", "10115", "DE")
                        .street("Straße 1")
                        .build(),
                )
                .vat_id("DE123456789")
                .electronic_address("EM", "billing@test.de")
                .build(),
            )
            .buyer(
                PartyBuilder::new(
                    buyer_name,
                    AddressBuilder::new("München", "80331", "DE")
                        .street("Straße 2")
                        .build(),
                )
                .electronic_address("EM", "buyer@test.de")
                .build(),
            )
            .add_line(
                LineItemBuilder::new("1", "Service", dec!(1), "C62", dec!(100))
                    .tax(TaxCategory::StandardRate, dec!(19))
                    .build(),
            )
            .payment(payment())
            .build()
            .unwrap();

        // UBL roundtrip
        let ubl = faktura::xrechnung::to_ubl_xml(&inv).unwrap();
        let parsed = faktura::xrechnung::from_ubl_xml(&ubl).unwrap();
        assert_eq!(
            parsed.seller.name, seller_name,
            "UBL seller name mismatch for {seller_name}"
        );
        assert_eq!(
            parsed.buyer.name, buyer_name,
            "UBL buyer name mismatch for {buyer_name}"
        );

        // CII roundtrip
        let cii = faktura::xrechnung::to_cii_xml(&inv).unwrap();
        let parsed = faktura::xrechnung::from_cii_xml(&cii).unwrap();
        assert_eq!(
            parsed.seller.name, seller_name,
            "CII seller name mismatch for {seller_name}"
        );
        assert_eq!(
            parsed.buyer.name, buyer_name,
            "CII buyer name mismatch for {buyer_name}"
        );
    }
}

// --- Max-length strings ---

#[test]
fn long_invoice_number() {
    let long_number = "R".repeat(200);
    let inv = InvoiceBuilder::new(&long_number, date(2024, 6, 15))
        .buyer_reference("BR-LONG")
        .tax_point_date(date(2024, 6, 15))
        .seller(seller())
        .buyer(buyer())
        .add_line(
            LineItemBuilder::new("1", "Test", dec!(1), "C62", dec!(100))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .payment(payment())
        .build()
        .unwrap();

    let ubl = faktura::xrechnung::to_ubl_xml(&inv).unwrap();
    let parsed = faktura::xrechnung::from_ubl_xml(&ubl).unwrap();
    assert_eq!(parsed.number, long_number);
}

#[test]
fn many_line_items() {
    let mut builder = InvoiceBuilder::new("RE-MANY-LINES", date(2024, 6, 15))
        .buyer_reference("BR-MANY")
        .tax_point_date(date(2024, 6, 15))
        .seller(seller())
        .buyer(buyer())
        .payment(payment());

    for i in 1..=100 {
        builder = builder.add_line(
            LineItemBuilder::new(
                format!("{i}"),
                format!("Item {i}"),
                dec!(1),
                "C62",
                dec!(10),
            )
            .tax(TaxCategory::StandardRate, dec!(19))
            .build(),
        );
    }

    let inv = builder.build().unwrap();
    assert_eq!(inv.lines.len(), 100);

    let ubl = faktura::xrechnung::to_ubl_xml(&inv).unwrap();
    let parsed = faktura::xrechnung::from_ubl_xml(&ubl).unwrap();
    assert_eq!(parsed.lines.len(), 100);

    let cii = faktura::xrechnung::to_cii_xml(&inv).unwrap();
    let parsed = faktura::xrechnung::from_cii_xml(&cii).unwrap();
    assert_eq!(parsed.lines.len(), 100);
}

// --- Zero-amount and boundary values ---

#[test]
fn zero_amount_invoice() {
    let inv = InvoiceBuilder::new("RE-ZERO", date(2024, 6, 15))
        .buyer_reference("BR-ZERO")
        .tax_point_date(date(2024, 6, 15))
        .seller(seller())
        .buyer(buyer())
        .add_line(
            LineItemBuilder::new("1", "Free sample", dec!(1), "C62", dec!(0))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .payment(payment())
        .build()
        .unwrap();

    let totals = inv.totals.as_ref().unwrap();
    assert_eq!(totals.line_net_total, dec!(0));
    assert_eq!(totals.vat_total, dec!(0));
    assert_eq!(totals.amount_due, dec!(0));

    // Roundtrip
    let ubl = faktura::xrechnung::to_ubl_xml(&inv).unwrap();
    let parsed = faktura::xrechnung::from_ubl_xml(&ubl).unwrap();
    assert_eq!(parsed.totals.as_ref().unwrap().amount_due, dec!(0));
}

#[test]
fn large_decimal_values() {
    let inv = InvoiceBuilder::new("RE-BIG", date(2024, 6, 15))
        .buyer_reference("BR-BIG")
        .tax_point_date(date(2024, 6, 15))
        .seller(seller())
        .buyer(buyer())
        .add_line(
            LineItemBuilder::new("1", "Enterprise License", dec!(1), "C62", dec!(999999.99))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .payment(payment())
        .build()
        .unwrap();

    let totals = inv.totals.as_ref().unwrap();
    assert_eq!(totals.line_net_total, dec!(999999.99));
    assert_eq!(totals.vat_total, dec!(190000.00)); // 999999.99 * 0.19 = 189999.9981 → 190000.00
    assert_eq!(totals.gross_total, dec!(1189999.99));

    let ubl = faktura::xrechnung::to_ubl_xml(&inv).unwrap();
    let parsed = faktura::xrechnung::from_ubl_xml(&ubl).unwrap();
    assert_eq!(
        parsed.totals.as_ref().unwrap().gross_total,
        dec!(1189999.99)
    );
}

#[test]
fn prepaid_exceeds_total() {
    let inv = InvoiceBuilder::new("RE-PREPAID", date(2024, 6, 15))
        .buyer_reference("BR-PREP")
        .tax_point_date(date(2024, 6, 15))
        .seller(seller())
        .buyer(buyer())
        .add_line(
            LineItemBuilder::new("1", "Service", dec!(1), "C62", dec!(100))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .prepaid(dec!(200))
        .payment(payment())
        .build()
        .unwrap();

    let totals = inv.totals.as_ref().unwrap();
    assert_eq!(totals.gross_total, dec!(119));
    assert_eq!(totals.prepaid, dec!(200));
    assert_eq!(totals.amount_due, dec!(-81)); // negative = overpayment

    // Arithmetic should still be valid
    let errors = validate_arithmetic(&inv);
    assert!(errors.is_empty(), "arithmetic errors: {:?}", errors);
}

// --- All payment means codes ---

#[test]
fn all_payment_means_codes() {
    let codes = [
        PaymentMeansCode::Cash,
        PaymentMeansCode::CreditTransfer,
        PaymentMeansCode::PaymentToBankAccount,
        PaymentMeansCode::BankCard,
        PaymentMeansCode::DirectDebit,
        PaymentMeansCode::StandingAgreement,
        PaymentMeansCode::SepaCreditTransfer,
        PaymentMeansCode::SepaDirectDebit,
    ];

    for code in codes {
        let mut pay = payment();
        pay.means_code = code;

        // Add appropriate payment details
        match code {
            PaymentMeansCode::BankCard => {
                pay.credit_transfer = None;
                pay.card_payment = Some(CardPayment {
                    account_number: "4111111111111111".into(),
                    holder_name: Some("Max Mustermann".into()),
                });
            }
            PaymentMeansCode::DirectDebit | PaymentMeansCode::SepaDirectDebit => {
                pay.credit_transfer = None;
                pay.direct_debit = Some(DirectDebit {
                    mandate_id: Some("MANDATE-001".into()),
                    creditor_id: Some("DE98ZZZ09999999999".into()),
                    debited_account_id: Some("DE89370400440532013000".into()),
                });
            }
            _ => {}
        }

        let inv = InvoiceBuilder::new(format!("RE-PAY-{}", code.code()), date(2024, 6, 15))
            .buyer_reference("BR-PAY")
            .tax_point_date(date(2024, 6, 15))
            .seller(seller())
            .buyer(buyer())
            .add_line(
                LineItemBuilder::new("1", "Service", dec!(1), "C62", dec!(100))
                    .tax(TaxCategory::StandardRate, dec!(19))
                    .build(),
            )
            .payment(pay)
            .build()
            .unwrap();

        // UBL roundtrip
        let ubl = faktura::xrechnung::to_ubl_xml(&inv).unwrap();
        let parsed = faktura::xrechnung::from_ubl_xml(&ubl).unwrap();
        assert_eq!(
            parsed.payment.as_ref().unwrap().means_code,
            code,
            "payment means code mismatch for {:?}",
            code
        );

        // CII roundtrip
        let cii = faktura::xrechnung::to_cii_xml(&inv).unwrap();
        let parsed = faktura::xrechnung::from_cii_xml(&cii).unwrap();
        assert_eq!(
            parsed.payment.as_ref().unwrap().means_code,
            code,
            "CII payment means code mismatch for {:?}",
            code
        );
    }
}

// --- All invoice type codes through roundtrip ---

#[test]
fn all_invoice_type_codes_roundtrip() {
    let codes = [
        InvoiceTypeCode::Invoice,
        InvoiceTypeCode::CreditNote,
        InvoiceTypeCode::Corrected,
        InvoiceTypeCode::Prepayment,
        InvoiceTypeCode::Partial,
    ];

    for type_code in codes {
        let inv = InvoiceBuilder::new(format!("RE-TYPE-{}", type_code.code()), date(2024, 6, 15))
            .type_code(type_code)
            .buyer_reference("BR-TYPE")
            .tax_point_date(date(2024, 6, 15))
            .seller(seller())
            .buyer(buyer())
            .add_line(
                LineItemBuilder::new("1", "Service", dec!(1), "C62", dec!(100))
                    .tax(TaxCategory::StandardRate, dec!(19))
                    .build(),
            )
            .payment(payment())
            .build()
            .unwrap();

        // UBL roundtrip
        let ubl = faktura::xrechnung::to_ubl_xml(&inv).unwrap();
        let parsed = faktura::xrechnung::from_ubl_xml(&ubl).unwrap();
        assert_eq!(
            parsed.type_code, type_code,
            "UBL type code mismatch for {:?}",
            type_code
        );

        // CII roundtrip
        let cii = faktura::xrechnung::to_cii_xml(&inv).unwrap();
        let parsed = faktura::xrechnung::from_cii_xml(&cii).unwrap();
        assert_eq!(
            parsed.type_code, type_code,
            "CII type code mismatch for {:?}",
            type_code
        );
    }
}

// --- Attachments ---

#[test]
fn invoice_with_attachments() {
    let mut builder = InvoiceBuilder::new("RE-ATTACH", date(2024, 6, 15))
        .buyer_reference("BR-ATTACH")
        .tax_point_date(date(2024, 6, 15))
        .seller(seller())
        .buyer(buyer())
        .add_line(
            LineItemBuilder::new("1", "Service", dec!(1), "C62", dec!(100))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .payment(payment());

    // Add 10 attachments (mix of external URIs and embedded)
    for i in 1..=10 {
        if i % 2 == 0 {
            builder = builder.add_attachment(DocumentAttachment {
                id: Some(format!("ATT-{i}")),
                description: Some(format!("Attachment {i}")),
                external_uri: Some(format!("https://example.com/doc/{i}.pdf")),
                embedded_document: None,
            });
        } else {
            builder = builder.add_attachment(DocumentAttachment {
                id: Some(format!("ATT-{i}")),
                description: Some(format!("Embedded doc {i}")),
                external_uri: None,
                embedded_document: Some(EmbeddedDocument {
                    content: "SGVsbG8gV29ybGQ=".into(), // "Hello World" in base64
                    mime_type: "application/pdf".into(),
                    filename: format!("doc_{i}.pdf"),
                }),
            });
        }
    }

    let inv = builder.build().unwrap();
    assert_eq!(inv.attachments.len(), 10);

    let ubl = faktura::xrechnung::to_ubl_xml(&inv).unwrap();
    let parsed = faktura::xrechnung::from_ubl_xml(&ubl).unwrap();
    assert_eq!(parsed.attachments.len(), 10);

    let cii = faktura::xrechnung::to_cii_xml(&inv).unwrap();
    let parsed = faktura::xrechnung::from_cii_xml(&cii).unwrap();
    assert_eq!(parsed.attachments.len(), 10);
}

// --- Document-level allowances + charges with reason codes ---

#[test]
fn allowances_and_charges_with_reason_codes() {
    let inv = InvoiceBuilder::new("RE-AC", date(2024, 6, 15))
        .buyer_reference("BR-AC")
        .tax_point_date(date(2024, 6, 15))
        .seller(seller())
        .buyer(buyer())
        .add_line(
            LineItemBuilder::new("1", "Product", dec!(10), "C62", dec!(100))
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
        .add_allowance(AllowanceCharge {
            is_charge: false,
            amount: dec!(25),
            percentage: None,
            base_amount: None,
            tax_category: TaxCategory::StandardRate,
            tax_rate: dec!(19),
            reason: Some("Discount".into()),
            reason_code: Some("100".into()),
        })
        .add_charge(AllowanceCharge {
            is_charge: true,
            amount: dec!(30),
            percentage: None,
            base_amount: None,
            tax_category: TaxCategory::StandardRate,
            tax_rate: dec!(19),
            reason: Some("Versandkosten".into()),
            reason_code: Some("FC".into()),
        })
        .payment(payment())
        .build()
        .unwrap();

    let totals = inv.totals.as_ref().unwrap();
    // Lines: 10 * 100 = 1000
    assert_eq!(totals.line_net_total, dec!(1000));
    assert_eq!(totals.allowances_total, dec!(75));
    assert_eq!(totals.charges_total, dec!(30));
    // Net: 1000 - 75 + 30 = 955
    assert_eq!(totals.net_total, dec!(955));

    let errors = validate_arithmetic(&inv);
    assert!(errors.is_empty(), "arithmetic errors: {:?}", errors);

    // UBL roundtrip
    let ubl = faktura::xrechnung::to_ubl_xml(&inv).unwrap();
    let parsed = faktura::xrechnung::from_ubl_xml(&ubl).unwrap();
    assert_eq!(parsed.allowances.len(), 2);
    assert_eq!(parsed.charges.len(), 1);

    // CII roundtrip
    let cii = faktura::xrechnung::to_cii_xml(&inv).unwrap();
    let parsed = faktura::xrechnung::from_cii_xml(&cii).unwrap();
    assert_eq!(parsed.allowances.len(), 2);
    assert_eq!(parsed.charges.len(), 1);
}

// --- Line-level allowances + charges with gross price ---

#[test]
fn line_level_allowances_charges_and_gross_price() {
    let inv = InvoiceBuilder::new("RE-LINE-AC", date(2024, 6, 15))
        .buyer_reference("BR-LINE-AC")
        .tax_point_date(date(2024, 6, 15))
        .seller(seller())
        .buyer(buyer())
        .add_line(
            LineItemBuilder::new("1", "Product with discount", dec!(5), "C62", dec!(90))
                .tax(TaxCategory::StandardRate, dec!(19))
                .gross_price(dec!(100))
                .add_allowance(AllowanceCharge {
                    is_charge: false,
                    amount: dec!(25),
                    percentage: None,
                    base_amount: None,
                    tax_category: TaxCategory::StandardRate,
                    tax_rate: dec!(19),
                    reason: Some("Line discount".into()),
                    reason_code: None,
                })
                .add_charge(AllowanceCharge {
                    is_charge: true,
                    amount: dec!(10),
                    percentage: None,
                    base_amount: None,
                    tax_category: TaxCategory::StandardRate,
                    tax_rate: dec!(19),
                    reason: Some("Handling fee".into()),
                    reason_code: None,
                })
                .build(),
        )
        .add_line(
            LineItemBuilder::new("2", "Service per 100 hours", dec!(200), "HUR", dec!(50))
                .tax(TaxCategory::StandardRate, dec!(19))
                .base_quantity(dec!(100), Some("HUR".into()))
                .build(),
        )
        .payment(payment())
        .build()
        .unwrap();

    let totals = inv.totals.as_ref().unwrap();
    // Line 1: 5 * 90 = 450 - 25 + 10 = 435
    // Line 2: 200 * 50 = 10000
    assert_eq!(totals.line_net_total, dec!(10435));

    let errors = validate_arithmetic(&inv);
    assert!(errors.is_empty(), "arithmetic errors: {:?}", errors);

    // UBL roundtrip
    let ubl = faktura::xrechnung::to_ubl_xml(&inv).unwrap();
    let parsed = faktura::xrechnung::from_ubl_xml(&ubl).unwrap();
    assert_eq!(parsed.lines[0].gross_price, Some(dec!(100)));
    assert_eq!(parsed.lines[0].allowances.len(), 1);
    assert_eq!(parsed.lines[0].charges.len(), 1);
    assert_eq!(parsed.lines[1].base_quantity, Some(dec!(100)));

    // CII roundtrip
    let cii = faktura::xrechnung::to_cii_xml(&inv).unwrap();
    let parsed = faktura::xrechnung::from_cii_xml(&cii).unwrap();
    assert_eq!(parsed.lines[0].gross_price, Some(dec!(100)));
    assert_eq!(parsed.lines[1].base_quantity, Some(dec!(100)));
}

// --- Multi-currency invoice ---

#[test]
fn multi_currency_invoice() {
    let inv = InvoiceBuilder::new("RE-MULTI-CUR", date(2024, 6, 15))
        .buyer_reference("BR-MCUR")
        .tax_point_date(date(2024, 6, 15))
        .seller(seller())
        .buyer(buyer())
        .add_line(
            LineItemBuilder::new("1", "Service", dec!(1), "C62", dec!(1000))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .tax_currency("USD", dec!(207.48)) // VAT in USD
        .payment(payment())
        .build()
        .unwrap();

    assert_eq!(inv.tax_currency_code, Some("USD".into()));
    let totals = inv.totals.as_ref().unwrap();
    assert_eq!(totals.vat_total_in_tax_currency, Some(dec!(207.48)));
    assert_eq!(totals.vat_total, dec!(190)); // EUR

    // UBL roundtrip
    let ubl = faktura::xrechnung::to_ubl_xml(&inv).unwrap();
    let parsed = faktura::xrechnung::from_ubl_xml(&ubl).unwrap();
    assert_eq!(parsed.tax_currency_code, Some("USD".into()));
    assert_eq!(
        parsed.totals.as_ref().unwrap().vat_total_in_tax_currency,
        Some(dec!(207.48))
    );

    // CII roundtrip
    let cii = faktura::xrechnung::to_cii_xml(&inv).unwrap();
    let parsed = faktura::xrechnung::from_cii_xml(&cii).unwrap();
    assert_eq!(parsed.tax_currency_code, Some("USD".into()));
    assert_eq!(
        parsed.totals.as_ref().unwrap().vat_total_in_tax_currency,
        Some(dec!(207.48))
    );
}
