//! DATEV EXTF Buchungsstapel CSV generation.

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use super::accounts::{self, ChartOfAccounts};
use super::bu_key;
use crate::core::{Invoice, InvoiceTypeCode, RechnungError, TaxCategory};

/// Configuration for DATEV EXTF export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatevConfig {
    /// DATEV consultant number (Beraternummer), min 1001.
    pub consultant_number: u32,
    /// DATEV client number (Mandantennummer).
    pub client_number: u32,
    /// Start of fiscal year (Wirtschaftsjahr-Beginn).
    pub fiscal_year_start: NaiveDate,
    /// G/L account length (Sachkontenlänge), typically 4.
    pub account_length: u8,
    /// Chart of accounts (SKR03 or SKR04).
    pub chart: ChartOfAccounts,
    /// Default debitor account number for customers without a specific one.
    /// Debitor accounts are typically 10000-69999.
    pub default_debitor: u32,
    /// Source identifier for the header (Herkunft), max 2 chars.
    pub source: String,
    /// Name of the exporting system (Exportiert von), max 25 chars.
    pub exported_by: String,
    /// Batch description (Bezeichnung), max 30 chars.
    pub description: String,
    /// Lock postings on import (Festschreibung).
    pub lock_postings: bool,
}

impl Default for DatevConfig {
    fn default() -> Self {
        Self {
            consultant_number: 1001,
            client_number: 1,
            fiscal_year_start: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            account_length: 4,
            chart: ChartOfAccounts::SKR03,
            default_debitor: 10000,
            source: "RE".into(),
            exported_by: String::new(),
            description: "Buchungsstapel".into(),
            lock_postings: false,
        }
    }
}

/// Builder for [`DatevConfig`].
///
/// # Example
///
/// ```
/// use faktura::datev::DatevConfigBuilder;
/// use chrono::NaiveDate;
///
/// let config = DatevConfigBuilder::new(12345, 99999)
///     .fiscal_year_start(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap())
///     .exported_by("faktura")
///     .build();
/// ```
pub struct DatevConfigBuilder {
    config: DatevConfig,
}

impl DatevConfigBuilder {
    /// Create a new builder with required consultant and client numbers.
    pub fn new(consultant_number: u32, client_number: u32) -> Self {
        Self {
            config: DatevConfig {
                consultant_number,
                client_number,
                ..Default::default()
            },
        }
    }

    /// Set the fiscal year start date.
    pub fn fiscal_year_start(mut self, date: NaiveDate) -> Self {
        self.config.fiscal_year_start = date;
        self
    }

    /// Set the G/L account length (typically 4).
    pub fn account_length(mut self, len: u8) -> Self {
        self.config.account_length = len;
        self
    }

    /// Set the chart of accounts.
    pub fn chart(mut self, chart: ChartOfAccounts) -> Self {
        self.config.chart = chart;
        self
    }

    /// Set the default debitor account.
    pub fn default_debitor(mut self, account: u32) -> Self {
        self.config.default_debitor = account;
        self
    }

    /// Set the source identifier (max 2 chars).
    pub fn source(mut self, source: impl Into<String>) -> Self {
        self.config.source = source.into();
        self
    }

    /// Set the "exported by" label (max 25 chars).
    pub fn exported_by(mut self, name: impl Into<String>) -> Self {
        self.config.exported_by = name.into();
        self
    }

    /// Set the batch description (max 30 chars).
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.config.description = desc.into();
        self
    }

    /// Enable posting lock on import.
    pub fn lock_postings(mut self, lock: bool) -> Self {
        self.config.lock_postings = lock;
        self
    }

    /// Build the configuration.
    pub fn build(self) -> DatevConfig {
        self.config
    }
}

/// A single DATEV Buchungsstapel row (intermediate representation).
#[derive(Debug, Clone)]
pub struct DatevRow {
    /// Gross amount (always positive).
    pub amount: Decimal,
    /// S = Soll (debit), H = Haben (credit).
    pub debit_credit: DebitCredit,
    /// Account number (Konto).
    pub account: u32,
    /// Contra account (Gegenkonto).
    pub contra_account: u32,
    /// BU-Schlüssel (tax key), empty if Automatikkonto.
    pub bu_key: Option<u8>,
    /// Document date.
    pub date: NaiveDate,
    /// Document number (Belegfeld 1).
    pub document_number: String,
    /// Posting text (Buchungstext), max 60 chars.
    pub posting_text: String,
    /// Service date (Leistungsdatum).
    pub service_date: Option<NaiveDate>,
    /// Due date (Fälligkeit).
    pub due_date: Option<NaiveDate>,
    /// EU country + VAT ID (for EU transactions).
    pub eu_vat_id: Option<String>,
    /// Generalumkehr flag (for storno bookings).
    pub general_reversal: bool,
}

/// Debit/Credit indicator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebitCredit {
    /// Soll (debit).
    Soll,
    /// Haben (credit).
    Haben,
}

impl DebitCredit {
    fn code(&self) -> &'static str {
        match self {
            Self::Soll => "S",
            Self::Haben => "H",
        }
    }
}

/// Generate a DATEV EXTF Buchungsstapel CSV from a set of invoices.
///
/// Returns the CSV as a string (ISO-8859-1 compatible content, using CRLF line endings).
/// The caller is responsible for encoding to ISO-8859-1 bytes if needed.
pub fn to_extf(invoices: &[Invoice], config: &DatevConfig) -> Result<String, RechnungError> {
    if invoices.is_empty() {
        return Err(RechnungError::Builder("no invoices to export".into()));
    }

    // Determine period from invoice dates
    let (period_start, period_end) = date_range(invoices);

    let mut rows = Vec::new();
    for inv in invoices {
        let inv_rows = invoice_to_rows(inv, config)?;
        rows.extend(inv_rows);
    }

    let mut out = String::new();

    // Line 1: EXTF header
    write_header(&mut out, config, period_start, period_end);

    // Line 2: Column headers
    write_column_headers(&mut out);

    // Lines 3+: Data rows
    for row in &rows {
        write_data_row(&mut out, row);
    }

    Ok(out)
}

/// Convert a single invoice into one or more DATEV rows.
///
/// For invoices with multiple tax rates, produces one row per line/tax group.
fn invoice_to_rows(inv: &Invoice, config: &DatevConfig) -> Result<Vec<DatevRow>, RechnungError> {
    let totals = inv.totals.as_ref().ok_or_else(|| {
        RechnungError::Builder(format!(
            "invoice {} has no calculated totals — call calculate_totals() first",
            inv.number
        ))
    })?;

    // Determine debit/credit direction
    let is_credit_note = inv.type_code == InvoiceTypeCode::CreditNote;

    let mut rows = Vec::new();

    // Strategy: one row per VAT breakdown group, booking gross amounts.
    // For Automatikkonten (8400, 4400, etc.) the BU key is omitted.
    // For non-Automatikkonten, the BU key is set.
    for vb in &totals.vat_breakdown {
        let gross = vb.taxable_amount + vb.tax_amount;
        if gross.is_zero() {
            continue;
        }

        let mapping =
            accounts::revenue_account(config.chart, inv.vat_scenario, vb.category, vb.rate);

        let bu_key = if mapping.is_automatik {
            None
        } else {
            bu_key::bu_schluessel(vb.category, vb.rate).map(|k| k.0)
        };

        // Posting text: use first line item name or invoice number
        let posting_text = build_posting_text(inv);

        let (debit_credit, account, contra_account) = if is_credit_note {
            // Credit note: flip direction (H = credit the debitor)
            (
                DebitCredit::Haben,
                config.default_debitor,
                mapping.revenue_account,
            )
        } else {
            // Normal invoice: S = debit the debitor
            (
                DebitCredit::Soll,
                config.default_debitor,
                mapping.revenue_account,
            )
        };

        let eu_vat_id = match vb.category {
            TaxCategory::IntraCommunitySupply | TaxCategory::ReverseCharge => {
                inv.buyer.vat_id.clone()
            }
            _ => None,
        };

        rows.push(DatevRow {
            amount: gross.abs(),
            debit_credit,
            account,
            contra_account,
            bu_key,
            date: inv.issue_date,
            document_number: truncate(&inv.number, 36),
            posting_text: truncate(&posting_text, 60),
            service_date: inv.tax_point_date,
            due_date: inv.due_date,
            eu_vat_id,
            general_reversal: false,
        });
    }

    Ok(rows)
}

fn build_posting_text(inv: &Invoice) -> String {
    if inv.lines.len() == 1 {
        format!("{} {}", inv.number, inv.lines[0].item_name)
    } else {
        inv.number.clone()
    }
}

fn date_range(invoices: &[Invoice]) -> (NaiveDate, NaiveDate) {
    let mut min = invoices[0].issue_date;
    let mut max = invoices[0].issue_date;
    for inv in invoices {
        if inv.issue_date < min {
            min = inv.issue_date;
        }
        if inv.issue_date > max {
            max = inv.issue_date;
        }
    }
    (min, max)
}

fn write_header(
    out: &mut String,
    config: &DatevConfig,
    period_start: NaiveDate,
    period_end: NaiveDate,
) {
    let now = chrono::Local::now().format("%Y%m%d%H%M%S000");
    let fy = config.fiscal_year_start.format("%Y%m%d");
    let ps = period_start.format("%Y%m%d");
    let pe = period_end.format("%Y%m%d");

    // 31 fields, semicolon-separated
    out.push_str(&format!(
        "\"EXTF\";700;21;\"Buchungsstapel\";13;{now};;\"{}\";\"{}\";\"\";\
         {};{};{fy};{};{ps};{pe};\"{}\";\"\";1;0;{};\"EUR\";;\"\";;\
         ;\"{}\";;;\"\"",
        truncate(&config.source, 2),
        truncate(&config.exported_by, 25),
        config.consultant_number,
        config.client_number,
        config.account_length,
        truncate(&config.description, 30),
        if config.lock_postings { 1 } else { 0 },
        config.chart.code(),
    ));
    out.push_str("\r\n");
}

/// The official DATEV column header line (116+ fields).
/// We output all standard fields to ensure compatibility.
fn write_column_headers(out: &mut String) {
    let headers = [
        "Umsatz (ohne Soll/Haben-Kz)",
        "Soll/Haben-Kennzeichen",
        "WKZ Umsatz",
        "Kurs",
        "Basisumsatz",
        "WKZ Basisumsatz",
        "Konto",
        "Gegenkonto (ohne BU-Schlüssel)",
        "BU-Schlüssel",
        "Belegdatum",
        "Belegfeld 1",
        "Belegfeld 2",
        "Skonto",
        "Buchungstext",
        // Fields 15-114: we output empty headers for compatibility
    ];

    // Write the named headers
    for (i, h) in headers.iter().enumerate() {
        if i > 0 {
            out.push(';');
        }
        out.push_str(h);
    }

    // Pad remaining fields (15 through 120) with empty separators
    for _ in headers.len()..120 {
        out.push(';');
    }

    out.push_str("\r\n");
}

fn write_data_row(out: &mut String, row: &DatevRow) {
    // Field 1: Umsatz — decimal with comma separator, 2 decimal places
    out.push_str(&format_amount(row.amount));
    out.push(';');

    // Field 2: Soll/Haben
    out.push('"');
    out.push_str(row.debit_credit.code());
    out.push('"');
    out.push(';');

    // Fields 3-6: WKZ, Kurs, Basisumsatz, WKZ Basisumsatz — empty for EUR
    out.push_str(";;;;");

    // Field 7: Konto
    out.push_str(&row.account.to_string());
    out.push(';');

    // Field 8: Gegenkonto
    out.push_str(&row.contra_account.to_string());
    out.push(';');

    // Field 9: BU-Schlüssel
    if let Some(bu) = row.bu_key {
        out.push_str(&bu.to_string());
    }
    out.push(';');

    // Field 10: Belegdatum (DDMM format)
    out.push_str(&row.date.format("%d%m").to_string());
    out.push(';');

    // Field 11: Belegfeld 1 (invoice number)
    out.push('"');
    out.push_str(&row.document_number);
    out.push('"');
    out.push(';');

    // Field 12: Belegfeld 2 — empty
    out.push(';');

    // Field 13: Skonto — empty
    out.push(';');

    // Field 14: Buchungstext
    out.push('"');
    out.push_str(&row.posting_text);
    out.push('"');

    // Fields 15-120: mostly empty, but we need specific ones
    // Pad fields 15-39
    for _ in 14..39 {
        out.push(';');
    }

    // Field 40: EU-Land u. USt-IdNr.
    if let Some(ref vat_id) = row.eu_vat_id {
        out.push('"');
        out.push_str(vat_id);
        out.push('"');
    }
    out.push(';');

    // Fields 41-114: empty
    for _ in 40..114 {
        out.push(';');
    }

    // Field 115: Leistungsdatum (DDMMYYYY)
    if let Some(d) = row.service_date {
        out.push_str(&d.format("%d%m%Y").to_string());
    }
    out.push(';');

    // Field 116: Datum Zuord. — empty
    out.push(';');

    // Field 117: Fälligkeit (DDMMYYYY)
    if let Some(d) = row.due_date {
        out.push_str(&d.format("%d%m%Y").to_string());
    }
    out.push(';');

    // Field 118: Generalumkehr
    if row.general_reversal {
        out.push('1');
    }

    // Fields 119-120: empty
    out.push_str(";;");

    out.push_str("\r\n");
}

/// Format a Decimal as German number: comma separator, 2 decimal places.
fn format_amount(d: Decimal) -> String {
    let scaled = d.round_dp(2);
    let s = format!("{:.2}", scaled);
    s.replace('.', ",")
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        s[..max].to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_amount_basic() {
        assert_eq!(format_amount(Decimal::new(119000, 2)), "1190,00");
        assert_eq!(format_amount(Decimal::new(2495, 2)), "24,95");
        assert_eq!(format_amount(Decimal::new(100, 0)), "100,00");
    }

    #[test]
    fn format_amount_rounds() {
        assert_eq!(format_amount(Decimal::new(123456, 3)), "123,46");
    }

    #[test]
    fn truncate_short() {
        assert_eq!(truncate("abc", 5), "abc");
    }

    #[test]
    fn truncate_long() {
        assert_eq!(truncate("abcdef", 3), "abc");
    }
}
