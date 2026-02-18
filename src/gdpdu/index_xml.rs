//! GDPdU index.xml generation using quick-xml.

use quick_xml::Writer;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use std::io::{Cursor, Write};

use super::GdpduConfig;
use crate::core::{Invoice, RechnungError};

/// Generate the index.xml content for the GDPdU export.
pub fn generate_index_xml(
    invoices: &[Invoice],
    config: &GdpduConfig,
) -> Result<String, RechnungError> {
    let mut writer = Writer::new_with_indent(Cursor::new(Vec::new()), b' ', 2);

    // XML declaration
    writer
        .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
        .map_err(xml_err)?;

    // DOCTYPE — write via the Cursor to keep its position in sync
    writer
        .get_mut()
        .write_all(b"\n<!DOCTYPE DataSet SYSTEM \"gdpdu-01-08-2002.dtd\">\n")
        .map_err(xml_err)?;

    // <DataSet>
    writer
        .write_event(Event::Start(BytesStart::new("DataSet")))
        .map_err(xml_err)?;

    // <Version>1.0</Version>
    write_text_element(&mut writer, "Version", "1.0")?;

    // <DataSupplier>
    if !config.company_name.is_empty() {
        writer
            .write_event(Event::Start(BytesStart::new("DataSupplier")))
            .map_err(xml_err)?;
        write_text_element(&mut writer, "Name", &config.company_name)?;
        write_text_element(&mut writer, "Location", &config.location)?;
        write_text_element(&mut writer, "Comment", &config.comment)?;
        writer
            .write_event(Event::End(BytesEnd::new("DataSupplier")))
            .map_err(xml_err)?;
    }

    // Determine validity period from invoices
    let (period_from, period_to) = date_range(invoices);

    // <Media>
    writer
        .write_event(Event::Start(BytesStart::new("Media")))
        .map_err(xml_err)?;
    write_text_element(&mut writer, "Name", "Datenexport")?;

    // Table: Kunden
    write_kunden_table(&mut writer)?;

    // Table: Rechnungsausgang
    write_rechnungsausgang_table(&mut writer, &period_from, &period_to)?;

    writer
        .write_event(Event::End(BytesEnd::new("Media")))
        .map_err(xml_err)?;

    // </DataSet>
    writer
        .write_event(Event::End(BytesEnd::new("DataSet")))
        .map_err(xml_err)?;

    let buf = writer.into_inner().into_inner();
    String::from_utf8(buf).map_err(|e| RechnungError::Builder(format!("UTF-8 error: {e}")))
}

fn write_kunden_table(writer: &mut Writer<Cursor<Vec<u8>>>) -> Result<(), RechnungError> {
    writer
        .write_event(Event::Start(BytesStart::new("Table")))
        .map_err(xml_err)?;

    write_text_element(writer, "URL", "kunden.csv")?;
    write_text_element(writer, "Name", "Kunden")?;
    write_text_element(writer, "Description", "Kundenstammdaten")?;

    // Encoding
    writer
        .write_event(Event::Empty(BytesStart::new("UTF8")))
        .map_err(xml_err)?;
    write_text_element(writer, "DecimalSymbol", ",")?;
    write_text_element(writer, "DigitGroupingSymbol", ".")?;

    // VariableLength
    writer
        .write_event(Event::Start(BytesStart::new("VariableLength")))
        .map_err(xml_err)?;
    write_text_element(writer, "ColumnDelimiter", ";")?;
    write_text_element(writer, "TextEncapsulator", "\"")?;

    // Primary key
    write_variable_pk(writer, "Kundenkontonummer", None, ColType::AlphaNumeric)?;

    // Columns
    write_variable_col(writer, "Kundenname", None, ColType::AlphaNumeric)?;
    write_variable_col(writer, "Strasse", None, ColType::AlphaNumeric)?;
    write_variable_col(writer, "PLZ", None, ColType::AlphaNumeric)?;
    write_variable_col(writer, "Ort", None, ColType::AlphaNumeric)?;
    write_variable_col(writer, "Land", None, ColType::AlphaNumeric)?;
    write_variable_col(writer, "UStIdNr", None, ColType::AlphaNumeric)?;

    writer
        .write_event(Event::End(BytesEnd::new("VariableLength")))
        .map_err(xml_err)?;
    writer
        .write_event(Event::End(BytesEnd::new("Table")))
        .map_err(xml_err)?;
    Ok(())
}

fn write_rechnungsausgang_table(
    writer: &mut Writer<Cursor<Vec<u8>>>,
    period_from: &str,
    period_to: &str,
) -> Result<(), RechnungError> {
    writer
        .write_event(Event::Start(BytesStart::new("Table")))
        .map_err(xml_err)?;

    write_text_element(writer, "URL", "rechnungsausgang.csv")?;
    write_text_element(writer, "Name", "Rechnungsausgang")?;
    write_text_element(writer, "Description", "Ausgangsrechnungen")?;

    // Validity period
    writer
        .write_event(Event::Start(BytesStart::new("Validity")))
        .map_err(xml_err)?;
    writer
        .write_event(Event::Start(BytesStart::new("Range")))
        .map_err(xml_err)?;
    write_text_element(writer, "From", period_from)?;
    write_text_element(writer, "To", period_to)?;
    writer
        .write_event(Event::End(BytesEnd::new("Range")))
        .map_err(xml_err)?;
    write_text_element(writer, "Format", "YYYYMMDD")?;
    writer
        .write_event(Event::End(BytesEnd::new("Validity")))
        .map_err(xml_err)?;

    // Encoding
    writer
        .write_event(Event::Empty(BytesStart::new("UTF8")))
        .map_err(xml_err)?;
    write_text_element(writer, "DecimalSymbol", ",")?;
    write_text_element(writer, "DigitGroupingSymbol", ".")?;

    // VariableLength
    writer
        .write_event(Event::Start(BytesStart::new("VariableLength")))
        .map_err(xml_err)?;
    write_text_element(writer, "ColumnDelimiter", ";")?;
    write_text_element(writer, "TextEncapsulator", "\"")?;

    // Primary key
    write_variable_pk(
        writer,
        "Belegnummer",
        Some("Rechnungsnummer"),
        ColType::AlphaNumeric,
    )?;

    // Columns
    write_variable_col(writer, "Belegdatum", Some("Rechnungsdatum"), ColType::Date)?;
    write_variable_col(writer, "Faelligkeitsdatum", None, ColType::Date)?;
    write_variable_col(
        writer,
        "Leistungsdatum",
        Some("Liefer-/Leistungsdatum"),
        ColType::Date,
    )?;
    write_variable_col(
        writer,
        "Kundenkontonummer",
        Some("Debitorennummer"),
        ColType::AlphaNumeric,
    )?;
    write_variable_col(writer, "Kundenname", None, ColType::AlphaNumeric)?;
    write_variable_col(
        writer,
        "Buchungstext",
        Some("Rechnungsbetreff"),
        ColType::AlphaNumeric,
    )?;
    write_variable_col(writer, "Nettobetrag", None, ColType::Numeric2)?;
    write_variable_col(
        writer,
        "Steuersatz",
        Some("USt-Satz in Prozent"),
        ColType::Numeric2,
    )?;
    write_variable_col(writer, "Steuerbetrag", None, ColType::Numeric2)?;
    write_variable_col(writer, "Bruttobetrag", None, ColType::Numeric2)?;
    write_variable_col(writer, "Waehrung", None, ColType::AlphaNumeric)?;
    write_variable_col(
        writer,
        "Belegtyp",
        Some("UNTDID 1001 Belegtyp"),
        ColType::AlphaNumeric,
    )?;

    // ForeignKey to Kunden
    writer
        .write_event(Event::Start(BytesStart::new("ForeignKey")))
        .map_err(xml_err)?;
    write_text_element(writer, "Name", "Kundenkontonummer")?;
    write_text_element(writer, "References", "Kunden")?;
    writer
        .write_event(Event::End(BytesEnd::new("ForeignKey")))
        .map_err(xml_err)?;

    writer
        .write_event(Event::End(BytesEnd::new("VariableLength")))
        .map_err(xml_err)?;
    writer
        .write_event(Event::End(BytesEnd::new("Table")))
        .map_err(xml_err)?;
    Ok(())
}

enum ColType {
    AlphaNumeric,
    Numeric2,
    Date,
}

fn write_variable_pk(
    writer: &mut Writer<Cursor<Vec<u8>>>,
    name: &str,
    desc: Option<&str>,
    col_type: ColType,
) -> Result<(), RechnungError> {
    writer
        .write_event(Event::Start(BytesStart::new("VariablePrimaryKey")))
        .map_err(xml_err)?;
    write_text_element(writer, "Name", name)?;
    if let Some(d) = desc {
        write_text_element(writer, "Description", d)?;
    }
    write_col_type(writer, col_type)?;
    writer
        .write_event(Event::End(BytesEnd::new("VariablePrimaryKey")))
        .map_err(xml_err)?;
    Ok(())
}

fn write_variable_col(
    writer: &mut Writer<Cursor<Vec<u8>>>,
    name: &str,
    desc: Option<&str>,
    col_type: ColType,
) -> Result<(), RechnungError> {
    writer
        .write_event(Event::Start(BytesStart::new("VariableColumn")))
        .map_err(xml_err)?;
    write_text_element(writer, "Name", name)?;
    if let Some(d) = desc {
        write_text_element(writer, "Description", d)?;
    }
    write_col_type(writer, col_type)?;
    writer
        .write_event(Event::End(BytesEnd::new("VariableColumn")))
        .map_err(xml_err)?;
    Ok(())
}

fn write_col_type(
    writer: &mut Writer<Cursor<Vec<u8>>>,
    col_type: ColType,
) -> Result<(), RechnungError> {
    match col_type {
        ColType::AlphaNumeric => {
            writer
                .write_event(Event::Empty(BytesStart::new("AlphaNumeric")))
                .map_err(xml_err)?;
        }
        ColType::Numeric2 => {
            writer
                .write_event(Event::Start(BytesStart::new("Numeric")))
                .map_err(xml_err)?;
            write_text_element(writer, "Accuracy", "2")?;
            writer
                .write_event(Event::End(BytesEnd::new("Numeric")))
                .map_err(xml_err)?;
        }
        ColType::Date => {
            writer
                .write_event(Event::Start(BytesStart::new("Date")))
                .map_err(xml_err)?;
            write_text_element(writer, "Format", "DD.MM.YYYY")?;
            writer
                .write_event(Event::End(BytesEnd::new("Date")))
                .map_err(xml_err)?;
        }
    }
    Ok(())
}

fn write_text_element(
    writer: &mut Writer<Cursor<Vec<u8>>>,
    tag: &str,
    text: &str,
) -> Result<(), RechnungError> {
    writer
        .write_event(Event::Start(BytesStart::new(tag)))
        .map_err(xml_err)?;
    writer
        .write_event(Event::Text(BytesText::new(text)))
        .map_err(xml_err)?;
    writer
        .write_event(Event::End(BytesEnd::new(tag)))
        .map_err(xml_err)?;
    Ok(())
}

fn date_range(invoices: &[Invoice]) -> (String, String) {
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
    (
        min.format("%Y%m%d").to_string(),
        max.format("%Y%m%d").to_string(),
    )
}

fn xml_err(e: std::io::Error) -> RechnungError {
    RechnungError::Builder(format!("XML generation error: {e}"))
}
