#![cfg(feature = "datev")]

use chrono::NaiveDate;
use faktura::core::*;
use faktura::datev::*;
use rust_decimal_macros::dec;

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

fn default_config() -> DatevConfig {
    DatevConfig {
        consultant_number: 29098,
        client_number: 55003,
        fiscal_year_start: date(2024, 1, 1),
        account_length: 4,
        chart: ChartOfAccounts::SKR03,
        default_debitor: 10000,
        source: "RE".into(),
        exported_by: "faktura".into(),
        description: "Buchungsstapel".into(),
        lock_postings: false,
    }
}

fn domestic_invoice() -> Invoice {
    InvoiceBuilder::new("RE-2024-001", date(2024, 6, 15))
        .due_date(date(2024, 7, 15))
        .seller(
            PartyBuilder::new(
                "ACME GmbH",
                AddressBuilder::new("Berlin", "10115", "DE")
                    .street("Friedrichstraße 123")
                    .build(),
            )
            .vat_id("DE123456789")
            .build(),
        )
        .buyer(
            PartyBuilder::new(
                "Kunde AG",
                AddressBuilder::new("München", "80331", "DE")
                    .street("Marienplatz 1")
                    .build(),
            )
            .build(),
        )
        .add_line(
            LineItemBuilder::new("1", "Beratung", dec!(10), "HUR", dec!(150))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .build()
        .unwrap()
}

fn mixed_rate_invoice() -> Invoice {
    InvoiceBuilder::new("RE-2024-002", date(2024, 7, 1))
        .due_date(date(2024, 7, 31))
        .vat_scenario(VatScenario::Mixed)
        .seller(
            PartyBuilder::new(
                "Buchhandlung GmbH",
                AddressBuilder::new("Hamburg", "20095", "DE").build(),
            )
            .vat_id("DE987654321")
            .build(),
        )
        .buyer(
            PartyBuilder::new(
                "Leser AG",
                AddressBuilder::new("Köln", "50667", "DE").build(),
            )
            .build(),
        )
        .add_line(
            LineItemBuilder::new("1", "Fachbuch", dec!(5), "C62", dec!(30))
                .tax(TaxCategory::StandardRate, dec!(7))
                .build(),
        )
        .add_line(
            LineItemBuilder::new("2", "Beratung", dec!(2), "HUR", dec!(200))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .build()
        .unwrap()
}

fn credit_note() -> Invoice {
    InvoiceBuilder::new("GS-2024-001", date(2024, 8, 1))
        .type_code(InvoiceTypeCode::CreditNote)
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
            LineItemBuilder::new("1", "Gutschrift Beratung", dec!(5), "HUR", dec!(150))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .build()
        .unwrap()
}

fn eu_invoice() -> Invoice {
    InvoiceBuilder::new("RE-2024-003", date(2024, 9, 1))
        .vat_scenario(VatScenario::IntraCommunitySupply)
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
                "Client SARL",
                AddressBuilder::new("Paris", "75001", "FR").build(),
            )
            .vat_id("FR12345678901")
            .build(),
        )
        .add_line(
            LineItemBuilder::new("1", "Consulting", dec!(20), "HUR", dec!(100))
                .tax(TaxCategory::IntraCommunitySupply, dec!(0))
                .build(),
        )
        .build()
        .unwrap()
}

// ---------------------------------------------------------------------------
// EXTF Header Tests
// ---------------------------------------------------------------------------

#[test]
fn header_starts_with_extf() {
    let inv = domestic_invoice();
    let csv = to_extf(&[inv], &default_config()).unwrap();
    assert!(csv.starts_with("\"EXTF\";700;21;\"Buchungsstapel\";13;"));
}

#[test]
fn header_contains_consultant_and_client() {
    let inv = domestic_invoice();
    let csv = to_extf(&[inv], &default_config()).unwrap();
    let header = csv.lines().next().unwrap();
    assert!(header.contains(";29098;"), "missing consultant number");
    assert!(header.contains(";55003;"), "missing client number");
}

#[test]
fn header_contains_skr03() {
    let inv = domestic_invoice();
    let csv = to_extf(&[inv], &default_config()).unwrap();
    let header = csv.lines().next().unwrap();
    assert!(header.contains(";\"03\";"), "missing SKR03 identifier");
}

#[test]
fn header_contains_skr04() {
    let inv = domestic_invoice();
    let mut config = default_config();
    config.chart = ChartOfAccounts::SKR04;
    let csv = to_extf(&[inv], &config).unwrap();
    let header = csv.lines().next().unwrap();
    assert!(header.contains(";\"04\";"), "missing SKR04 identifier");
}

#[test]
fn header_contains_fiscal_year() {
    let inv = domestic_invoice();
    let csv = to_extf(&[inv], &default_config()).unwrap();
    let header = csv.lines().next().unwrap();
    assert!(header.contains(";20240101;"), "missing fiscal year start");
}

#[test]
fn header_contains_period() {
    let inv = domestic_invoice();
    let csv = to_extf(&[inv], &default_config()).unwrap();
    let header = csv.lines().next().unwrap();
    // Period should span the invoice date
    assert!(
        header.contains(";20240615;20240615;"),
        "missing period dates"
    );
}

// ---------------------------------------------------------------------------
// Column Header Tests
// ---------------------------------------------------------------------------

#[test]
fn second_line_is_column_headers() {
    let inv = domestic_invoice();
    let csv = to_extf(&[inv], &default_config()).unwrap();
    let lines: Vec<&str> = csv.lines().collect();
    assert!(lines.len() >= 3, "expected at least 3 lines");
    assert!(lines[1].starts_with("Umsatz (ohne Soll/Haben-Kz);"));
    assert!(lines[1].contains("Buchungstext"));
}

// ---------------------------------------------------------------------------
// Data Row Tests — Domestic Invoice
// ---------------------------------------------------------------------------

#[test]
fn domestic_invoice_produces_one_row() {
    let inv = domestic_invoice();
    let csv = to_extf(&[inv], &default_config()).unwrap();
    let lines: Vec<&str> = csv.lines().collect();
    // Header + column headers + 1 data row = 3 lines
    assert_eq!(
        lines.len(),
        3,
        "expected exactly 3 lines, got {}",
        lines.len()
    );
}

#[test]
fn domestic_invoice_gross_amount() {
    let inv = domestic_invoice();
    let csv = to_extf(&[inv], &default_config()).unwrap();
    let data_line = csv.lines().nth(2).unwrap();
    // 10 * 150 = 1500 net, 19% = 285, gross = 1785
    assert!(
        data_line.starts_with("1785,00;"),
        "expected gross 1785,00, got: {}",
        data_line
    );
}

#[test]
fn domestic_invoice_debit_direction() {
    let inv = domestic_invoice();
    let csv = to_extf(&[inv], &default_config()).unwrap();
    let data_line = csv.lines().nth(2).unwrap();
    let fields: Vec<&str> = data_line.split(';').collect();
    assert_eq!(fields[1], "\"S\"", "expected Soll for normal invoice");
}

#[test]
fn domestic_invoice_accounts_skr03() {
    let inv = domestic_invoice();
    let csv = to_extf(&[inv], &default_config()).unwrap();
    let data_line = csv.lines().nth(2).unwrap();
    let fields: Vec<&str> = data_line.split(';').collect();
    // Konto = debitor (10000), Gegenkonto = 8400 (Erlöse 19%)
    assert_eq!(fields[6], "10000", "expected debitor account 10000");
    assert_eq!(fields[7], "8400", "expected revenue account 8400");
}

#[test]
fn domestic_invoice_no_bu_key_for_automatikkonto() {
    let inv = domestic_invoice();
    let csv = to_extf(&[inv], &default_config()).unwrap();
    let data_line = csv.lines().nth(2).unwrap();
    let fields: Vec<&str> = data_line.split(';').collect();
    // BU-Schlüssel should be empty for Automatikkonto 8400
    assert_eq!(fields[8], "", "expected empty BU key for Automatikkonto");
}

#[test]
fn domestic_invoice_date_format_ddmm() {
    let inv = domestic_invoice();
    let csv = to_extf(&[inv], &default_config()).unwrap();
    let data_line = csv.lines().nth(2).unwrap();
    let fields: Vec<&str> = data_line.split(';').collect();
    assert_eq!(fields[9], "1506", "expected date 1506 (June 15)");
}

#[test]
fn domestic_invoice_document_number() {
    let inv = domestic_invoice();
    let csv = to_extf(&[inv], &default_config()).unwrap();
    let data_line = csv.lines().nth(2).unwrap();
    let fields: Vec<&str> = data_line.split(';').collect();
    assert_eq!(fields[10], "\"RE-2024-001\"");
}

#[test]
fn domestic_invoice_posting_text() {
    let inv = domestic_invoice();
    let csv = to_extf(&[inv], &default_config()).unwrap();
    let data_line = csv.lines().nth(2).unwrap();
    let fields: Vec<&str> = data_line.split(';').collect();
    // Single line item: "RE-2024-001 Beratung"
    assert_eq!(fields[13], "\"RE-2024-001 Beratung\"");
}

// ---------------------------------------------------------------------------
// SKR04 Tests
// ---------------------------------------------------------------------------

#[test]
fn domestic_invoice_accounts_skr04() {
    let inv = domestic_invoice();
    let mut config = default_config();
    config.chart = ChartOfAccounts::SKR04;
    let csv = to_extf(&[inv], &config).unwrap();
    let data_line = csv.lines().nth(2).unwrap();
    let fields: Vec<&str> = data_line.split(';').collect();
    assert_eq!(fields[7], "4400", "expected SKR04 revenue account 4400");
}

// ---------------------------------------------------------------------------
// Mixed Rate Invoice
// ---------------------------------------------------------------------------

#[test]
fn mixed_rate_produces_two_rows() {
    let inv = mixed_rate_invoice();
    let csv = to_extf(&[inv], &default_config()).unwrap();
    let lines: Vec<&str> = csv.lines().collect();
    // Header + column headers + 2 data rows = 4 lines
    assert_eq!(lines.len(), 4, "expected 4 lines for mixed rate invoice");
}

#[test]
fn mixed_rate_7pct_row() {
    let inv = mixed_rate_invoice();
    let csv = to_extf(&[inv], &default_config()).unwrap();
    let data_line = csv.lines().nth(2).unwrap();
    let fields: Vec<&str> = data_line.split(';').collect();
    // 5 * 30 = 150 net, 7% = 10.50, gross = 160.50
    assert!(
        data_line.starts_with("160,50;"),
        "expected 160,50 for 7% group"
    );
    assert_eq!(fields[7], "8300", "expected 8300 for 7% revenue");
}

#[test]
fn mixed_rate_19pct_row() {
    let inv = mixed_rate_invoice();
    let csv = to_extf(&[inv], &default_config()).unwrap();
    let data_line = csv.lines().nth(3).unwrap();
    let fields: Vec<&str> = data_line.split(';').collect();
    // 2 * 200 = 400 net, 19% = 76, gross = 476
    assert!(
        data_line.starts_with("476,00;"),
        "expected 476,00 for 19% group"
    );
    assert_eq!(fields[7], "8400", "expected 8400 for 19% revenue");
}

// ---------------------------------------------------------------------------
// Credit Note
// ---------------------------------------------------------------------------

#[test]
fn credit_note_uses_haben() {
    let inv = credit_note();
    let csv = to_extf(&[inv], &default_config()).unwrap();
    let data_line = csv.lines().nth(2).unwrap();
    let fields: Vec<&str> = data_line.split(';').collect();
    assert_eq!(fields[1], "\"H\"", "expected Haben for credit note");
}

#[test]
fn credit_note_positive_amount() {
    let inv = credit_note();
    let csv = to_extf(&[inv], &default_config()).unwrap();
    let data_line = csv.lines().nth(2).unwrap();
    // 5 * 150 = 750 net, 19% = 142.50, gross = 892.50
    assert!(
        data_line.starts_with("892,50;"),
        "expected positive amount 892,50"
    );
}

// ---------------------------------------------------------------------------
// EU / Intra-Community Invoice
// ---------------------------------------------------------------------------

#[test]
fn eu_invoice_uses_account_8125() {
    let inv = eu_invoice();
    let csv = to_extf(&[inv], &default_config()).unwrap();
    let data_line = csv.lines().nth(2).unwrap();
    let fields: Vec<&str> = data_line.split(';').collect();
    assert_eq!(
        fields[7], "8125",
        "expected 8125 for intra-community supply"
    );
}

#[test]
fn eu_invoice_includes_vat_id() {
    let inv = eu_invoice();
    let csv = to_extf(&[inv], &default_config()).unwrap();
    let data_line = csv.lines().nth(2).unwrap();
    assert!(
        data_line.contains("FR12345678901"),
        "expected buyer VAT ID in EU field"
    );
}

// ---------------------------------------------------------------------------
// CRLF Line Endings
// ---------------------------------------------------------------------------

#[test]
fn output_uses_crlf() {
    let inv = domestic_invoice();
    let csv = to_extf(&[inv], &default_config()).unwrap();
    assert!(csv.contains("\r\n"), "expected CRLF line endings");
    // Should NOT have bare LF without CR
    let without_cr = csv.replace("\r\n", "");
    assert!(!without_cr.contains('\n'), "found bare LF without CR");
}

// ---------------------------------------------------------------------------
// Multiple Invoices
// ---------------------------------------------------------------------------

#[test]
fn multiple_invoices_in_one_batch() {
    let inv1 = domestic_invoice();
    let inv2 = mixed_rate_invoice();
    let csv = to_extf(&[inv1, inv2], &default_config()).unwrap();
    let lines: Vec<&str> = csv.lines().collect();
    // Header + column headers + 1 row (inv1) + 2 rows (inv2) = 5 lines
    assert_eq!(lines.len(), 5, "expected 5 lines for 2 invoices");
}

#[test]
fn multiple_invoices_period_spans_all() {
    let inv1 = domestic_invoice(); // June 15
    let inv2 = mixed_rate_invoice(); // July 1
    let csv = to_extf(&[inv1, inv2], &default_config()).unwrap();
    let header = csv.lines().next().unwrap();
    assert!(
        header.contains(";20240615;"),
        "period start should be June 15"
    );
    assert!(header.contains(";20240701;"), "period end should be July 1");
}

// ---------------------------------------------------------------------------
// Edge Cases
// ---------------------------------------------------------------------------

#[test]
fn empty_invoices_returns_error() {
    let result = to_extf(&[], &default_config());
    assert!(result.is_err());
}

#[test]
fn invoice_without_totals_returns_error() {
    let mut inv = domestic_invoice();
    inv.totals = None;
    let result = to_extf(&[inv], &default_config());
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("no calculated totals")
    );
}

// ---------------------------------------------------------------------------
// BU-Schlüssel Tests
// ---------------------------------------------------------------------------

#[test]
fn bu_schluessel_19pct() {
    let bu = bu_schluessel(TaxCategory::StandardRate, dec!(19));
    assert_eq!(bu, Some(BuSchluessel(3)));
}

#[test]
fn bu_schluessel_7pct() {
    let bu = bu_schluessel(TaxCategory::StandardRate, dec!(7));
    assert_eq!(bu, Some(BuSchluessel(2)));
}

#[test]
fn bu_schluessel_exempt_none() {
    assert_eq!(bu_schluessel(TaxCategory::Exempt, dec!(0)), None);
}

#[test]
fn bu_schluessel_export_none() {
    assert_eq!(bu_schluessel(TaxCategory::Export, dec!(0)), None);
}

#[test]
fn bu_schluessel_eu_delivery() {
    assert_eq!(
        bu_schluessel(TaxCategory::IntraCommunitySupply, dec!(0)),
        Some(BuSchluessel(10))
    );
}

// ---------------------------------------------------------------------------
// Due Date in Output
// ---------------------------------------------------------------------------

#[test]
fn due_date_in_field_117() {
    let inv = domestic_invoice();
    let csv = to_extf(&[inv], &default_config()).unwrap();
    let data_line = csv.lines().nth(2).unwrap();
    // Due date is July 15, 2024 → 15072024
    assert!(
        data_line.contains("15072024"),
        "expected due date 15072024 in output"
    );
}
