//! External validator integration tests.
//!
//! These tests require a running easybill/e-invoice-validator Docker container:
//!
//! ```sh
//! docker run -d --name faktura-validator -p 8081:8080 easybill/e-invoice-validator:latest
//! ```
//!
//! Run with:
//! ```sh
//! cargo test --features all --test validator_tests -- --ignored
//! ```

#![cfg(feature = "xrechnung")]

use chrono::NaiveDate;
use faktura::core::*;
use rust_decimal_macros::dec;

const VALIDATOR_URL: &str = "http://localhost:8081/validation";

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

// ── Helpers ──────────────────────────────────────────────────────────────────

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

/// Validate XML against the easybill validator and return the response body.
/// Panics with detailed error info if validation fails.
fn validate_xml(xml: &str, label: &str) {
    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(VALIDATOR_URL)
        .header("Content-Type", "application/xml")
        .body(xml.to_string())
        .send()
        .unwrap_or_else(|e| {
            panic!(
                "[{label}] Failed to connect to validator at {VALIDATOR_URL}. \
                 Is the Docker container running? Error: {e}"
            )
        });

    let status = resp.status();
    let body = resp.text().unwrap();

    if !status.is_success() {
        // Parse JSON response for error details
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
            let errors = json
                .get("violations")
                .or_else(|| json.get("errors"))
                .cloned()
                .unwrap_or(json.clone());
            panic!(
                "[{label}] Validation FAILED (HTTP {status}):\n{}\n\nFull XML:\n{}",
                serde_json::to_string_pretty(&errors).unwrap_or(body.clone()),
                &xml[..2000.min(xml.len())]
            );
        }
        panic!("[{label}] Validation FAILED (HTTP {status}):\n{body}\n\nFull XML:\n{}", &xml[..2000.min(xml.len())]);
    }

    eprintln!("[{label}] ✓ Valid (HTTP {status})");
}

// ── Domestic Invoice (19% VAT) ───────────────────────────────────────────────

fn domestic_invoice() -> Invoice {
    InvoiceBuilder::new("RE-2024-001", date(2024, 6, 15))
        .buyer_reference("BUYER-REF-001")
        .due_date(date(2024, 7, 15))
        .tax_point_date(date(2024, 6, 15))
        .seller(seller())
        .buyer(buyer())
        .add_line(
            LineItemBuilder::new("1", "Softwareentwicklung", dec!(80), "HUR", dec!(120))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .add_line(
            LineItemBuilder::new("2", "Hosting", dec!(1), "C62", dec!(49.90))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .payment(payment())
        .payment_terms("Zahlbar innerhalb von 30 Tagen")
        .build()
        .unwrap()
}

#[test]
#[ignore]
fn domestic_ubl_valid() {
    let inv = domestic_invoice();
    let xml = faktura::xrechnung::to_ubl_xml(&inv).unwrap();
    validate_xml(&xml, "Domestic UBL");
}

#[test]
#[ignore]
fn domestic_cii_valid() {
    let inv = domestic_invoice();
    let xml = faktura::xrechnung::to_cii_xml(&inv).unwrap();
    validate_xml(&xml, "Domestic CII");
}

// ── Mixed VAT Rates (7% + 19%) ──────────────────────────────────────────────

fn mixed_vat_invoice() -> Invoice {
    InvoiceBuilder::new("RE-2024-002", date(2024, 6, 15))
        .buyer_reference("BUYER-REF-002")
        .vat_scenario(VatScenario::Mixed)
        .due_date(date(2024, 7, 15))
        .tax_point_date(date(2024, 6, 15))
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
        .payment(payment())
        .build()
        .unwrap()
}

#[test]
#[ignore]
fn mixed_vat_ubl_valid() {
    let inv = mixed_vat_invoice();
    let xml = faktura::xrechnung::to_ubl_xml(&inv).unwrap();
    validate_xml(&xml, "Mixed VAT UBL");
}

#[test]
#[ignore]
fn mixed_vat_cii_valid() {
    let inv = mixed_vat_invoice();
    let xml = faktura::xrechnung::to_cii_xml(&inv).unwrap();
    validate_xml(&xml, "Mixed VAT CII");
}

// ── Kleinunternehmer (§19 UStG) ─────────────────────────────────────────────

fn kleinunternehmer_invoice() -> Invoice {
    InvoiceBuilder::new("RE-2024-003", date(2024, 6, 15))
        .buyer_reference("BUYER-REF-003")
        .vat_scenario(VatScenario::Kleinunternehmer)
        .note("Kein Ausweis von Umsatzsteuer, da Kleinunternehmer gemäß §19 UStG")
        .due_date(date(2024, 7, 15))
        .tax_point_date(date(2024, 6, 15))
        .seller(seller())
        .buyer(buyer())
        .add_line(
            LineItemBuilder::new("1", "Webdesign", dec!(1), "C62", dec!(2500))
                .tax(TaxCategory::NotSubjectToVat, dec!(0))
                .build(),
        )
        .payment(payment())
        .build()
        .unwrap()
}

#[test]
#[ignore]
fn kleinunternehmer_ubl_valid() {
    let inv = kleinunternehmer_invoice();
    let xml = faktura::xrechnung::to_ubl_xml(&inv).unwrap();
    validate_xml(&xml, "Kleinunternehmer UBL");
}

#[test]
#[ignore]
fn kleinunternehmer_cii_valid() {
    let inv = kleinunternehmer_invoice();
    let xml = faktura::xrechnung::to_cii_xml(&inv).unwrap();
    validate_xml(&xml, "Kleinunternehmer CII");
}

// ── Reverse Charge (§13b UStG) ──────────────────────────────────────────────

fn reverse_charge_invoice() -> Invoice {
    let buyer_at = PartyBuilder::new(
        "Austrian Corp GmbH",
        AddressBuilder::new("Wien", "1010", "AT")
            .street("Stephansplatz 1")
            .build(),
    )
    .vat_id("ATU12345678")
    .electronic_address("EM", "rechnung@austrian.at")
    .build();

    InvoiceBuilder::new("RE-2024-004", date(2024, 6, 15))
        .buyer_reference("BUYER-REF-004")
        .vat_scenario(VatScenario::ReverseCharge)
        .note("Steuerschuldnerschaft des Leistungsempfängers gemäß §13b UStG")
        .due_date(date(2024, 7, 15))
        .tax_point_date(date(2024, 6, 15))
        .seller(seller())
        .buyer(buyer_at)
        .add_line(
            LineItemBuilder::new("1", "IT Consulting", dec!(40), "HUR", dec!(150))
                .tax(TaxCategory::ReverseCharge, dec!(0))
                .build(),
        )
        .payment(payment())
        .build()
        .unwrap()
}

#[test]
#[ignore]
fn reverse_charge_ubl_valid() {
    let inv = reverse_charge_invoice();
    let xml = faktura::xrechnung::to_ubl_xml(&inv).unwrap();
    validate_xml(&xml, "Reverse Charge UBL");
}

#[test]
#[ignore]
fn reverse_charge_cii_valid() {
    let inv = reverse_charge_invoice();
    let xml = faktura::xrechnung::to_cii_xml(&inv).unwrap();
    validate_xml(&xml, "Reverse Charge CII");
}

// ── Intra-Community Supply ──────────────────────────────────────────────────

fn intra_community_invoice() -> Invoice {
    let buyer_fr = PartyBuilder::new(
        "French SARL",
        AddressBuilder::new("Paris", "75001", "FR")
            .street("Rue de Rivoli 1")
            .build(),
    )
    .vat_id("FR12345678901")
    .electronic_address("EM", "facture@french-sarl.fr")
    .build();

    InvoiceBuilder::new("RE-2024-005", date(2024, 6, 15))
        .buyer_reference("BUYER-REF-005")
        .vat_scenario(VatScenario::IntraCommunitySupply)
        .due_date(date(2024, 7, 15))
        .tax_point_date(date(2024, 6, 15))
        .seller(seller())
        .buyer(buyer_fr)
        .add_line(
            LineItemBuilder::new("1", "Maschine", dec!(1), "C62", dec!(50000))
                .tax(TaxCategory::IntraCommunitySupply, dec!(0))
                .build(),
        )
        .payment(payment())
        .build()
        .unwrap()
}

#[test]
#[ignore]
fn intra_community_ubl_valid() {
    let inv = intra_community_invoice();
    let xml = faktura::xrechnung::to_ubl_xml(&inv).unwrap();
    validate_xml(&xml, "Intra-Community UBL");
}

#[test]
#[ignore]
fn intra_community_cii_valid() {
    let inv = intra_community_invoice();
    let xml = faktura::xrechnung::to_cii_xml(&inv).unwrap();
    validate_xml(&xml, "Intra-Community CII");
}

// ── Export (Third Country) ──────────────────────────────────────────────────

fn export_invoice() -> Invoice {
    let buyer_us = PartyBuilder::new(
        "US Corporation Inc",
        AddressBuilder::new("New York", "10001", "US")
            .street("5th Avenue 100")
            .build(),
    )
    .electronic_address("EM", "invoices@uscorp.com")
    .build();

    InvoiceBuilder::new("RE-2024-006", date(2024, 6, 15))
        .buyer_reference("BUYER-REF-006")
        .vat_scenario(VatScenario::Export)
        .due_date(date(2024, 7, 15))
        .tax_point_date(date(2024, 6, 15))
        .seller(seller())
        .buyer(buyer_us)
        .add_line(
            LineItemBuilder::new("1", "Software License", dec!(1), "C62", dec!(10000))
                .tax(TaxCategory::Export, dec!(0))
                .build(),
        )
        .payment(payment())
        .build()
        .unwrap()
}

#[test]
#[ignore]
fn export_ubl_valid() {
    let inv = export_invoice();
    let xml = faktura::xrechnung::to_ubl_xml(&inv).unwrap();
    validate_xml(&xml, "Export UBL");
}

#[test]
#[ignore]
fn export_cii_valid() {
    let inv = export_invoice();
    let xml = faktura::xrechnung::to_cii_xml(&inv).unwrap();
    validate_xml(&xml, "Export CII");
}

// ── Small Invoice (< €250) ─────────────────────────────────────────────────

fn small_invoice() -> Invoice {
    InvoiceBuilder::new("RE-2024-007", date(2024, 6, 15))
        .buyer_reference("BUYER-REF-007")
        .due_date(date(2024, 7, 15))
        .tax_point_date(date(2024, 6, 15))
        .seller(seller())
        .buyer(buyer())
        .add_line(
            LineItemBuilder::new("1", "Druckerpapier", dec!(2), "C62", dec!(9.99))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .payment(payment())
        .build()
        .unwrap()
}

#[test]
#[ignore]
fn small_invoice_ubl_valid() {
    let inv = small_invoice();
    let xml = faktura::xrechnung::to_ubl_xml(&inv).unwrap();
    validate_xml(&xml, "Small Invoice UBL");
}

#[test]
#[ignore]
fn small_invoice_cii_valid() {
    let inv = small_invoice();
    let xml = faktura::xrechnung::to_cii_xml(&inv).unwrap();
    validate_xml(&xml, "Small Invoice CII");
}

// ── Credit Note (Type 381) ──────────────────────────────────────────────────

fn credit_note() -> Invoice {
    InvoiceBuilder::new("GS-2024-001", date(2024, 6, 15))
        .type_code(InvoiceTypeCode::CreditNote)
        .buyer_reference("BUYER-REF-CN1")
        .due_date(date(2024, 7, 15))
        .tax_point_date(date(2024, 6, 15))
        .seller(seller())
        .buyer(buyer())
        .add_line(
            LineItemBuilder::new("1", "Gutschrift Beratung", dec!(5), "HUR", dec!(120))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .payment(payment())
        .build()
        .unwrap()
}

#[test]
#[ignore]
fn credit_note_ubl_valid() {
    let inv = credit_note();
    let xml = faktura::xrechnung::to_ubl_xml(&inv).unwrap();
    validate_xml(&xml, "Credit Note UBL");
}

#[test]
#[ignore]
fn credit_note_cii_valid() {
    let inv = credit_note();
    let xml = faktura::xrechnung::to_cii_xml(&inv).unwrap();
    validate_xml(&xml, "Credit Note CII");
}

// ── Corrected Invoice (Type 384) ────────────────────────────────────────────

fn corrected_invoice() -> Invoice {
    InvoiceBuilder::new("REC-2024-001", date(2024, 6, 15))
        .type_code(InvoiceTypeCode::Corrected)
        .buyer_reference("BUYER-REF-CORR1")
        .due_date(date(2024, 7, 15))
        .tax_point_date(date(2024, 6, 15))
        .seller(seller())
        .buyer(buyer())
        .add_line(
            LineItemBuilder::new("1", "Korrigierte Beratung", dec!(10), "HUR", dec!(150))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .payment(payment())
        .build()
        .unwrap()
}

#[test]
#[ignore]
fn corrected_invoice_ubl_valid() {
    let inv = corrected_invoice();
    let xml = faktura::xrechnung::to_ubl_xml(&inv).unwrap();
    validate_xml(&xml, "Corrected Invoice UBL");
}

#[test]
#[ignore]
fn corrected_invoice_cii_valid() {
    let inv = corrected_invoice();
    let xml = faktura::xrechnung::to_cii_xml(&inv).unwrap();
    validate_xml(&xml, "Corrected Invoice CII");
}

// ── Peppol Invoice ──────────────────────────────────────────────────────────

#[cfg(feature = "peppol")]
fn peppol_invoice() -> Invoice {
    InvoiceBuilder::new("PEPP-2024-001", date(2024, 6, 15))
        .buyer_reference("BR-PEPPOL-001")
        .due_date(date(2024, 7, 15))
        .tax_point_date(date(2024, 6, 15))
        .seller(
            PartyBuilder::new(
                "Seller GmbH",
                AddressBuilder::new("Berlin", "10115", "DE")
                    .street("Unter den Linden 1")
                    .build(),
            )
            .vat_id("DE123456789")
            .electronic_address("EM", "seller@peppol.eu")
            .contact(
                Some("Max Mustermann".into()),
                Some("+49 30 12345678".into()),
                Some("max@seller.de".into()),
            )
            .build(),
        )
        .buyer(
            PartyBuilder::new(
                "Buyer AG",
                AddressBuilder::new("München", "80331", "DE")
                    .street("Leopoldstraße 1")
                    .build(),
            )
            .electronic_address("EM", "buyer@peppol.eu")
            .build(),
        )
        .add_line(
            LineItemBuilder::new("1", "Consulting services", dec!(10), "HUR", dec!(150))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .payment(PaymentInstructions {
            means_code: PaymentMeansCode::SepaCreditTransfer,
            means_text: None,
            remittance_info: Some("PEPP-2024-001".into()),
            credit_transfer: Some(CreditTransfer {
                iban: "DE89370400440532013000".into(),
                bic: Some("COBADEFFXXX".into()),
                account_name: Some("Seller GmbH".into()),
            }),
            card_payment: None,
            direct_debit: None,
        })
        .build()
        .unwrap()
}

#[test]
#[ignore]
#[cfg(feature = "peppol")]
fn peppol_ubl_valid() {
    let inv = peppol_invoice();
    let xml = faktura::peppol::to_ubl_xml(&inv).unwrap();
    validate_xml(&xml, "Peppol UBL");
}
