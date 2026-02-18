use chrono::NaiveDate;
use quick_xml::Reader;
use quick_xml::events::Event;
use rust_decimal::Decimal;
use std::str::FromStr;

use super::xml_utils::{XmlResult, XmlWriter, format_decimal};
use super::{PEPPOL_PROFILE_ID, XRECHNUNG_CUSTOMIZATION_ID, cii_ns};
use crate::core::*;

/// Generate XRechnung-compliant CII (Cross Industry Invoice) XML.
pub fn to_cii_xml(invoice: &Invoice) -> XmlResult {
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

    // --- ExchangedDocumentContext ---
    w.start_element("rsm:ExchangedDocumentContext")?;
    w.start_element("ram:BusinessProcessSpecifiedDocumentContextParameter")?;
    w.text_element("ram:ID", PEPPOL_PROFILE_ID)?;
    w.end_element("ram:BusinessProcessSpecifiedDocumentContextParameter")?;
    w.start_element("ram:GuidelineSpecifiedDocumentContextParameter")?;
    w.text_element("ram:ID", XRECHNUNG_CUSTOMIZATION_ID)?;
    w.end_element("ram:GuidelineSpecifiedDocumentContextParameter")?;
    w.end_element("rsm:ExchangedDocumentContext")?;

    // --- ExchangedDocument ---
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

    // --- SupplyChainTradeTransaction ---
    w.start_element("rsm:SupplyChainTradeTransaction")?;

    // Lines
    for line in &invoice.lines {
        write_cii_line(&mut w, line, currency)?;
    }

    // --- ApplicableHeaderTradeAgreement ---
    w.start_element("ram:ApplicableHeaderTradeAgreement")?;
    if let Some(br) = &invoice.buyer_reference {
        w.text_element("ram:BuyerReference", br)?;
    }
    write_cii_party(&mut w, &invoice.seller, "ram:SellerTradeParty")?;
    write_cii_party(&mut w, &invoice.buyer, "ram:BuyerTradeParty")?;
    if let Some(or) = &invoice.order_reference {
        w.start_element("ram:BuyerOrderReferencedDocument")?;
        w.text_element("ram:IssuerAssignedID", or)?;
        w.end_element("ram:BuyerOrderReferencedDocument")?;
    }
    // BG-3: Preceding invoice references
    for pi in &invoice.preceding_invoices {
        w.start_element("ram:InvoiceReferencedDocument")?;
        w.text_element("ram:IssuerAssignedID", &pi.number)?;
        if let Some(d) = &pi.issue_date {
            write_cii_date(&mut w, "ram:FormattedIssueDateTime", d)?;
        }
        w.end_element("ram:InvoiceReferencedDocument")?;
    }
    // BG-24: Document attachments
    for att in &invoice.attachments {
        w.start_element("ram:AdditionalReferencedDocument")?;
        w.text_element("ram:IssuerAssignedID", att.id.as_deref().unwrap_or("n/a"))?;
        w.text_element("ram:TypeCode", "916")?;
        if let Some(desc) = &att.description {
            w.text_element("ram:Name", desc)?;
        }
        if let Some(emb) = &att.embedded_document {
            w.text_element_with_attrs(
                "ram:AttachmentBinaryObject",
                &emb.content,
                &[("mimeCode", &emb.mime_type), ("filename", &emb.filename)],
            )?;
        } else if let Some(uri) = &att.external_uri {
            w.text_element("ram:URIID", uri)?;
        }
        w.end_element("ram:AdditionalReferencedDocument")?;
    }
    w.end_element("ram:ApplicableHeaderTradeAgreement")?;

    // --- ApplicableHeaderTradeDelivery ---
    w.start_element("ram:ApplicableHeaderTradeDelivery")?;
    if let Some(tpd) = &invoice.tax_point_date {
        w.start_element("ram:ActualDeliverySupplyChainEvent")?;
        write_cii_date(&mut w, "ram:OccurrenceDateTime", tpd)?;
        w.end_element("ram:ActualDeliverySupplyChainEvent")?;
    }
    if let Some(period) = &invoice.invoicing_period {
        w.start_element("ram:BillingSpecifiedPeriod")?;
        write_cii_date(&mut w, "ram:StartDateTime", &period.start)?;
        write_cii_date(&mut w, "ram:EndDateTime", &period.end)?;
        w.end_element("ram:BillingSpecifiedPeriod")?;
    }
    w.end_element("ram:ApplicableHeaderTradeDelivery")?;

    // --- ApplicableHeaderTradeSettlement ---
    w.start_element("ram:ApplicableHeaderTradeSettlement")?;
    if let Some(tcc) = &invoice.tax_currency_code {
        w.text_element("ram:TaxCurrencyCode", tcc)?;
    }
    w.text_element("ram:InvoiceCurrencyCode", currency)?;

    // Payment means
    if let Some(payment) = &invoice.payment {
        w.start_element("ram:SpecifiedTradeSettlementPaymentMeans")?;
        w.text_element("ram:TypeCode", &payment.means_code.code().to_string())?;
        if let Some(ct) = &payment.credit_transfer {
            w.start_element("ram:PayeePartyCreditorFinancialAccount")?;
            w.text_element("ram:IBANID", &ct.iban)?;
            if let Some(name) = &ct.account_name {
                w.text_element("ram:AccountName", name)?;
            }
            w.end_element("ram:PayeePartyCreditorFinancialAccount")?;
            if let Some(bic) = &ct.bic {
                w.start_element("ram:PayeeSpecifiedCreditorFinancialInstitution")?;
                w.text_element("ram:BICID", bic)?;
                w.end_element("ram:PayeeSpecifiedCreditorFinancialInstitution")?;
            }
        }
        w.end_element("ram:SpecifiedTradeSettlementPaymentMeans")?;
    }

    // VAT breakdown
    for bd in &totals.vat_breakdown {
        w.start_element("ram:ApplicableTradeTax")?;
        w.text_element("ram:CalculatedAmount", &format_decimal(bd.tax_amount))?;
        w.text_element("ram:TypeCode", "VAT")?;
        if let Some(reason) = &bd.exemption_reason {
            w.text_element("ram:ExemptionReason", reason)?;
        }
        w.text_element("ram:BasisAmount", &format_decimal(bd.taxable_amount))?;
        w.text_element("ram:CategoryCode", bd.category.code())?;
        if let Some(code) = &bd.exemption_reason_code {
            w.text_element("ram:ExemptionReasonCode", code)?;
        }
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

    // Document-level allowances/charges
    for ac in invoice.allowances.iter().chain(invoice.charges.iter()) {
        write_cii_allowance_charge(&mut w, ac)?;
    }

    // Monetary summation
    w.start_element("ram:SpecifiedTradeSettlementHeaderMonetarySummation")?;
    w.text_element(
        "ram:LineTotalAmount",
        &format_decimal(totals.line_net_total),
    )?;
    if totals.charges_total > Decimal::ZERO {
        w.text_element(
            "ram:ChargeTotalAmount",
            &format_decimal(totals.charges_total),
        )?;
    }
    if totals.allowances_total > Decimal::ZERO {
        w.text_element(
            "ram:AllowanceTotalAmount",
            &format_decimal(totals.allowances_total),
        )?;
    }
    w.text_element("ram:TaxBasisTotalAmount", &format_decimal(totals.net_total))?;
    w.text_element_with_attrs(
        "ram:TaxTotalAmount",
        &format_decimal(totals.vat_total),
        &[("currencyID", currency.as_str())],
    )?;
    // BT-111: Tax total in tax currency
    if let (Some(tcc), Some(tax_total)) =
        (&invoice.tax_currency_code, totals.vat_total_in_tax_currency)
    {
        w.text_element_with_attrs(
            "ram:TaxTotalAmount",
            &format_decimal(tax_total),
            &[("currencyID", tcc.as_str())],
        )?;
    }
    w.text_element("ram:GrandTotalAmount", &format_decimal(totals.gross_total))?;
    if totals.prepaid > Decimal::ZERO {
        w.text_element("ram:TotalPrepaidAmount", &format_decimal(totals.prepaid))?;
    }
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
    // CII schema requires strict element order within TradeParty:
    // Name → SpecifiedLegalOrganization → DefinedTradeContact →
    // PostalTradeAddress → URIUniversalCommunication → SpecifiedTaxRegistration
    w.start_element(element)?;
    w.text_element("ram:Name", &party.name)?;

    // Legal organization
    if let Some(reg_id) = &party.registration_id {
        w.start_element("ram:SpecifiedLegalOrganization")?;
        w.text_element("ram:ID", reg_id)?;
        if let Some(tn) = &party.trading_name {
            w.text_element("ram:TradingBusinessName", tn)?;
        }
        w.end_element("ram:SpecifiedLegalOrganization")?;
    } else if let Some(tn) = &party.trading_name {
        w.start_element("ram:SpecifiedLegalOrganization")?;
        w.text_element("ram:TradingBusinessName", tn)?;
        w.end_element("ram:SpecifiedLegalOrganization")?;
    }

    // Contact
    if let Some(contact) = &party.contact {
        w.start_element("ram:DefinedTradeContact")?;
        if let Some(name) = &contact.name {
            w.text_element("ram:PersonName", name)?;
        }
        if let Some(phone) = &contact.phone {
            w.start_element("ram:TelephoneUniversalCommunication")?;
            w.text_element("ram:CompleteNumber", phone)?;
            w.end_element("ram:TelephoneUniversalCommunication")?;
        }
        if let Some(email) = &contact.email {
            w.start_element("ram:EmailURIUniversalCommunication")?;
            w.text_element("ram:URIID", email)?;
            w.end_element("ram:EmailURIUniversalCommunication")?;
        }
        w.end_element("ram:DefinedTradeContact")?;
    }

    // Postal address
    w.start_element("ram:PostalTradeAddress")?;
    w.text_element("ram:PostcodeCode", &party.address.postal_code)?;
    if let Some(street) = &party.address.street {
        w.text_element("ram:LineOne", street)?;
    }
    if let Some(additional) = &party.address.additional {
        w.text_element("ram:LineTwo", additional)?;
    }
    w.text_element("ram:CityName", &party.address.city)?;
    w.text_element("ram:CountryID", &party.address.country_code)?;
    if let Some(sub) = &party.address.subdivision {
        w.text_element("ram:CountrySubDivisionName", sub)?;
    }
    w.end_element("ram:PostalTradeAddress")?;

    // Electronic address
    if let Some(ea) = &party.electronic_address {
        w.start_element("ram:URIUniversalCommunication")?;
        w.text_element_with_attrs("ram:URIID", &ea.value, &[("schemeID", &ea.scheme)])?;
        w.end_element("ram:URIUniversalCommunication")?;
    }

    // Tax registrations (must come LAST per CII schema)
    if let Some(vat_id) = &party.vat_id {
        w.start_element("ram:SpecifiedTaxRegistration")?;
        w.text_element_with_attrs("ram:ID", vat_id, &[("schemeID", "VA")])?;
        w.end_element("ram:SpecifiedTaxRegistration")?;
    }
    if let Some(tax_num) = &party.tax_number {
        w.start_element("ram:SpecifiedTaxRegistration")?;
        w.text_element_with_attrs("ram:ID", tax_num, &[("schemeID", "FC")])?;
        w.end_element("ram:SpecifiedTaxRegistration")?;
    }

    w.end_element(element)?;
    Ok(())
}

fn write_cii_line(
    w: &mut XmlWriter,
    line: &LineItem,
    _currency: &str,
) -> Result<(), RechnungError> {
    w.start_element("ram:IncludedSupplyChainTradeLineItem")?;

    // Line document
    w.start_element("ram:AssociatedDocumentLineDocument")?;
    w.text_element("ram:LineID", &line.id)?;
    w.end_element("ram:AssociatedDocumentLineDocument")?;

    // Product
    w.start_element("ram:SpecifiedTradeProduct")?;
    if let Some(sid) = &line.seller_item_id {
        w.text_element("ram:SellerAssignedID", sid)?;
    }
    if let Some(std_id) = &line.standard_item_id {
        w.start_element("ram:GlobalID")?;
        w.text_element_with_attrs("ram:ID", std_id, &[("schemeID", "0160")])?;
        w.end_element("ram:GlobalID")?;
    }
    w.text_element("ram:Name", &line.item_name)?;
    if let Some(desc) = &line.description {
        w.text_element("ram:Description", desc)?;
    }
    // BT-160/BT-161: Item attributes
    for attr in &line.attributes {
        w.start_element("ram:ApplicableProductCharacteristic")?;
        w.text_element("ram:Description", &attr.name)?;
        w.text_element("ram:Value", &attr.value)?;
        w.end_element("ram:ApplicableProductCharacteristic")?;
    }
    w.end_element("ram:SpecifiedTradeProduct")?;

    // Trade agreement (price)
    w.start_element("ram:SpecifiedLineTradeAgreement")?;
    if let Some(gp) = line.gross_price {
        w.start_element("ram:GrossPriceProductTradePrice")?;
        w.text_element("ram:ChargeAmount", &format_decimal(gp))?;
        w.end_element("ram:GrossPriceProductTradePrice")?;
    }
    w.start_element("ram:NetPriceProductTradePrice")?;
    w.text_element("ram:ChargeAmount", &format_decimal(line.unit_price))?;
    w.end_element("ram:NetPriceProductTradePrice")?;
    w.end_element("ram:SpecifiedLineTradeAgreement")?;

    // Delivery (quantity)
    w.start_element("ram:SpecifiedLineTradeDelivery")?;
    w.text_element_with_attrs(
        "ram:BilledQuantity",
        &format_decimal(line.quantity),
        &[("unitCode", line.unit.as_str())],
    )?;
    w.end_element("ram:SpecifiedLineTradeDelivery")?;

    // Settlement (tax + line total)
    w.start_element("ram:SpecifiedLineTradeSettlement")?;
    // BG-26: Line invoicing period
    if let Some(period) = &line.invoicing_period {
        w.start_element("ram:BillingSpecifiedPeriod")?;
        write_cii_date(w, "ram:StartDateTime", &period.start)?;
        write_cii_date(w, "ram:EndDateTime", &period.end)?;
        w.end_element("ram:BillingSpecifiedPeriod")?;
    }
    w.start_element("ram:ApplicableTradeTax")?;
    w.text_element("ram:TypeCode", "VAT")?;
    w.text_element("ram:CategoryCode", line.tax_category.code())?;
    w.text_element("ram:RateApplicablePercent", &format_decimal(line.tax_rate))?;
    w.end_element("ram:ApplicableTradeTax")?;
    w.start_element("ram:SpecifiedTradeSettlementLineMonetarySummation")?;
    if let Some(amt) = line.line_amount {
        w.text_element("ram:LineTotalAmount", &format_decimal(amt))?;
    }
    w.end_element("ram:SpecifiedTradeSettlementLineMonetarySummation")?;
    w.end_element("ram:SpecifiedLineTradeSettlement")?;

    w.end_element("ram:IncludedSupplyChainTradeLineItem")?;
    Ok(())
}

fn write_cii_allowance_charge(
    w: &mut XmlWriter,
    ac: &AllowanceCharge,
) -> Result<(), RechnungError> {
    w.start_element("ram:SpecifiedTradeAllowanceCharge")?;
    w.text_element(
        "ram:ChargeIndicator",
        if ac.is_charge { "true" } else { "false" },
    )?;
    w.text_element("ram:ActualAmount", &format_decimal(ac.amount))?;
    if let Some(reason) = &ac.reason {
        w.text_element("ram:Reason", reason)?;
    }
    if let Some(code) = &ac.reason_code {
        w.text_element("ram:ReasonCode", code)?;
    }
    w.start_element("ram:CategoryTradeTax")?;
    w.text_element("ram:TypeCode", "VAT")?;
    w.text_element("ram:CategoryCode", ac.tax_category.code())?;
    w.text_element("ram:RateApplicablePercent", &format_decimal(ac.tax_rate))?;
    w.end_element("ram:CategoryTradeTax")?;
    w.end_element("ram:SpecifiedTradeAllowanceCharge")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

/// Parse a CII (Cross Industry Invoice) XML string into an Invoice struct.
pub fn from_cii_xml(xml: &str) -> Result<Invoice, RechnungError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut p = CiiParsed::default();
    let mut path: Vec<String> = Vec::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                let name = std::str::from_utf8(e.name().as_ref())
                    .unwrap_or("")
                    .to_string();

                // Capture attributes
                if name == "ram:URIID"
                    || name == "ram:ID"
                    || name == "ram:BilledQuantity"
                    || name == "udt:DateTimeString"
                    || name == "ram:TaxTotalAmount"
                    || name == "ram:AttachmentBinaryObject"
                {
                    for attr in e.attributes().flatten() {
                        let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                        let val = std::str::from_utf8(&attr.value).unwrap_or("");
                        match key {
                            "schemeID" => p.current_scheme_id = Some(val.to_string()),
                            "unitCode" => p.current_unit_code = Some(val.to_string()),
                            "currencyID" => p.current_currency_id = Some(val.to_string()),
                            "mimeCode" => {
                                if let Some(att) = p.current_attachment.as_mut() {
                                    att.mime_type = Some(val.to_string());
                                }
                            }
                            "filename" => {
                                if let Some(att) = p.current_attachment.as_mut() {
                                    att.filename = Some(val.to_string());
                                }
                            }
                            _ => {}
                        }
                    }
                }

                path.push(name);
            }
            Ok(Event::Text(ref e)) => {
                let text = e.unescape().unwrap_or_default().to_string();
                if !text.is_empty() {
                    p.handle_text(&path, &text);
                }
            }
            Ok(Event::End(_)) => {
                let ended = path.pop().unwrap_or_default();
                if ended == "ram:IncludedSupplyChainTradeLineItem" {
                    if let Some(line) = p.current_line.take() {
                        p.lines.push(line);
                    }
                }
                if ended == "ram:ApplicableTradeTax"
                    && !path
                        .iter()
                        .any(|p| p == "ram:IncludedSupplyChainTradeLineItem")
                    && !path
                        .iter()
                        .any(|p| p == "ram:SpecifiedTradeAllowanceCharge")
                {
                    if let Some(bd) = p.current_breakdown.take() {
                        p.vat_breakdown.push(bd);
                    }
                }
                if ended == "ram:InvoiceReferencedDocument" {
                    if let Some(pi) = p.current_preceding.take() {
                        p.preceding_invoices.push(pi);
                    }
                }
                if ended == "ram:AdditionalReferencedDocument" {
                    if let Some(att) = p.current_attachment.take() {
                        p.attachments.push(att);
                    }
                }
                // ram:ApplicableProductCharacteristic — attributes handled during text parsing
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(RechnungError::Builder(format!("XML parse error: {e}")));
            }
            _ => {}
        }
    }

    p.into_invoice()
}

#[derive(Default)]
struct CiiParsed {
    number: Option<String>,
    type_code: Option<String>,
    issue_date: Option<String>,
    currency_code: Option<String>,
    tax_currency_code: Option<String>,
    buyer_reference: Option<String>,
    order_reference: Option<String>,
    notes: Vec<String>,
    preceding_invoices: Vec<CiiPrecedingInvoice>,
    current_preceding: Option<CiiPrecedingInvoice>,
    attachments: Vec<CiiAttachment>,
    current_attachment: Option<CiiAttachment>,
    invoicing_period_start: Option<String>,
    invoicing_period_end: Option<String>,

    seller_name: Option<String>,
    seller_vat_id: Option<String>,
    seller_tax_number: Option<String>,
    seller_reg_id: Option<String>,
    seller_trading_name: Option<String>,
    seller_street: Option<String>,
    seller_additional: Option<String>,
    seller_city: Option<String>,
    seller_postal: Option<String>,
    seller_country: Option<String>,
    seller_contact_name: Option<String>,
    seller_contact_phone: Option<String>,
    seller_contact_email: Option<String>,
    seller_endpoint_scheme: Option<String>,
    seller_endpoint_value: Option<String>,

    buyer_name: Option<String>,
    buyer_vat_id: Option<String>,
    buyer_street: Option<String>,
    buyer_additional: Option<String>,
    buyer_city: Option<String>,
    buyer_postal: Option<String>,
    buyer_country: Option<String>,
    buyer_endpoint_scheme: Option<String>,
    buyer_endpoint_value: Option<String>,

    payment_means_code: Option<String>,
    payment_iban: Option<String>,
    payment_bic: Option<String>,
    payment_account_name: Option<String>,
    payment_terms: Option<String>,
    due_date: Option<String>,

    tax_total_in_tax_currency: Option<String>,

    line_total_amount: Option<String>,
    tax_basis_total: Option<String>,
    tax_total: Option<String>,
    grand_total: Option<String>,
    due_payable: Option<String>,
    prepaid_total: Option<String>,
    allowance_total: Option<String>,
    charge_total: Option<String>,

    vat_breakdown: Vec<CiiVatBreakdown>,
    current_breakdown: Option<CiiVatBreakdown>,

    lines: Vec<CiiLine>,
    current_line: Option<CiiLine>,

    // Temp state
    current_scheme_id: Option<String>,
    current_unit_code: Option<String>,
    current_currency_id: Option<String>,
    tax_point_date: Option<String>,
}

#[derive(Default, Clone)]
struct CiiVatBreakdown {
    calculated_amount: Option<String>,
    basis_amount: Option<String>,
    category_code: Option<String>,
    rate: Option<String>,
    exemption_reason: Option<String>,
    exemption_reason_code: Option<String>,
}

#[derive(Default, Clone)]
struct CiiLine {
    id: Option<String>,
    name: Option<String>,
    description: Option<String>,
    seller_item_id: Option<String>,
    quantity: Option<String>,
    unit: Option<String>,
    price: Option<String>,
    line_total: Option<String>,
    tax_category: Option<String>,
    tax_rate: Option<String>,
    attributes: Vec<(String, String)>,
    current_attr_name: Option<String>,
    invoicing_period_start: Option<String>,
    invoicing_period_end: Option<String>,
}

#[derive(Default, Clone)]
struct CiiPrecedingInvoice {
    number: Option<String>,
    issue_date: Option<String>,
}

#[derive(Default, Clone)]
struct CiiAttachment {
    id: Option<String>,
    description: Option<String>,
    content: Option<String>,
    mime_type: Option<String>,
    filename: Option<String>,
    external_uri: Option<String>,
}

impl CiiParsed {
    fn handle_text(&mut self, path: &[String], text: &str) {
        let leaf = path.last().map(|s| s.as_str()).unwrap_or("");
        let parent = if path.len() >= 2 {
            path[path.len() - 2].as_str()
        } else {
            ""
        };

        let in_seller = path.iter().any(|p| p == "ram:SellerTradeParty");
        let in_buyer = path.iter().any(|p| p == "ram:BuyerTradeParty");
        let in_line = path
            .iter()
            .any(|p| p == "ram:IncludedSupplyChainTradeLineItem");
        let in_settlement = path
            .iter()
            .any(|p| p == "ram:ApplicableHeaderTradeSettlement");
        let in_tax = path.iter().any(|p| p == "ram:ApplicableTradeTax") && !in_line;
        let in_monetary = path
            .iter()
            .any(|p| p == "ram:SpecifiedTradeSettlementHeaderMonetarySummation");

        // Document level
        if leaf == "ram:ID" && parent == "rsm:ExchangedDocument" {
            self.number = Some(text.to_string());
        }
        if leaf == "ram:TypeCode" && parent == "rsm:ExchangedDocument" {
            self.type_code = Some(text.to_string());
        }
        if leaf == "udt:DateTimeString" && parent == "ram:IssueDateTime" {
            self.issue_date = Some(text.to_string());
        }
        if leaf == "ram:Content" && parent == "ram:IncludedNote" {
            self.notes.push(text.to_string());
        }
        if leaf == "ram:BuyerReference" {
            self.buyer_reference = Some(text.to_string());
        }
        if leaf == "ram:IssuerAssignedID" && parent == "ram:BuyerOrderReferencedDocument" {
            self.order_reference = Some(text.to_string());
        }
        if leaf == "ram:InvoiceCurrencyCode" {
            self.currency_code = Some(text.to_string());
        }
        if leaf == "ram:TaxCurrencyCode" {
            self.tax_currency_code = Some(text.to_string());
        }

        // Delivery date
        if leaf == "udt:DateTimeString" && parent == "ram:OccurrenceDateTime" {
            self.tax_point_date = Some(text.to_string());
        }

        // BG-14: Document-level invoicing period (in delivery)
        let in_header_delivery = path
            .iter()
            .any(|p| p == "ram:ApplicableHeaderTradeDelivery");
        let in_billing_period = path.iter().any(|p| p == "ram:BillingSpecifiedPeriod");
        if in_header_delivery && in_billing_period && !in_line {
            if leaf == "udt:DateTimeString" && parent == "ram:StartDateTime" {
                self.invoicing_period_start = Some(text.to_string());
            }
            if leaf == "udt:DateTimeString" && parent == "ram:EndDateTime" {
                self.invoicing_period_end = Some(text.to_string());
            }
        }

        // BG-3: Preceding invoice references
        let in_invoice_ref = path.iter().any(|p| p == "ram:InvoiceReferencedDocument");
        if in_invoice_ref {
            let pi = self.current_preceding.get_or_insert_with(Default::default);
            if leaf == "ram:IssuerAssignedID" {
                pi.number = Some(text.to_string());
            }
            if leaf == "udt:DateTimeString" && parent == "ram:FormattedIssueDateTime" {
                pi.issue_date = Some(text.to_string());
            }
        }

        // BG-24: Document attachments
        let in_additional_ref = path.iter().any(|p| p == "ram:AdditionalReferencedDocument");
        if in_additional_ref && !in_line {
            let att = self.current_attachment.get_or_insert_with(Default::default);
            if leaf == "ram:IssuerAssignedID" && parent == "ram:AdditionalReferencedDocument" {
                att.id = Some(text.to_string());
            }
            if leaf == "ram:Name" && parent == "ram:AdditionalReferencedDocument" {
                att.description = Some(text.to_string());
            }
            if leaf == "ram:AttachmentBinaryObject" {
                att.content = Some(text.to_string());
            }
            if leaf == "ram:URIID" && parent == "ram:AdditionalReferencedDocument" {
                att.external_uri = Some(text.to_string());
            }
        }

        // Seller
        if in_seller && !in_line {
            match leaf {
                "ram:Name" if parent == "ram:SellerTradeParty" => {
                    self.seller_name = Some(text.to_string());
                }
                "ram:ID" if parent == "ram:SpecifiedTaxRegistration" => {
                    let scheme = self.current_scheme_id.take().unwrap_or_default();
                    if scheme == "VA" {
                        self.seller_vat_id = Some(text.to_string());
                    } else if scheme == "FC" {
                        self.seller_tax_number = Some(text.to_string());
                    }
                }
                "ram:ID" if parent == "ram:SpecifiedLegalOrganization" => {
                    self.seller_reg_id = Some(text.to_string());
                }
                "ram:TradingBusinessName" => self.seller_trading_name = Some(text.to_string()),
                "ram:PersonName" => self.seller_contact_name = Some(text.to_string()),
                "ram:CompleteNumber" => self.seller_contact_phone = Some(text.to_string()),
                "ram:URIID" if parent == "ram:EmailURIUniversalCommunication" => {
                    self.seller_contact_email = Some(text.to_string());
                }
                "ram:URIID" if parent == "ram:URIUniversalCommunication" => {
                    self.seller_endpoint_value = Some(text.to_string());
                    self.seller_endpoint_scheme = self.current_scheme_id.take();
                }
                "ram:LineOne" => self.seller_street = Some(text.to_string()),
                "ram:LineTwo" => self.seller_additional = Some(text.to_string()),
                "ram:CityName" => self.seller_city = Some(text.to_string()),
                "ram:PostcodeCode" => self.seller_postal = Some(text.to_string()),
                "ram:CountryID" => self.seller_country = Some(text.to_string()),
                _ => {}
            }
        }

        // Buyer
        if in_buyer && !in_line {
            match leaf {
                "ram:Name" if parent == "ram:BuyerTradeParty" => {
                    self.buyer_name = Some(text.to_string());
                }
                "ram:ID" if parent == "ram:SpecifiedTaxRegistration" => {
                    self.buyer_vat_id = Some(text.to_string());
                }
                "ram:URIID" if parent == "ram:URIUniversalCommunication" => {
                    self.buyer_endpoint_value = Some(text.to_string());
                    self.buyer_endpoint_scheme = self.current_scheme_id.take();
                }
                "ram:LineOne" => self.buyer_street = Some(text.to_string()),
                "ram:LineTwo" => self.buyer_additional = Some(text.to_string()),
                "ram:CityName" => self.buyer_city = Some(text.to_string()),
                "ram:PostcodeCode" => self.buyer_postal = Some(text.to_string()),
                "ram:CountryID" => self.buyer_country = Some(text.to_string()),
                _ => {}
            }
        }

        // Payment
        if in_settlement && !in_line {
            if leaf == "ram:TypeCode" && parent == "ram:SpecifiedTradeSettlementPaymentMeans" {
                self.payment_means_code = Some(text.to_string());
            }
            if leaf == "ram:IBANID" {
                self.payment_iban = Some(text.to_string());
            }
            if leaf == "ram:BICID" {
                self.payment_bic = Some(text.to_string());
            }
            if leaf == "ram:AccountName" && parent == "ram:PayeePartyCreditorFinancialAccount" {
                self.payment_account_name = Some(text.to_string());
            }
            if leaf == "ram:Description" && parent == "ram:SpecifiedTradePaymentTerms" {
                self.payment_terms = Some(text.to_string());
            }
            if leaf == "udt:DateTimeString" && parent == "ram:DueDateDateTime" {
                self.due_date = Some(text.to_string());
            }
        }

        // Tax breakdown
        if in_tax && in_settlement && !in_line {
            let bd = self.current_breakdown.get_or_insert_with(Default::default);
            match leaf {
                "ram:CalculatedAmount" => bd.calculated_amount = Some(text.to_string()),
                "ram:BasisAmount" => bd.basis_amount = Some(text.to_string()),
                "ram:CategoryCode" => bd.category_code = Some(text.to_string()),
                "ram:RateApplicablePercent" => bd.rate = Some(text.to_string()),
                "ram:ExemptionReason" => bd.exemption_reason = Some(text.to_string()),
                "ram:ExemptionReasonCode" => bd.exemption_reason_code = Some(text.to_string()),
                _ => {}
            }
        }

        // Monetary summation
        if in_monetary {
            match leaf {
                "ram:LineTotalAmount" => self.line_total_amount = Some(text.to_string()),
                "ram:TaxBasisTotalAmount" => self.tax_basis_total = Some(text.to_string()),
                "ram:TaxTotalAmount" => {
                    // Distinguish document currency vs tax currency
                    let cur_id = self.current_currency_id.take();
                    let is_tax_currency = cur_id.as_ref() != self.currency_code.as_ref()
                        && self.tax_currency_code.is_some()
                        && cur_id.as_ref() == self.tax_currency_code.as_ref();
                    if is_tax_currency {
                        self.tax_total_in_tax_currency = Some(text.to_string());
                    } else {
                        self.tax_total = Some(text.to_string());
                    }
                }
                "ram:GrandTotalAmount" => self.grand_total = Some(text.to_string()),
                "ram:DuePayableAmount" => self.due_payable = Some(text.to_string()),
                "ram:TotalPrepaidAmount" => self.prepaid_total = Some(text.to_string()),
                "ram:AllowanceTotalAmount" => self.allowance_total = Some(text.to_string()),
                "ram:ChargeTotalAmount" => self.charge_total = Some(text.to_string()),
                _ => {}
            }
        }

        // Line items
        if in_line {
            let line = self.current_line.get_or_insert_with(Default::default);

            let in_product_char = path
                .iter()
                .any(|p| p == "ram:ApplicableProductCharacteristic");
            let in_line_billing_period = path.iter().any(|p| p == "ram:BillingSpecifiedPeriod");

            if in_product_char {
                // BT-160/BT-161: Item attributes
                if leaf == "ram:Description" && parent == "ram:ApplicableProductCharacteristic" {
                    line.current_attr_name = Some(text.to_string());
                }
                if leaf == "ram:Value" && parent == "ram:ApplicableProductCharacteristic" {
                    let name = line.current_attr_name.take().unwrap_or_default();
                    line.attributes.push((name, text.to_string()));
                }
            } else if in_line_billing_period {
                // BG-26: Line invoicing period
                if leaf == "udt:DateTimeString" && parent == "ram:StartDateTime" {
                    line.invoicing_period_start = Some(text.to_string());
                }
                if leaf == "udt:DateTimeString" && parent == "ram:EndDateTime" {
                    line.invoicing_period_end = Some(text.to_string());
                }
            } else {
                match leaf {
                    "ram:LineID" => line.id = Some(text.to_string()),
                    "ram:Name" if parent == "ram:SpecifiedTradeProduct" => {
                        line.name = Some(text.to_string())
                    }
                    "ram:Description" if parent == "ram:SpecifiedTradeProduct" => {
                        line.description = Some(text.to_string())
                    }
                    "ram:SellerAssignedID" => line.seller_item_id = Some(text.to_string()),
                    "ram:BilledQuantity" => {
                        line.quantity = Some(text.to_string());
                        line.unit = self.current_unit_code.take();
                    }
                    "ram:ChargeAmount" if parent == "ram:NetPriceProductTradePrice" => {
                        line.price = Some(text.to_string());
                    }
                    "ram:LineTotalAmount" => line.line_total = Some(text.to_string()),
                    "ram:CategoryCode" if path.iter().any(|p| p == "ram:ApplicableTradeTax") => {
                        line.tax_category = Some(text.to_string());
                    }
                    "ram:RateApplicablePercent"
                        if path.iter().any(|p| p == "ram:ApplicableTradeTax") =>
                    {
                        line.tax_rate = Some(text.to_string());
                    }
                    _ => {}
                }
            }
        }
    }

    fn into_invoice(self) -> Result<Invoice, RechnungError> {
        let parse_decimal = |s: &str| -> Result<Decimal, RechnungError> {
            Decimal::from_str(s)
                .map_err(|e| RechnungError::Builder(format!("invalid decimal '{s}': {e}")))
        };

        let parse_cii_date = |s: &str| -> Result<NaiveDate, RechnungError> {
            NaiveDate::parse_from_str(s, "%Y%m%d")
                .map_err(|e| RechnungError::Builder(format!("invalid CII date '{s}': {e}")))
        };

        let issue_date = parse_cii_date(
            self.issue_date
                .as_deref()
                .ok_or_else(|| RechnungError::Builder("missing issue date".into()))?,
        )?;

        let type_code_num: u16 = self
            .type_code
            .as_deref()
            .unwrap_or("380")
            .parse()
            .map_err(|e| RechnungError::Builder(format!("invalid type code: {e}")))?;

        let seller = Party {
            name: self.seller_name.unwrap_or_default(),
            vat_id: self.seller_vat_id,
            tax_number: self.seller_tax_number,
            registration_id: self.seller_reg_id,
            trading_name: self.seller_trading_name,
            address: Address {
                street: self.seller_street,
                additional: self.seller_additional,
                city: self.seller_city.unwrap_or_default(),
                postal_code: self.seller_postal.unwrap_or_default(),
                country_code: self.seller_country.unwrap_or_default(),
                subdivision: None,
            },
            contact: if self.seller_contact_name.is_some()
                || self.seller_contact_phone.is_some()
                || self.seller_contact_email.is_some()
            {
                Some(Contact {
                    name: self.seller_contact_name,
                    phone: self.seller_contact_phone,
                    email: self.seller_contact_email,
                })
            } else {
                None
            },
            electronic_address: match (self.seller_endpoint_scheme, self.seller_endpoint_value) {
                (Some(s), Some(v)) => Some(ElectronicAddress {
                    scheme: s,
                    value: v,
                }),
                (None, Some(v)) => Some(ElectronicAddress {
                    scheme: "EM".to_string(),
                    value: v,
                }),
                _ => None,
            },
        };

        let buyer = Party {
            name: self.buyer_name.unwrap_or_default(),
            vat_id: self.buyer_vat_id,
            tax_number: None,
            registration_id: None,
            trading_name: None,
            address: Address {
                street: self.buyer_street,
                additional: self.buyer_additional,
                city: self.buyer_city.unwrap_or_default(),
                postal_code: self.buyer_postal.unwrap_or_default(),
                country_code: self.buyer_country.unwrap_or_default(),
                subdivision: None,
            },
            contact: None,
            electronic_address: match (self.buyer_endpoint_scheme, self.buyer_endpoint_value) {
                (Some(s), Some(v)) => Some(ElectronicAddress {
                    scheme: s,
                    value: v,
                }),
                (None, Some(v)) => Some(ElectronicAddress {
                    scheme: "EM".to_string(),
                    value: v,
                }),
                _ => None,
            },
        };

        let mut lines = Vec::new();
        for pl in self.lines {
            let attributes = pl
                .attributes
                .into_iter()
                .map(|(n, v)| ItemAttribute { name: n, value: v })
                .collect();
            let line_period = match (pl.invoicing_period_start, pl.invoicing_period_end) {
                (Some(s), Some(e)) => {
                    let start = parse_cii_date(&s).ok();
                    let end = parse_cii_date(&e).ok();
                    match (start, end) {
                        (Some(s), Some(e)) => Some(Period { start: s, end: e }),
                        _ => None,
                    }
                }
                _ => None,
            };

            lines.push(LineItem {
                id: pl.id.unwrap_or_default(),
                quantity: parse_decimal(pl.quantity.as_deref().unwrap_or("1"))?,
                unit: pl.unit.unwrap_or_else(|| "C62".to_string()),
                unit_price: parse_decimal(pl.price.as_deref().unwrap_or("0"))?,
                gross_price: None,
                allowances: Vec::new(),
                charges: Vec::new(),
                tax_category: TaxCategory::from_code(pl.tax_category.as_deref().unwrap_or("S"))
                    .unwrap_or(TaxCategory::StandardRate),
                tax_rate: parse_decimal(pl.tax_rate.as_deref().unwrap_or("0"))?,
                item_name: pl.name.unwrap_or_default(),
                description: pl.description,
                seller_item_id: pl.seller_item_id,
                standard_item_id: None,
                line_amount: pl.line_total.as_deref().and_then(|s| parse_decimal(s).ok()),
                attributes,
                invoicing_period: line_period,
            });
        }

        let mut vat_breakdown = Vec::new();
        for bd in self.vat_breakdown {
            vat_breakdown.push(VatBreakdown {
                category: TaxCategory::from_code(bd.category_code.as_deref().unwrap_or("S"))
                    .unwrap_or(TaxCategory::StandardRate),
                rate: parse_decimal(bd.rate.as_deref().unwrap_or("0"))?,
                taxable_amount: parse_decimal(bd.basis_amount.as_deref().unwrap_or("0"))?,
                tax_amount: parse_decimal(bd.calculated_amount.as_deref().unwrap_or("0"))?,
                exemption_reason: bd.exemption_reason,
                exemption_reason_code: bd.exemption_reason_code,
            });
        }

        let payment = if self.payment_means_code.is_some() || self.payment_iban.is_some() {
            let code: u16 = self
                .payment_means_code
                .as_deref()
                .unwrap_or("58")
                .parse()
                .unwrap_or(58);
            Some(PaymentInstructions {
                means_code: PaymentMeansCode::from_code(code),
                means_text: None,
                remittance_info: None,
                credit_transfer: if self.payment_iban.is_some() {
                    Some(CreditTransfer {
                        iban: self.payment_iban.unwrap_or_default(),
                        bic: self.payment_bic,
                        account_name: self.payment_account_name,
                    })
                } else {
                    None
                },
            })
        } else {
            None
        };

        let preceding_invoices = self
            .preceding_invoices
            .into_iter()
            .filter_map(|pi| {
                Some(PrecedingInvoiceReference {
                    number: pi.number?,
                    issue_date: pi
                        .issue_date
                        .as_deref()
                        .and_then(|d| parse_cii_date(d).ok()),
                })
            })
            .collect();

        let attachments = self
            .attachments
            .into_iter()
            .map(|a| DocumentAttachment {
                id: a.id,
                description: a.description,
                external_uri: a.external_uri,
                embedded_document: a.content.map(|c| EmbeddedDocument {
                    content: c,
                    mime_type: a.mime_type.unwrap_or_default(),
                    filename: a.filename.unwrap_or_default(),
                }),
            })
            .collect();

        let invoicing_period = match (self.invoicing_period_start, self.invoicing_period_end) {
            (Some(s), Some(e)) => {
                let start = parse_cii_date(&s).ok();
                let end = parse_cii_date(&e).ok();
                match (start, end) {
                    (Some(s), Some(e)) => Some(Period { start: s, end: e }),
                    _ => None,
                }
            }
            _ => None,
        };

        let vat_total_in_tax_currency = self
            .tax_total_in_tax_currency
            .as_deref()
            .and_then(|s| parse_decimal(s).ok());

        Ok(Invoice {
            number: self.number.unwrap_or_default(),
            issue_date,
            due_date: self
                .due_date
                .as_deref()
                .and_then(|d| parse_cii_date(d).ok()),
            type_code: InvoiceTypeCode::from_code(type_code_num)
                .unwrap_or(InvoiceTypeCode::Invoice),
            currency_code: self.currency_code.unwrap_or_else(|| "EUR".to_string()),
            tax_currency_code: self.tax_currency_code,
            notes: self.notes,
            buyer_reference: self.buyer_reference,
            order_reference: self.order_reference,
            seller,
            buyer,
            lines,
            vat_scenario: VatScenario::Domestic,
            allowances: Vec::new(),
            charges: Vec::new(),
            totals: Some(Totals {
                line_net_total: parse_decimal(self.line_total_amount.as_deref().unwrap_or("0"))?,
                allowances_total: parse_decimal(self.allowance_total.as_deref().unwrap_or("0"))?,
                charges_total: parse_decimal(self.charge_total.as_deref().unwrap_or("0"))?,
                net_total: parse_decimal(self.tax_basis_total.as_deref().unwrap_or("0"))?,
                vat_total: parse_decimal(self.tax_total.as_deref().unwrap_or("0"))?,
                vat_total_in_tax_currency,
                gross_total: parse_decimal(self.grand_total.as_deref().unwrap_or("0"))?,
                prepaid: parse_decimal(self.prepaid_total.as_deref().unwrap_or("0"))?,
                amount_due: parse_decimal(self.due_payable.as_deref().unwrap_or("0"))?,
                vat_breakdown,
            }),
            payment_terms: self.payment_terms,
            payment,
            tax_point_date: self
                .tax_point_date
                .as_deref()
                .and_then(|d| parse_cii_date(d).ok()),
            invoicing_period,
            preceding_invoices,
            attachments,
        })
    }
}
