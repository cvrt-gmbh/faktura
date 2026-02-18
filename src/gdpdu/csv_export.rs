//! CSV data file generation for GDPdU export.
//!
//! Generates headerless CSV files with semicolon separators and German locale
//! (comma decimal separator, period digit grouping).

use rust_decimal::Decimal;
use std::collections::BTreeMap;

use crate::core::{Invoice, Party, RechnungError};

/// Generate the kunden.csv and rechnungsausgang.csv content.
pub fn generate_csvs(invoices: &[Invoice]) -> Result<(String, String), RechnungError> {
    let kunden = generate_kunden_csv(invoices);
    let rechnungen = generate_rechnungsausgang_csv(invoices)?;
    Ok((kunden, rechnungen))
}

/// Generate kunden.csv — unique customers extracted from invoices.
///
/// Columns: Kundenkontonummer;Kundenname;Strasse;PLZ;Ort;Land;UStIdNr
fn generate_kunden_csv(invoices: &[Invoice]) -> String {
    // Deduplicate customers by name (since we don't have a customer ID)
    let mut customers: BTreeMap<String, &Party> = BTreeMap::new();
    for inv in invoices {
        customers
            .entry(inv.buyer.name.clone())
            .or_insert(&inv.buyer);
    }

    let mut out = String::new();
    for (i, (_name, party)) in customers.iter().enumerate() {
        let customer_id = format!("K-{:04}", i + 1);
        csv_field_str(&mut out, &customer_id);
        out.push(';');
        csv_field_str(&mut out, &party.name);
        out.push(';');
        csv_field_str(&mut out, party.address.street.as_deref().unwrap_or(""));
        out.push(';');
        csv_field_str(&mut out, &party.address.postal_code);
        out.push(';');
        csv_field_str(&mut out, &party.address.city);
        out.push(';');
        csv_field_str(&mut out, &party.address.country_code);
        out.push(';');
        csv_field_str(&mut out, party.vat_id.as_deref().unwrap_or(""));
        out.push_str("\r\n");
    }
    out
}

/// Generate rechnungsausgang.csv — one row per VAT breakdown group per invoice.
///
/// Columns: Belegnummer;Belegdatum;Faelligkeitsdatum;Leistungsdatum;
///          Kundenkontonummer;Kundenname;Buchungstext;
///          Nettobetrag;Steuersatz;Steuerbetrag;Bruttobetrag;
///          Waehrung;Belegtyp
fn generate_rechnungsausgang_csv(invoices: &[Invoice]) -> Result<String, RechnungError> {
    // Build customer ID lookup (same order as kunden_csv)
    let mut customer_ids: BTreeMap<String, String> = BTreeMap::new();
    let mut counter = 0usize;
    for inv in invoices {
        customer_ids
            .entry(inv.buyer.name.clone())
            .or_insert_with(|| {
                counter += 1;
                format!("K-{:04}", counter)
            });
    }

    let mut out = String::new();
    for inv in invoices {
        let totals = inv.totals.as_ref().ok_or_else(|| {
            RechnungError::Builder(format!(
                "invoice {} has no calculated totals — call calculate_totals() first",
                inv.number
            ))
        })?;

        let customer_id = customer_ids.get(&inv.buyer.name).ok_or_else(|| {
            RechnungError::Builder(format!("missing customer ID for '{}'", inv.buyer.name))
        })?;
        let type_code = inv.type_code.code().to_string();

        for vb in &totals.vat_breakdown {
            let gross = vb.taxable_amount + vb.tax_amount;
            if gross.is_zero() && vb.taxable_amount.is_zero() {
                continue;
            }

            let posting_text = if inv.lines.len() == 1 {
                inv.lines[0].item_name.clone()
            } else {
                inv.number.clone()
            };

            // Belegnummer
            csv_field_str(&mut out, &inv.number);
            out.push(';');
            // Belegdatum
            out.push_str(&inv.issue_date.format("%d.%m.%Y").to_string());
            out.push(';');
            // Faelligkeitsdatum
            if let Some(d) = inv.due_date {
                out.push_str(&d.format("%d.%m.%Y").to_string());
            }
            out.push(';');
            // Leistungsdatum
            if let Some(d) = inv.tax_point_date {
                out.push_str(&d.format("%d.%m.%Y").to_string());
            }
            out.push(';');
            // Kundenkontonummer
            csv_field_str(&mut out, customer_id);
            out.push(';');
            // Kundenname
            csv_field_str(&mut out, &inv.buyer.name);
            out.push(';');
            // Buchungstext
            csv_field_str(&mut out, &posting_text);
            out.push(';');
            // Nettobetrag
            csv_field_decimal(&mut out, vb.taxable_amount);
            out.push(';');
            // Steuersatz
            csv_field_decimal(&mut out, vb.rate);
            out.push(';');
            // Steuerbetrag
            csv_field_decimal(&mut out, vb.tax_amount);
            out.push(';');
            // Bruttobetrag
            csv_field_decimal(&mut out, gross);
            out.push(';');
            // Waehrung
            csv_field_str(&mut out, &inv.currency_code);
            out.push(';');
            // Belegtyp
            csv_field_str(&mut out, &type_code);
            out.push_str("\r\n");
        }
    }
    Ok(out)
}

fn csv_field_str(out: &mut String, value: &str) {
    out.push('"');
    // Escape internal double quotes
    for ch in value.chars() {
        if ch == '"' {
            out.push_str("\"\"");
        } else {
            out.push(ch);
        }
    }
    out.push('"');
}

fn csv_field_decimal(out: &mut String, d: Decimal) {
    let scaled = d.round_dp(2);
    let s = format!("{:.2}", scaled);
    out.push_str(&s.replace('.', ","));
}
