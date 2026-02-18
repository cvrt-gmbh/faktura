use quick_xml::Writer;
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use rust_decimal::Decimal;
use std::io::Cursor;

use crate::core::RechnungError;

pub type XmlResult = Result<String, RechnungError>;

fn xml_io(e: std::io::Error) -> RechnungError {
    RechnungError::Builder(format!("XML write error: {e}"))
}

pub struct XmlWriter {
    writer: Writer<Cursor<Vec<u8>>>,
}

impl XmlWriter {
    pub fn new() -> Result<Self, RechnungError> {
        let mut writer = Writer::new_with_indent(Cursor::new(Vec::new()), b' ', 2);
        writer
            .write_event(Event::Decl(quick_xml::events::BytesDecl::new(
                "1.0",
                Some("UTF-8"),
                None,
            )))
            .map_err(xml_io)?;
        Ok(Self { writer })
    }

    pub fn into_string(self) -> Result<String, RechnungError> {
        let buf = self.writer.into_inner().into_inner();
        String::from_utf8(buf).map_err(|e| RechnungError::Builder(format!("XML UTF-8 error: {e}")))
    }

    pub fn start_element(&mut self, name: &str) -> Result<&mut Self, RechnungError> {
        let elem = BytesStart::new(name);
        self.writer
            .write_event(Event::Start(elem))
            .map_err(xml_io)?;
        Ok(self)
    }

    pub fn start_element_with_attrs(
        &mut self,
        name: &str,
        attrs: &[(&str, &str)],
    ) -> Result<&mut Self, RechnungError> {
        let mut elem = BytesStart::new(name);
        for (k, v) in attrs {
            elem.push_attribute((*k, *v));
        }
        self.writer
            .write_event(Event::Start(elem))
            .map_err(xml_io)?;
        Ok(self)
    }

    pub fn end_element(&mut self, name: &str) -> Result<&mut Self, RechnungError> {
        self.writer
            .write_event(Event::End(BytesEnd::new(name)))
            .map_err(xml_io)?;
        Ok(self)
    }

    pub fn text_element(&mut self, name: &str, text: &str) -> Result<&mut Self, RechnungError> {
        self.start_element(name)?;
        self.writer
            .write_event(Event::Text(BytesText::new(text)))
            .map_err(xml_io)?;
        self.end_element(name)
    }

    pub fn text_element_with_attrs(
        &mut self,
        name: &str,
        text: &str,
        attrs: &[(&str, &str)],
    ) -> Result<&mut Self, RechnungError> {
        self.start_element_with_attrs(name, attrs)?;
        self.writer
            .write_event(Event::Text(BytesText::new(text)))
            .map_err(xml_io)?;
        self.end_element(name)
    }

    /// Write a decimal amount with currencyID attribute.
    pub fn amount_element(
        &mut self,
        name: &str,
        amount: Decimal,
        currency: &str,
    ) -> Result<&mut Self, RechnungError> {
        self.text_element_with_attrs(name, &format_decimal(amount), &[("currencyID", currency)])
    }

    /// Write a quantity with unitCode attribute.
    pub fn quantity_element(
        &mut self,
        name: &str,
        qty: Decimal,
        unit: &str,
    ) -> Result<&mut Self, RechnungError> {
        self.text_element_with_attrs(name, &format_decimal(qty), &[("unitCode", unit)])
    }
}

/// Format a Decimal for XML output — always include at least 2 decimal places,
/// strip trailing zeros beyond that.
pub fn format_decimal(d: Decimal) -> String {
    // Normalize to remove trailing zeros, but ensure at least 2 decimal places
    let s = d.normalize().to_string();
    if let Some(dot_pos) = s.find('.') {
        let decimals = s.len() - dot_pos - 1;
        if decimals < 2 {
            format!("{s}{}", "0".repeat(2 - decimals))
        } else {
            s
        }
    } else {
        format!("{s}.00")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn format_decimal_cases() {
        assert_eq!(format_decimal(dec!(100)), "100.00");
        assert_eq!(format_decimal(dec!(1500.0)), "1500.00");
        assert_eq!(format_decimal(dec!(49.90)), "49.90");
        assert_eq!(format_decimal(dec!(1833.48)), "1833.48");
        assert_eq!(format_decimal(dec!(0.005)), "0.005");
        assert_eq!(format_decimal(dec!(19)), "19.00");
    }
}
