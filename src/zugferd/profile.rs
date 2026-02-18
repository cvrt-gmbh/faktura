use crate::core::*;
use crate::xrechnung;
use crate::xrechnung::cii_ns;
use crate::xrechnung::xml_utils::{XmlWriter, format_decimal};
use chrono::NaiveDate;

/// ZUGFeRD / Factur-X conformance profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZugferdProfile {
    /// Minimal machine-readable data (no line items).
    Minimum,
    /// Basic data without line items.
    BasicWl,
    /// Basic with line items.
    Basic,
    /// Full EN 16931 European norm (recommended for most use cases).
    EN16931,
    /// Extended profile (beyond EN 16931).
    Extended,
    /// XRechnung profile (German public sector).
    XRechnung,
}

impl ZugferdProfile {
    /// The URN identifier used in the CII XML `GuidelineSpecifiedDocumentContextParameter`.
    pub fn urn(&self) -> &'static str {
        match self {
            Self::Minimum => "urn:factur-x.eu:1p0:minimum",
            Self::BasicWl => "urn:factur-x.eu:1p0:basicwl",
            Self::Basic => "urn:cen.eu:en16931:2017#compliant#urn:factur-x.eu:1p0:basic",
            Self::EN16931 => "urn:cen.eu:en16931:2017",
            Self::Extended => "urn:cen.eu:en16931:2017#conformant#urn:factur-x.eu:1p0:extended",
            Self::XRechnung => xrechnung::XRECHNUNG_CUSTOMIZATION_ID,
        }
    }

    /// The XMP ConformanceLevel value.
    pub fn conformance_level(&self) -> &'static str {
        match self {
            Self::Minimum => "MINIMUM",
            Self::BasicWl => "BASIC WL",
            Self::Basic => "BASIC",
            Self::EN16931 => "EN 16931",
            Self::Extended => "EXTENDED",
            Self::XRechnung => "XRECHNUNG",
        }
    }

    /// The AFRelationship value for the PDF FileSpec.
    pub fn af_relationship(&self) -> &'static str {
        match self {
            Self::Minimum | Self::BasicWl => "Data",
            _ => "Alternative",
        }
    }
}

/// Generate ZUGFeRD/Factur-X CII XML for the given profile.
///
/// For **Minimum** and **BasicWL** profiles, generates a reduced XML
/// without line items (as required by the Factur-X specification).
/// For **Basic** and above, generates the full CII XML with all line items.
pub fn to_xml(invoice: &Invoice, profile: ZugferdProfile) -> Result<String, RechnungError> {
    match profile {
        ZugferdProfile::Minimum => to_minimum_xml(invoice),
        ZugferdProfile::BasicWl => to_basicwl_xml(invoice),
        _ => {
            let xml = xrechnung::to_cii_xml(invoice)?;
            Ok(xml.replace(xrechnung::XRECHNUNG_CUSTOMIZATION_ID, profile.urn()))
        }
    }
}

/// Generate Minimum profile CII XML (document-level data only, no line items).
fn to_minimum_xml(invoice: &Invoice) -> Result<String, RechnungError> {
    let totals = invoice.totals.as_ref().ok_or_else(|| {
        RechnungError::Builder("totals must be calculated before XML generation".into())
    })?;

    let currency = &invoice.currency_code;
    let mut w = XmlWriter::new()?;

    w.start_element_with_attrs(
        "rsm:CrossIndustryInvoice",
        &[
            ("xmlns:rsm", cii_ns::RSM),
            ("xmlns:ram", cii_ns::RAM),
            ("xmlns:qdt", cii_ns::QDT),
            ("xmlns:udt", cii_ns::UDT),
        ],
    )?;

    // ExchangedDocumentContext
    w.start_element("rsm:ExchangedDocumentContext")?;
    w.start_element("ram:GuidelineSpecifiedDocumentContextParameter")?;
    w.text_element("ram:ID", ZugferdProfile::Minimum.urn())?;
    w.end_element("ram:GuidelineSpecifiedDocumentContextParameter")?;
    w.end_element("rsm:ExchangedDocumentContext")?;

    // ExchangedDocument
    w.start_element("rsm:ExchangedDocument")?;
    w.text_element("ram:ID", &invoice.number)?;
    w.text_element("ram:TypeCode", &invoice.type_code.code().to_string())?;
    write_cii_date(&mut w, "ram:IssueDateTime", &invoice.issue_date)?;
    w.end_element("rsm:ExchangedDocument")?;

    // SupplyChainTradeTransaction (no line items for Minimum)
    w.start_element("rsm:SupplyChainTradeTransaction")?;

    // ApplicableHeaderTradeAgreement — seller/buyer name only
    w.start_element("ram:ApplicableHeaderTradeAgreement")?;
    if let Some(br) = &invoice.buyer_reference {
        w.text_element("ram:BuyerReference", br)?;
    }
    w.start_element("ram:SellerTradeParty")?;
    w.text_element("ram:Name", &invoice.seller.name)?;
    w.end_element("ram:SellerTradeParty")?;
    w.start_element("ram:BuyerTradeParty")?;
    w.text_element("ram:Name", &invoice.buyer.name)?;
    w.end_element("ram:BuyerTradeParty")?;
    w.end_element("ram:ApplicableHeaderTradeAgreement")?;

    // ApplicableHeaderTradeDelivery (empty)
    w.start_element("ram:ApplicableHeaderTradeDelivery")?;
    w.end_element("ram:ApplicableHeaderTradeDelivery")?;

    // ApplicableHeaderTradeSettlement — currency + totals only
    w.start_element("ram:ApplicableHeaderTradeSettlement")?;
    w.text_element("ram:InvoiceCurrencyCode", currency)?;
    w.start_element("ram:SpecifiedTradeSettlementHeaderMonetarySummation")?;
    w.text_element("ram:TaxBasisTotalAmount", &format_decimal(totals.net_total))?;
    w.text_element_with_attrs(
        "ram:TaxTotalAmount",
        &format_decimal(totals.vat_total),
        &[("currencyID", currency.as_str())],
    )?;
    w.text_element("ram:GrandTotalAmount", &format_decimal(totals.gross_total))?;
    w.text_element("ram:DuePayableAmount", &format_decimal(totals.amount_due))?;
    w.end_element("ram:SpecifiedTradeSettlementHeaderMonetarySummation")?;
    w.end_element("ram:ApplicableHeaderTradeSettlement")?;

    w.end_element("rsm:SupplyChainTradeTransaction")?;
    w.end_element("rsm:CrossIndustryInvoice")?;

    w.into_string()
}

/// Generate BasicWL profile CII XML (full party/settlement data, no line items).
fn to_basicwl_xml(invoice: &Invoice) -> Result<String, RechnungError> {
    let totals = invoice.totals.as_ref().ok_or_else(|| {
        RechnungError::Builder("totals must be calculated before XML generation".into())
    })?;

    let currency = &invoice.currency_code;
    let mut w = XmlWriter::new()?;

    w.start_element_with_attrs(
        "rsm:CrossIndustryInvoice",
        &[
            ("xmlns:rsm", cii_ns::RSM),
            ("xmlns:ram", cii_ns::RAM),
            ("xmlns:qdt", cii_ns::QDT),
            ("xmlns:udt", cii_ns::UDT),
        ],
    )?;

    // ExchangedDocumentContext
    w.start_element("rsm:ExchangedDocumentContext")?;
    w.start_element("ram:GuidelineSpecifiedDocumentContextParameter")?;
    w.text_element("ram:ID", ZugferdProfile::BasicWl.urn())?;
    w.end_element("ram:GuidelineSpecifiedDocumentContextParameter")?;
    w.end_element("rsm:ExchangedDocumentContext")?;

    // ExchangedDocument
    w.start_element("rsm:ExchangedDocument")?;
    w.text_element("ram:ID", &invoice.number)?;
    w.text_element("ram:TypeCode", &invoice.type_code.code().to_string())?;
    write_cii_date(&mut w, "ram:IssueDateTime", &invoice.issue_date)?;
    for note in &invoice.notes {
        w.start_element("ram:IncludedNote")?;
        w.text_element("ram:Content", note)?;
        w.end_element("ram:IncludedNote")?;
    }
    w.end_element("rsm:ExchangedDocument")?;

    // SupplyChainTradeTransaction (no line items for BasicWL)
    w.start_element("rsm:SupplyChainTradeTransaction")?;

    // ApplicableHeaderTradeAgreement — full party details
    w.start_element("ram:ApplicableHeaderTradeAgreement")?;
    if let Some(br) = &invoice.buyer_reference {
        w.text_element("ram:BuyerReference", br)?;
    }
    write_cii_party(&mut w, &invoice.seller, "ram:SellerTradeParty")?;
    write_cii_party(&mut w, &invoice.buyer, "ram:BuyerTradeParty")?;
    w.end_element("ram:ApplicableHeaderTradeAgreement")?;

    // ApplicableHeaderTradeDelivery
    w.start_element("ram:ApplicableHeaderTradeDelivery")?;
    if let Some(tpd) = &invoice.tax_point_date {
        w.start_element("ram:ActualDeliverySupplyChainEvent")?;
        write_cii_date(&mut w, "ram:OccurrenceDateTime", tpd)?;
        w.end_element("ram:ActualDeliverySupplyChainEvent")?;
    }
    w.end_element("ram:ApplicableHeaderTradeDelivery")?;

    // ApplicableHeaderTradeSettlement — full settlement with VAT breakdown
    w.start_element("ram:ApplicableHeaderTradeSettlement")?;
    w.text_element("ram:InvoiceCurrencyCode", currency)?;

    // Payment means
    if let Some(payment) = &invoice.payment {
        w.start_element("ram:SpecifiedTradeSettlementPaymentMeans")?;
        w.text_element("ram:TypeCode", &payment.means_code.code().to_string())?;
        if let Some(ct) = &payment.credit_transfer {
            w.start_element("ram:PayeePartyCreditorFinancialAccount")?;
            w.text_element("ram:IBANID", &ct.iban)?;
            w.end_element("ram:PayeePartyCreditorFinancialAccount")?;
        }
        w.end_element("ram:SpecifiedTradeSettlementPaymentMeans")?;
    }

    // VAT breakdown
    for bd in &totals.vat_breakdown {
        w.start_element("ram:ApplicableTradeTax")?;
        w.text_element("ram:CalculatedAmount", &format_decimal(bd.tax_amount))?;
        w.text_element("ram:TypeCode", "VAT")?;
        w.text_element("ram:BasisAmount", &format_decimal(bd.taxable_amount))?;
        w.text_element("ram:CategoryCode", bd.category.code())?;
        w.text_element("ram:RateApplicablePercent", &format_decimal(bd.rate))?;
        w.end_element("ram:ApplicableTradeTax")?;
    }

    // Payment terms
    if let Some(terms) = &invoice.payment_terms {
        w.start_element("ram:SpecifiedTradePaymentTerms")?;
        w.text_element("ram:Description", terms)?;
        if let Some(due) = &invoice.due_date {
            write_cii_date(&mut w, "ram:DueDateDateTime", due)?;
        }
        w.end_element("ram:SpecifiedTradePaymentTerms")?;
    }

    // Monetary summation
    w.start_element("ram:SpecifiedTradeSettlementHeaderMonetarySummation")?;
    w.text_element(
        "ram:LineTotalAmount",
        &format_decimal(totals.line_net_total),
    )?;
    w.text_element("ram:TaxBasisTotalAmount", &format_decimal(totals.net_total))?;
    w.text_element_with_attrs(
        "ram:TaxTotalAmount",
        &format_decimal(totals.vat_total),
        &[("currencyID", currency.as_str())],
    )?;
    w.text_element("ram:GrandTotalAmount", &format_decimal(totals.gross_total))?;
    w.text_element("ram:DuePayableAmount", &format_decimal(totals.amount_due))?;
    w.end_element("ram:SpecifiedTradeSettlementHeaderMonetarySummation")?;

    w.end_element("ram:ApplicableHeaderTradeSettlement")?;
    w.end_element("rsm:SupplyChainTradeTransaction")?;
    w.end_element("rsm:CrossIndustryInvoice")?;

    w.into_string()
}

fn write_cii_date(w: &mut XmlWriter, element: &str, date: &NaiveDate) -> Result<(), RechnungError> {
    w.start_element(element)?;
    w.text_element_with_attrs(
        "udt:DateTimeString",
        &date.format("%Y%m%d").to_string(),
        &[("format", "102")],
    )?;
    w.end_element(element)?;
    Ok(())
}

fn write_cii_party(w: &mut XmlWriter, party: &Party, element: &str) -> Result<(), RechnungError> {
    w.start_element(element)?;
    w.text_element("ram:Name", &party.name)?;

    if let Some(vat_id) = &party.vat_id {
        w.start_element("ram:SpecifiedTaxRegistration")?;
        w.text_element_with_attrs("ram:ID", vat_id, &[("schemeID", "VA")])?;
        w.end_element("ram:SpecifiedTaxRegistration")?;
    }

    w.start_element("ram:PostalTradeAddress")?;
    w.text_element("ram:PostcodeCode", &party.address.postal_code)?;
    if let Some(street) = &party.address.street {
        w.text_element("ram:LineOne", street)?;
    }
    w.text_element("ram:CityName", &party.address.city)?;
    w.text_element("ram:CountryID", &party.address.country_code)?;
    w.end_element("ram:PostalTradeAddress")?;

    if let Some(ea) = &party.electronic_address {
        w.start_element("ram:URIUniversalCommunication")?;
        w.text_element_with_attrs("ram:URIID", &ea.value, &[("schemeID", &ea.scheme)])?;
        w.end_element("ram:URIUniversalCommunication")?;
    }

    w.end_element(element)?;
    Ok(())
}
