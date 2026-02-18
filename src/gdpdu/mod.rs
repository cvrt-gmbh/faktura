//! GDPdU/IDEA tax audit export.
//!
//! Generates `index.xml` and accompanying CSV files for German tax audits
//! (Grundsätze zum Datenzugriff und zur Prüfbarkeit digitaler Unterlagen).
//!
//! The export produces:
//! - `index.xml` — metadata describing tables, columns, and relationships
//! - `kunden.csv` — customer master data (Kundenstammdaten)
//! - `rechnungsausgang.csv` — outgoing invoices (Ausgangsrechnungen)
//!
//! # Example
//!
//! ```ignore
//! use faktura::gdpdu::*;
//!
//! let config = GdpduConfig {
//!     company_name: "ACME GmbH".into(),
//!     ..Default::default()
//! };
//! let export = to_gdpdu(&invoices, &config).unwrap();
//! // export.index_xml — the index.xml content
//! // export.files — vec of (filename, csv_content) pairs
//! ```

mod csv_export;
mod index_xml;

use crate::core::{Invoice, RechnungError};
use serde::{Deserialize, Serialize};

/// Configuration for GDPdU export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GdpduConfig {
    /// Company name (DataSupplier Name).
    pub company_name: String,
    /// Company location / country.
    pub location: String,
    /// Export comment / description.
    pub comment: String,
}

impl Default for GdpduConfig {
    fn default() -> Self {
        Self {
            company_name: String::new(),
            location: "Deutschland".into(),
            comment: "GDPdU-Export Ausgangsrechnungen".into(),
        }
    }
}

/// Result of a GDPdU export.
#[derive(Debug, Clone)]
pub struct GdpduExport {
    /// The `index.xml` content.
    pub index_xml: String,
    /// Data files: Vec of (filename, content) pairs.
    pub files: Vec<(String, String)>,
    /// The DTD content (gdpdu-01-08-2002.dtd) to include alongside.
    pub dtd: &'static str,
}

/// The standard GDPdU DTD (version 2002-08-01).
pub const GDPDU_DTD: &str = include_str!("gdpdu-01-08-2002.dtd");

/// Generate a GDPdU export (index.xml + CSV files) from a set of invoices.
pub fn to_gdpdu(invoices: &[Invoice], config: &GdpduConfig) -> Result<GdpduExport, RechnungError> {
    if invoices.is_empty() {
        return Err(RechnungError::Builder("no invoices to export".into()));
    }

    // Generate CSV data
    let (kunden_csv, invoice_csv) = csv_export::generate_csvs(invoices)?;

    // Generate index.xml
    let index_xml = index_xml::generate_index_xml(invoices, config)?;

    Ok(GdpduExport {
        index_xml,
        files: vec![
            ("kunden.csv".into(), kunden_csv),
            ("rechnungsausgang.csv".into(), invoice_csv),
        ],
        dtd: GDPDU_DTD,
    })
}
