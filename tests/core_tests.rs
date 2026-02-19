use chrono::NaiveDate;
use faktura::core::*;
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

// --- Domestic Invoice ---

#[test]
fn domestic_invoice_full() {
    let inv = InvoiceBuilder::new("RE-2024-001", date(2024, 6, 15))
        .due_date(date(2024, 7, 15))
        .seller(seller())
        .buyer(buyer())
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
            means_text: Some("SEPA Überweisung".into()),
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
        .tax_point_date(date(2024, 6, 15))
        .build()
        .unwrap();

    let totals = inv.totals.as_ref().unwrap();

    // 80 * 120 = 9600, 1 * 49.90 = 49.90 → 9649.90
    assert_eq!(totals.line_net_total, dec!(9649.90));
    // 9649.90 * 0.19 = 1833.481 → rounded 1833.48
    assert_eq!(totals.vat_total, dec!(1833.48));
    assert_eq!(totals.gross_total, dec!(11483.38));
    assert_eq!(totals.amount_due, dec!(11483.38));
    assert_eq!(totals.prepaid, dec!(0));

    // VAT breakdown
    assert_eq!(totals.vat_breakdown.len(), 1);
    assert_eq!(totals.vat_breakdown[0].rate, dec!(19));
    assert_eq!(totals.vat_breakdown[0].category, TaxCategory::StandardRate);
}

// --- Mixed VAT Rates ---

#[test]
fn mixed_vat_rates() {
    let inv = InvoiceBuilder::new("RE-2024-002", date(2024, 6, 15))
        .vat_scenario(VatScenario::Mixed)
        .seller(seller())
        .buyer(buyer())
        .add_line(
            LineItemBuilder::new("1", "Bücher", dec!(3), "C62", dec!(29.99))
                .tax(TaxCategory::StandardRate, dec!(7))
                .build(),
        )
        .add_line(
            LineItemBuilder::new("2", "Software", dec!(1), "C62", dec!(199))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .tax_point_date(date(2024, 6, 15))
        .build()
        .unwrap();

    let totals = inv.totals.as_ref().unwrap();
    // Books: 3 * 29.99 = 89.97, VAT = 89.97 * 0.07 = 6.2979 → 6.30
    // Software: 199, VAT = 199 * 0.19 = 37.81
    assert_eq!(totals.vat_breakdown.len(), 2);

    let vat_7 = totals
        .vat_breakdown
        .iter()
        .find(|b| b.rate == dec!(7))
        .unwrap();
    assert_eq!(vat_7.taxable_amount, dec!(89.97));
    assert_eq!(vat_7.tax_amount, dec!(6.30));

    let vat_19 = totals
        .vat_breakdown
        .iter()
        .find(|b| b.rate == dec!(19))
        .unwrap();
    assert_eq!(vat_19.taxable_amount, dec!(199));
    assert_eq!(vat_19.tax_amount, dec!(37.81));

    assert_eq!(totals.vat_total, dec!(44.11));
}

// --- Kleinunternehmer ---

#[test]
fn kleinunternehmer_invoice() {
    let inv = InvoiceBuilder::new("RE-2024-003", date(2024, 6, 15))
        .vat_scenario(VatScenario::Kleinunternehmer)
        .note("Kein Ausweis von Umsatzsteuer, da Kleinunternehmer gemäß §19 UStG")
        .seller(seller())
        .buyer(buyer())
        .add_line(
            LineItemBuilder::new("1", "Webdesign", dec!(1), "C62", dec!(2500))
                .tax(TaxCategory::NotSubjectToVat, dec!(0))
                .build(),
        )
        .tax_point_date(date(2024, 6, 15))
        .build()
        .unwrap();

    let totals = inv.totals.as_ref().unwrap();
    assert_eq!(totals.vat_total, dec!(0));
    assert_eq!(totals.net_total, dec!(2500));
    assert_eq!(totals.gross_total, dec!(2500));

    // Exemption reason in breakdown
    assert_eq!(totals.vat_breakdown.len(), 1);
    assert!(
        totals.vat_breakdown[0]
            .exemption_reason
            .as_ref()
            .unwrap()
            .contains("Kleinunternehmer")
    );
}

// --- Reverse Charge ---

#[test]
fn reverse_charge_invoice() {
    let buyer_at = PartyBuilder::new(
        "Austrian Corp",
        AddressBuilder::new("Wien", "1010", "AT").build(),
    )
    .vat_id("ATU12345678")
    .build();

    let inv = InvoiceBuilder::new("RE-2024-004", date(2024, 6, 15))
        .vat_scenario(VatScenario::ReverseCharge)
        .note("Steuerschuldnerschaft des Leistungsempfängers gemäß §13b UStG")
        .seller(seller())
        .buyer(buyer_at)
        .add_line(
            LineItemBuilder::new("1", "IT Consulting", dec!(40), "HUR", dec!(150))
                .tax(TaxCategory::ReverseCharge, dec!(0))
                .build(),
        )
        .tax_point_date(date(2024, 6, 15))
        .build()
        .unwrap();

    let totals = inv.totals.as_ref().unwrap();
    assert_eq!(totals.vat_total, dec!(0));
    assert_eq!(totals.net_total, dec!(6000));
    assert_eq!(totals.gross_total, dec!(6000));
}

// --- Intra-Community Supply ---

#[test]
fn intra_community_supply() {
    let buyer_fr = PartyBuilder::new(
        "French SARL",
        AddressBuilder::new("Paris", "75001", "FR").build(),
    )
    .vat_id("FR12345678901")
    .build();

    let inv = InvoiceBuilder::new("RE-2024-005", date(2024, 6, 15))
        .vat_scenario(VatScenario::IntraCommunitySupply)
        .seller(seller())
        .buyer(buyer_fr)
        .add_line(
            LineItemBuilder::new("1", "Maschine", dec!(1), "C62", dec!(50000))
                .tax(TaxCategory::IntraCommunitySupply, dec!(0))
                .build(),
        )
        .tax_point_date(date(2024, 6, 15))
        .build()
        .unwrap();

    let totals = inv.totals.as_ref().unwrap();
    assert_eq!(totals.vat_total, dec!(0));
    assert_eq!(totals.gross_total, dec!(50000));
}

// --- Export ---

#[test]
fn export_invoice() {
    let buyer_us = PartyBuilder::new(
        "US Corp",
        AddressBuilder::new("New York", "10001", "US").build(),
    )
    .build();

    let inv = InvoiceBuilder::new("RE-2024-006", date(2024, 6, 15))
        .vat_scenario(VatScenario::Export)
        .seller(seller())
        .buyer(buyer_us)
        .add_line(
            LineItemBuilder::new("1", "Software License", dec!(1), "C62", dec!(10000))
                .tax(TaxCategory::Export, dec!(0))
                .build(),
        )
        .tax_point_date(date(2024, 6, 15))
        .build()
        .unwrap();

    let totals = inv.totals.as_ref().unwrap();
    assert_eq!(totals.vat_total, dec!(0));
}

// --- Document-level Allowances & Charges ---

#[test]
fn allowances_and_charges() {
    let inv = InvoiceBuilder::new("RE-2024-007", date(2024, 6, 15))
        .seller(seller())
        .buyer(buyer())
        .add_line(
            LineItemBuilder::new("1", "Produkt", dec!(10), "C62", dec!(100))
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
            reason_code: None,
        })
        .add_charge(AllowanceCharge {
            is_charge: true,
            amount: dec!(25),
            percentage: None,
            base_amount: None,
            tax_category: TaxCategory::StandardRate,
            tax_rate: dec!(19),
            reason: Some("Versandkosten".into()),
            reason_code: None,
        })
        .tax_point_date(date(2024, 6, 15))
        .build()
        .unwrap();

    let totals = inv.totals.as_ref().unwrap();
    // Lines: 10 * 100 = 1000
    assert_eq!(totals.line_net_total, dec!(1000));
    assert_eq!(totals.allowances_total, dec!(50));
    assert_eq!(totals.charges_total, dec!(25));
    // Net: 1000 - 50 + 25 = 975
    assert_eq!(totals.net_total, dec!(975));
    // VAT base: 1000 - 50 + 25 = 975, VAT: 975 * 0.19 = 185.25
    assert_eq!(totals.vat_total, dec!(185.25));
    assert_eq!(totals.gross_total, dec!(1160.25));
}

// --- Prepayment ---

#[test]
fn prepaid_amount() {
    let inv = InvoiceBuilder::new("RE-2024-008", date(2024, 6, 15))
        .seller(seller())
        .buyer(buyer())
        .add_line(
            LineItemBuilder::new("1", "Projekt", dec!(1), "C62", dec!(10000))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .prepaid(dec!(5000))
        .tax_point_date(date(2024, 6, 15))
        .build()
        .unwrap();

    let totals = inv.totals.as_ref().unwrap();
    assert_eq!(totals.gross_total, dec!(11900));
    assert_eq!(totals.prepaid, dec!(5000));
    assert_eq!(totals.amount_due, dec!(6900));
}

// --- Credit Note ---

#[test]
fn credit_note() {
    let inv = InvoiceBuilder::new("GS-2024-001", date(2024, 6, 15))
        .type_code(InvoiceTypeCode::CreditNote)
        .seller(seller())
        .buyer(buyer())
        .add_line(
            LineItemBuilder::new("1", "Gutschrift Beratung", dec!(5), "HUR", dec!(120))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .tax_point_date(date(2024, 6, 15))
        .build()
        .unwrap();

    assert_eq!(inv.type_code, InvoiceTypeCode::CreditNote);
    assert_eq!(inv.type_code.code(), 381);
}

// --- Validation Failures ---

#[test]
fn rejects_empty_invoice_number() {
    let result = InvoiceBuilder::new("", date(2024, 6, 15))
        .seller(seller())
        .buyer(buyer())
        .add_line(
            LineItemBuilder::new("1", "Test", dec!(1), "C62", dec!(100))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .build();

    assert!(result.is_err());
}

#[test]
fn rejects_no_lines() {
    let result = InvoiceBuilder::new("RE-001", date(2024, 6, 15))
        .seller(seller())
        .buyer(buyer())
        .build();

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("line item"));
}

#[test]
fn rejects_invalid_vat_id_format() {
    let bad_seller = PartyBuilder::new(
        "Bad GmbH",
        AddressBuilder::new("Berlin", "10115", "DE").build(),
    )
    .vat_id("DE12345") // too short
    .build();

    let result = InvoiceBuilder::new("RE-001", date(2024, 6, 15))
        .seller(bad_seller)
        .buyer(buyer())
        .add_line(
            LineItemBuilder::new("1", "Test", dec!(1), "C62", dec!(100))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .build();

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("9 digits"));
}

#[test]
fn rejects_wrong_tax_category_for_scenario() {
    // Kleinunternehmer with StandardRate should fail
    let result = InvoiceBuilder::new("RE-001", date(2024, 6, 15))
        .vat_scenario(VatScenario::Kleinunternehmer)
        .note("Gemäß §19 UStG wird keine Umsatzsteuer berechnet")
        .seller(seller())
        .buyer(buyer())
        .add_line(
            LineItemBuilder::new("1", "Test", dec!(1), "C62", dec!(100))
                .tax(TaxCategory::StandardRate, dec!(19)) // wrong for Kleinunternehmer
                .build(),
        )
        .build();

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("NotSubjectToVat"));
}

// --- Serialization ---

#[test]
fn invoice_serializes_to_json() {
    let inv = InvoiceBuilder::new("RE-2024-001", date(2024, 6, 15))
        .seller(seller())
        .buyer(buyer())
        .add_line(
            LineItemBuilder::new("1", "Test", dec!(1), "C62", dec!(100))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .tax_point_date(date(2024, 6, 15))
        .build()
        .unwrap();

    let json = serde_json::to_string_pretty(&inv).unwrap();
    assert!(json.contains("RE-2024-001"));
    assert!(json.contains("ACME GmbH"));

    // Roundtrip
    let deserialized: faktura::Invoice = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.number, "RE-2024-001");
}

// --- Numbering ---

#[test]
fn gapless_numbering_sequence() {
    let mut seq = InvoiceNumberSequence::new("RE-", 2024);

    let numbers: Vec<String> = (0..5).map(|_| seq.next_number()).collect();
    assert_eq!(
        numbers,
        vec![
            "RE-2024-001",
            "RE-2024-002",
            "RE-2024-003",
            "RE-2024-004",
            "RE-2024-005",
        ]
    );
}

#[test]
fn numbering_year_rollover() {
    let mut seq = InvoiceNumberSequence::new("RE-", 2024);
    seq.next_number(); // 001
    seq.next_number(); // 002

    let jan_1 = date(2025, 1, 1);
    seq.auto_advance(jan_1);
    assert_eq!(seq.next_number(), "RE-2025-001");
}
