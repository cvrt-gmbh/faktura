#![cfg(feature = "gdpdu")]

use chrono::NaiveDate;
use faktura::core::*;
use faktura::gdpdu::*;
use rust_decimal_macros::dec;

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

fn default_config() -> GdpduConfig {
    GdpduConfig {
        company_name: "ACME GmbH".into(),
        location: "Deutschland".into(),
        comment: "GDPdU-Export Test".into(),
    }
}

fn domestic_invoice() -> Invoice {
    InvoiceBuilder::new("RE-2024-001", date(2024, 3, 15))
        .due_date(date(2024, 4, 15))
        .tax_point_date(date(2024, 3, 15))
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
            .vat_id("DE987654321")
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
    InvoiceBuilder::new("RE-2024-002", date(2024, 4, 1))
        .due_date(date(2024, 5, 1))
        .vat_scenario(VatScenario::Mixed)
        .seller(
            PartyBuilder::new(
                "Buchhandlung GmbH",
                AddressBuilder::new("Hamburg", "20095", "DE").build(),
            )
            .vat_id("DE111222333")
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

// ---------------------------------------------------------------------------
// Export Structure Tests
// ---------------------------------------------------------------------------

#[test]
fn export_produces_index_xml_and_two_csv_files() {
    let inv = domestic_invoice();
    let export = to_gdpdu(&[inv], &default_config()).unwrap();
    assert!(!export.index_xml.is_empty());
    assert_eq!(export.files.len(), 2);
    assert_eq!(export.files[0].0, "kunden.csv");
    assert_eq!(export.files[1].0, "rechnungsausgang.csv");
}

#[test]
fn export_includes_dtd() {
    let inv = domestic_invoice();
    let export = to_gdpdu(&[inv], &default_config()).unwrap();
    assert!(export.dtd.contains("<!ELEMENT DataSet"));
    assert!(export.dtd.contains("<!ELEMENT VariableLength"));
}

#[test]
fn empty_invoices_returns_error() {
    let result = to_gdpdu(&[], &default_config());
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// index.xml Tests
// ---------------------------------------------------------------------------

#[test]
fn index_xml_has_doctype() {
    let inv = domestic_invoice();
    let export = to_gdpdu(&[inv], &default_config()).unwrap();
    assert!(
        export
            .index_xml
            .contains("<!DOCTYPE DataSet SYSTEM \"gdpdu-01-08-2002.dtd\">")
    );
}

#[test]
fn index_xml_has_version() {
    let inv = domestic_invoice();
    let export = to_gdpdu(&[inv], &default_config()).unwrap();
    assert!(export.index_xml.contains("<Version>1.0</Version>"));
}

#[test]
fn index_xml_has_data_supplier() {
    let inv = domestic_invoice();
    let export = to_gdpdu(&[inv], &default_config()).unwrap();
    assert!(export.index_xml.contains("<Name>ACME GmbH</Name>"));
    assert!(
        export
            .index_xml
            .contains("<Location>Deutschland</Location>")
    );
}

#[test]
fn index_xml_has_kunden_table() {
    let inv = domestic_invoice();
    let export = to_gdpdu(&[inv], &default_config()).unwrap();
    assert!(export.index_xml.contains("<URL>kunden.csv</URL>"));
    assert!(export.index_xml.contains("<Name>Kunden</Name>"));
    assert!(export.index_xml.contains("<Name>Kundenkontonummer</Name>"));
}

#[test]
fn index_xml_has_rechnungsausgang_table() {
    let inv = domestic_invoice();
    let export = to_gdpdu(&[inv], &default_config()).unwrap();
    assert!(export.index_xml.contains("<URL>rechnungsausgang.csv</URL>"));
    assert!(export.index_xml.contains("<Name>Rechnungsausgang</Name>"));
    assert!(export.index_xml.contains("<Name>Belegnummer</Name>"));
    assert!(export.index_xml.contains("<Name>Nettobetrag</Name>"));
    assert!(export.index_xml.contains("<Name>Steuerbetrag</Name>"));
    assert!(export.index_xml.contains("<Name>Bruttobetrag</Name>"));
}

#[test]
fn index_xml_has_numeric_accuracy() {
    let inv = domestic_invoice();
    let export = to_gdpdu(&[inv], &default_config()).unwrap();
    assert!(export.index_xml.contains("<Accuracy>2</Accuracy>"));
}

#[test]
fn index_xml_has_date_format() {
    let inv = domestic_invoice();
    let export = to_gdpdu(&[inv], &default_config()).unwrap();
    assert!(export.index_xml.contains("<Format>DD.MM.YYYY</Format>"));
}

#[test]
fn index_xml_has_utf8_encoding() {
    let inv = domestic_invoice();
    let export = to_gdpdu(&[inv], &default_config()).unwrap();
    assert!(export.index_xml.contains("<UTF8/>"));
}

#[test]
fn index_xml_has_foreign_key() {
    let inv = domestic_invoice();
    let export = to_gdpdu(&[inv], &default_config()).unwrap();
    assert!(export.index_xml.contains("<ForeignKey>"));
    assert!(export.index_xml.contains("<References>Kunden</References>"));
}

#[test]
fn index_xml_has_validity_period() {
    let inv = domestic_invoice();
    let export = to_gdpdu(&[inv], &default_config()).unwrap();
    assert!(export.index_xml.contains("<From>20240315</From>"));
    assert!(export.index_xml.contains("<To>20240315</To>"));
}

#[test]
fn index_xml_validity_spans_multiple_invoices() {
    let inv1 = domestic_invoice(); // March 15
    let inv2 = mixed_rate_invoice(); // April 1
    let export = to_gdpdu(&[inv1, inv2], &default_config()).unwrap();
    assert!(export.index_xml.contains("<From>20240315</From>"));
    assert!(export.index_xml.contains("<To>20240401</To>"));
}

#[test]
fn index_xml_has_column_delimiter() {
    let inv = domestic_invoice();
    let export = to_gdpdu(&[inv], &default_config()).unwrap();
    assert!(
        export
            .index_xml
            .contains("<ColumnDelimiter>;</ColumnDelimiter>")
    );
}

#[test]
fn index_xml_has_decimal_symbol() {
    let inv = domestic_invoice();
    let export = to_gdpdu(&[inv], &default_config()).unwrap();
    assert!(
        export
            .index_xml
            .contains("<DecimalSymbol>,</DecimalSymbol>")
    );
}

// ---------------------------------------------------------------------------
// kunden.csv Tests
// ---------------------------------------------------------------------------

#[test]
fn kunden_csv_contains_buyer() {
    let inv = domestic_invoice();
    let export = to_gdpdu(&[inv], &default_config()).unwrap();
    let kunden = &export.files[0].1;
    assert!(kunden.contains("\"Kunde AG\""));
    assert!(kunden.contains("\"München\""));
    assert!(kunden.contains("\"80331\""));
    assert!(kunden.contains("\"DE\""));
    assert!(kunden.contains("\"DE987654321\""));
}

#[test]
fn kunden_csv_deduplicates_customers() {
    let inv1 = domestic_invoice();
    let inv2 = domestic_invoice(); // Same buyer
    let export = to_gdpdu(&[inv1, inv2], &default_config()).unwrap();
    let kunden = &export.files[0].1;
    let line_count = kunden.lines().count();
    assert_eq!(
        line_count, 1,
        "expected 1 deduplicated customer, got {}",
        line_count
    );
}

#[test]
fn kunden_csv_multiple_customers() {
    let inv1 = domestic_invoice();
    let inv2 = mixed_rate_invoice(); // Different buyer
    let export = to_gdpdu(&[inv1, inv2], &default_config()).unwrap();
    let kunden = &export.files[0].1;
    let line_count = kunden.lines().count();
    assert_eq!(line_count, 2, "expected 2 customers, got {}", line_count);
}

#[test]
fn kunden_csv_uses_crlf() {
    let inv = domestic_invoice();
    let export = to_gdpdu(&[inv], &default_config()).unwrap();
    let kunden = &export.files[0].1;
    assert!(kunden.contains("\r\n"), "expected CRLF in kunden.csv");
}

// ---------------------------------------------------------------------------
// rechnungsausgang.csv Tests
// ---------------------------------------------------------------------------

#[test]
fn rechnungsausgang_contains_invoice_number() {
    let inv = domestic_invoice();
    let export = to_gdpdu(&[inv], &default_config()).unwrap();
    let rechnungen = &export.files[1].1;
    assert!(rechnungen.contains("\"RE-2024-001\""));
}

#[test]
fn rechnungsausgang_date_format() {
    let inv = domestic_invoice();
    let export = to_gdpdu(&[inv], &default_config()).unwrap();
    let rechnungen = &export.files[1].1;
    assert!(
        rechnungen.contains("15.03.2024"),
        "expected DD.MM.YYYY date format"
    );
}

#[test]
fn rechnungsausgang_amounts() {
    let inv = domestic_invoice();
    let export = to_gdpdu(&[inv], &default_config()).unwrap();
    let rechnungen = &export.files[1].1;
    // 10 * 150 = 1500 net, 19% = 285, gross = 1785
    assert!(rechnungen.contains("1500,00"), "expected net 1500,00");
    assert!(rechnungen.contains("19,00"), "expected tax rate 19,00");
    assert!(rechnungen.contains("285,00"), "expected tax amount 285,00");
    assert!(rechnungen.contains("1785,00"), "expected gross 1785,00");
}

#[test]
fn rechnungsausgang_currency() {
    let inv = domestic_invoice();
    let export = to_gdpdu(&[inv], &default_config()).unwrap();
    let rechnungen = &export.files[1].1;
    assert!(rechnungen.contains("\"EUR\""));
}

#[test]
fn rechnungsausgang_type_code() {
    let inv = domestic_invoice();
    let export = to_gdpdu(&[inv], &default_config()).unwrap();
    let rechnungen = &export.files[1].1;
    assert!(
        rechnungen.contains("\"380\""),
        "expected invoice type code 380"
    );
}

#[test]
fn rechnungsausgang_due_date() {
    let inv = domestic_invoice();
    let export = to_gdpdu(&[inv], &default_config()).unwrap();
    let rechnungen = &export.files[1].1;
    assert!(rechnungen.contains("15.04.2024"), "expected due date");
}

#[test]
fn rechnungsausgang_service_date() {
    let inv = domestic_invoice();
    let export = to_gdpdu(&[inv], &default_config()).unwrap();
    let rechnungen = &export.files[1].1;
    // tax_point_date = 2024-03-15
    assert!(rechnungen.contains("15.03.2024"), "expected service date");
}

#[test]
fn rechnungsausgang_customer_reference() {
    let inv = domestic_invoice();
    let export = to_gdpdu(&[inv], &default_config()).unwrap();
    let rechnungen = &export.files[1].1;
    assert!(
        rechnungen.contains("\"K-0001\""),
        "expected customer reference"
    );
    assert!(
        rechnungen.contains("\"Kunde AG\""),
        "expected customer name"
    );
}

#[test]
fn rechnungsausgang_mixed_rates_two_rows() {
    let inv = mixed_rate_invoice();
    let export = to_gdpdu(&[inv], &default_config()).unwrap();
    let rechnungen = &export.files[1].1;
    let line_count = rechnungen.lines().count();
    assert_eq!(
        line_count, 2,
        "expected 2 rows for mixed rate invoice, got {}",
        line_count
    );
}

#[test]
fn rechnungsausgang_mixed_rate_amounts() {
    let inv = mixed_rate_invoice();
    let export = to_gdpdu(&[inv], &default_config()).unwrap();
    let rechnungen = &export.files[1].1;
    // Line 1: 5 * 30 = 150 net, 7% = 10.50
    assert!(rechnungen.contains("150,00"), "expected 7% net amount");
    assert!(rechnungen.contains("10,50"), "expected 7% tax amount");
    // Line 2: 2 * 200 = 400 net, 19% = 76
    assert!(rechnungen.contains("400,00"), "expected 19% net amount");
    assert!(rechnungen.contains("76,00"), "expected 19% tax amount");
}

#[test]
fn rechnungsausgang_uses_crlf() {
    let inv = domestic_invoice();
    let export = to_gdpdu(&[inv], &default_config()).unwrap();
    let rechnungen = &export.files[1].1;
    assert!(rechnungen.contains("\r\n"), "expected CRLF");
    let without_cr = rechnungen.replace("\r\n", "");
    assert!(!without_cr.contains('\n'), "found bare LF without CR");
}

#[test]
fn rechnungsausgang_no_header_row() {
    let inv = domestic_invoice();
    let export = to_gdpdu(&[inv], &default_config()).unwrap();
    let rechnungen = &export.files[1].1;
    // First line should start with the invoice number, not column headers
    let first_line = rechnungen.lines().next().unwrap();
    assert!(
        first_line.starts_with("\"RE-2024-001\""),
        "CSV should have no header row"
    );
}

// ---------------------------------------------------------------------------
// Multiple Invoices
// ---------------------------------------------------------------------------

#[test]
fn multiple_invoices_export() {
    let inv1 = domestic_invoice();
    let inv2 = mixed_rate_invoice();
    let export = to_gdpdu(&[inv1, inv2], &default_config()).unwrap();
    let rechnungen = &export.files[1].1;
    let line_count = rechnungen.lines().count();
    // 1 row for domestic + 2 rows for mixed = 3
    assert_eq!(line_count, 3, "expected 3 data rows total");
}

// ---------------------------------------------------------------------------
// Invoice Without Totals
// ---------------------------------------------------------------------------

#[test]
fn invoice_without_totals_returns_error() {
    let mut inv = domestic_invoice();
    inv.totals = None;
    let result = to_gdpdu(&[inv], &default_config());
    assert!(result.is_err());
}
