use chrono::NaiveDate;
use quick_xml::Reader;
use quick_xml::events::Event;
use rust_decimal::Decimal;
use std::str::FromStr;

use super::xml_utils::{XmlResult, XmlWriter};
use super::{PEPPOL_PROFILE_ID, XRECHNUNG_CUSTOMIZATION_ID, ubl_ns};
use crate::core::*;

/// Generate XRechnung-compliant UBL 2.1 Invoice XML from an Invoice.
pub fn to_ubl_xml(invoice: &Invoice) -> XmlResult {
    let totals = invoice.totals.as_ref().ok_or_else(|| {
        RechnungError::Builder("totals must be calculated before XML generation".into())
    })?;

    let currency = &invoice.currency_code;
    let mut w = XmlWriter::new()?;

    // Root element with namespaces
    let is_credit_note = invoice.type_code == InvoiceTypeCode::CreditNote;
    let root_tag = if is_credit_note {
        "ubl:CreditNote"
    } else {
        "ubl:Invoice"
    };
    let root_ns = if is_credit_note {
        ubl_ns::CREDIT_NOTE
    } else {
        ubl_ns::INVOICE
    };

    w.start_element_with_attrs(
        root_tag,
        &[
            ("xmlns:ubl", root_ns),
            ("xmlns:cac", ubl_ns::CAC),
            ("xmlns:cbc", ubl_ns::CBC),
        ],
    )?;

    // BT-24: CustomizationID
    w.text_element("cbc:CustomizationID", XRECHNUNG_CUSTOMIZATION_ID)?;
    // BT-23: ProfileID
    w.text_element("cbc:ProfileID", PEPPOL_PROFILE_ID)?;
    // BT-1: Invoice number
    w.text_element("cbc:ID", &invoice.number)?;
    // BT-2: Issue date
    w.text_element("cbc:IssueDate", &invoice.issue_date.to_string())?;
    // BT-9: Due date
    if let Some(due) = &invoice.due_date {
        w.text_element("cbc:DueDate", &due.to_string())?;
    }
    // BT-3: Invoice type code
    w.text_element("cbc:InvoiceTypeCode", &invoice.type_code.code().to_string())?;
    // BT-22: Notes
    for note in &invoice.notes {
        w.text_element("cbc:Note", note)?;
    }
    // BT-7: Tax point date
    if let Some(tpd) = &invoice.tax_point_date {
        w.text_element("cbc:TaxPointDate", &tpd.to_string())?;
    }
    // BT-5: Currency code
    w.text_element("cbc:DocumentCurrencyCode", currency)?;
    // BT-6: Tax currency code
    if let Some(tcc) = &invoice.tax_currency_code {
        w.text_element("cbc:TaxCurrencyCode", tcc)?;
    }
    // BT-10: Buyer reference (Leitweg-ID)
    if let Some(br) = &invoice.buyer_reference {
        w.text_element("cbc:BuyerReference", br)?;
    }

    // BT-13: Order reference
    if let Some(or) = &invoice.order_reference {
        w.start_element("cac:OrderReference")?;
        w.text_element("cbc:ID", or)?;
        w.end_element("cac:OrderReference")?;
    }

    // BG-3: Billing reference (preceding invoices)
    for pi in &invoice.preceding_invoices {
        w.start_element("cac:BillingReference")?;
        w.start_element("cac:InvoiceDocumentReference")?;
        w.text_element("cbc:ID", &pi.number)?;
        if let Some(d) = &pi.issue_date {
            w.text_element("cbc:IssueDate", &d.to_string())?;
        }
        w.end_element("cac:InvoiceDocumentReference")?;
        w.end_element("cac:BillingReference")?;
    }

    // BG-24: Document attachments
    for att in &invoice.attachments {
        w.start_element("cac:AdditionalDocumentReference")?;
        w.text_element("cbc:ID", att.id.as_deref().unwrap_or("n/a"))?;
        if let Some(desc) = &att.description {
            w.text_element("cbc:DocumentDescription", desc)?;
        }
        if let Some(emb) = &att.embedded_document {
            w.start_element("cac:Attachment")?;
            w.text_element_with_attrs(
                "cbc:EmbeddedDocumentBinaryObject",
                &emb.content,
                &[("mimeCode", &emb.mime_type), ("filename", &emb.filename)],
            )?;
            w.end_element("cac:Attachment")?;
        } else if let Some(uri) = &att.external_uri {
            w.start_element("cac:Attachment")?;
            w.text_element("cbc:URI", uri)?;
            w.end_element("cac:Attachment")?;
        }
        w.end_element("cac:AdditionalDocumentReference")?;
    }

    // BG-14: Invoicing period
    if let Some(period) = &invoice.invoicing_period {
        w.start_element("cac:InvoicePeriod")?;
        w.text_element("cbc:StartDate", &period.start.to_string())?;
        w.text_element("cbc:EndDate", &period.end.to_string())?;
        w.end_element("cac:InvoicePeriod")?;
    }

    // BG-4: Seller
    write_ubl_party(&mut w, &invoice.seller, "cac:AccountingSupplierParty")?;
    // BG-7: Buyer
    write_ubl_party(&mut w, &invoice.buyer, "cac:AccountingCustomerParty")?;

    // BG-13: Delivery information
    if invoice.tax_point_date.is_some() || invoice.invoicing_period.is_some() {
        w.start_element("cac:Delivery")?;
        if let Some(tpd) = &invoice.tax_point_date {
            w.text_element("cbc:ActualDeliveryDate", &tpd.to_string())?;
        }
        w.end_element("cac:Delivery")?;
    }

    // BG-16: Payment means
    if let Some(payment) = &invoice.payment {
        w.start_element("cac:PaymentMeans")?;
        w.text_element(
            "cbc:PaymentMeansCode",
            &payment.means_code.code().to_string(),
        )?;
        if let Some(ri) = &payment.remittance_info {
            w.text_element("cbc:PaymentID", ri)?;
        }
        if let Some(ct) = &payment.credit_transfer {
            w.start_element("cac:PayeeFinancialAccount")?;
            w.text_element("cbc:ID", &ct.iban)?;
            if let Some(name) = &ct.account_name {
                w.text_element("cbc:Name", name)?;
            }
            if let Some(bic) = &ct.bic {
                w.start_element("cac:FinancialInstitutionBranch")?;
                w.text_element("cbc:ID", bic)?;
                w.end_element("cac:FinancialInstitutionBranch")?;
            }
            w.end_element("cac:PayeeFinancialAccount")?;
        }
        w.end_element("cac:PaymentMeans")?;
    }

    // BT-20: Payment terms
    if let Some(terms) = &invoice.payment_terms {
        w.start_element("cac:PaymentTerms")?;
        w.text_element("cbc:Note", terms)?;
        w.end_element("cac:PaymentTerms")?;
    }

    // BG-20: Document-level allowances
    for allowance in &invoice.allowances {
        write_ubl_allowance_charge(&mut w, allowance, currency)?;
    }

    // BG-21: Document-level charges
    for charge in &invoice.charges {
        write_ubl_allowance_charge(&mut w, charge, currency)?;
    }

    // BG-23: Tax total
    w.start_element("cac:TaxTotal")?;
    w.amount_element("cbc:TaxAmount", totals.vat_total, currency)?;

    for breakdown in &totals.vat_breakdown {
        w.start_element("cac:TaxSubtotal")?;
        w.amount_element("cbc:TaxableAmount", breakdown.taxable_amount, currency)?;
        w.amount_element("cbc:TaxAmount", breakdown.tax_amount, currency)?;
        w.start_element("cac:TaxCategory")?;
        w.text_element("cbc:ID", breakdown.category.code())?;
        w.text_element(
            "cbc:Percent",
            &super::xml_utils::format_decimal(breakdown.rate),
        )?;
        if let Some(reason) = &breakdown.exemption_reason {
            w.text_element("cbc:TaxExemptionReason", reason)?;
        }
        if let Some(code) = &breakdown.exemption_reason_code {
            w.text_element("cbc:TaxExemptionReasonCode", code)?;
        }
        w.start_element("cac:TaxScheme")?;
        w.text_element("cbc:ID", "VAT")?;
        w.end_element("cac:TaxScheme")?;
        w.end_element("cac:TaxCategory")?;
        w.end_element("cac:TaxSubtotal")?;
    }
    w.end_element("cac:TaxTotal")?;

    // BT-111: Tax total in tax currency
    if let (Some(tcc), Some(tax_total)) =
        (&invoice.tax_currency_code, totals.vat_total_in_tax_currency)
    {
        w.start_element("cac:TaxTotal")?;
        w.amount_element("cbc:TaxAmount", tax_total, tcc)?;
        w.end_element("cac:TaxTotal")?;
    }

    // BG-22: Legal monetary total
    w.start_element("cac:LegalMonetaryTotal")?;
    w.amount_element("cbc:LineExtensionAmount", totals.line_net_total, currency)?;
    w.amount_element("cbc:TaxExclusiveAmount", totals.net_total, currency)?;
    w.amount_element("cbc:TaxInclusiveAmount", totals.gross_total, currency)?;
    if totals.allowances_total > Decimal::ZERO {
        w.amount_element(
            "cbc:AllowanceTotalAmount",
            totals.allowances_total,
            currency,
        )?;
    }
    if totals.charges_total > Decimal::ZERO {
        w.amount_element("cbc:ChargeTotalAmount", totals.charges_total, currency)?;
    }
    if totals.prepaid > Decimal::ZERO {
        w.amount_element("cbc:PrepaidAmount", totals.prepaid, currency)?;
    }
    w.amount_element("cbc:PayableAmount", totals.amount_due, currency)?;
    w.end_element("cac:LegalMonetaryTotal")?;

    // BG-25: Invoice lines
    for line in &invoice.lines {
        write_ubl_line(&mut w, line, currency)?;
    }

    w.end_element(root_tag)?;

    w.into_string()
}

fn write_ubl_party(w: &mut XmlWriter, party: &Party, wrapper: &str) -> Result<(), RechnungError> {
    w.start_element(wrapper)?;
    w.start_element("cac:Party")?;

    // BT-34/49: Electronic address
    if let Some(ea) = &party.electronic_address {
        w.text_element_with_attrs("cbc:EndpointID", &ea.value, &[("schemeID", &ea.scheme)])?;
    }

    // BT-29/46: Trading name
    if let Some(tn) = &party.trading_name {
        w.start_element("cac:PartyName")?;
        w.text_element("cbc:Name", tn)?;
        w.end_element("cac:PartyName")?;
    }

    // BG-5/8: Postal address
    w.start_element("cac:PostalAddress")?;
    if let Some(street) = &party.address.street {
        w.text_element("cbc:StreetName", street)?;
    }
    if let Some(additional) = &party.address.additional {
        w.text_element("cbc:AdditionalStreetName", additional)?;
    }
    w.text_element("cbc:CityName", &party.address.city)?;
    w.text_element("cbc:PostalZone", &party.address.postal_code)?;
    if let Some(sub) = &party.address.subdivision {
        w.text_element("cbc:CountrySubentity", sub)?;
    }
    w.start_element("cac:Country")?;
    w.text_element("cbc:IdentificationCode", &party.address.country_code)?;
    w.end_element("cac:Country")?;
    w.end_element("cac:PostalAddress")?;

    // BT-31: VAT identifier
    if let Some(vat_id) = &party.vat_id {
        w.start_element("cac:PartyTaxScheme")?;
        w.text_element("cbc:CompanyID", vat_id)?;
        w.start_element("cac:TaxScheme")?;
        w.text_element("cbc:ID", "VAT")?;
        w.end_element("cac:TaxScheme")?;
        w.end_element("cac:PartyTaxScheme")?;
    }

    // BT-32: Tax number (Steuernummer) — uses FC scheme
    if let Some(tax_num) = &party.tax_number {
        w.start_element("cac:PartyTaxScheme")?;
        w.text_element("cbc:CompanyID", tax_num)?;
        w.start_element("cac:TaxScheme")?;
        w.text_element("cbc:ID", "FC")?;
        w.end_element("cac:TaxScheme")?;
        w.end_element("cac:PartyTaxScheme")?;
    }

    // BT-27/44: Legal entity
    w.start_element("cac:PartyLegalEntity")?;
    w.text_element("cbc:RegistrationName", &party.name)?;
    if let Some(reg_id) = &party.registration_id {
        w.text_element("cbc:CompanyID", reg_id)?;
    }
    w.end_element("cac:PartyLegalEntity")?;

    // BG-6/9: Contact
    if let Some(contact) = &party.contact {
        w.start_element("cac:Contact")?;
        if let Some(name) = &contact.name {
            w.text_element("cbc:Name", name)?;
        }
        if let Some(phone) = &contact.phone {
            w.text_element("cbc:Telephone", phone)?;
        }
        if let Some(email) = &contact.email {
            w.text_element("cbc:ElectronicMail", email)?;
        }
        w.end_element("cac:Contact")?;
    }

    w.end_element("cac:Party")?;
    w.end_element(wrapper)?;
    Ok(())
}

fn write_ubl_allowance_charge(
    w: &mut XmlWriter,
    ac: &AllowanceCharge,
    currency: &str,
) -> Result<(), RechnungError> {
    w.start_element("cac:AllowanceCharge")?;
    w.text_element(
        "cbc:ChargeIndicator",
        if ac.is_charge { "true" } else { "false" },
    )?;
    if let Some(code) = &ac.reason_code {
        w.text_element("cbc:AllowanceChargeReasonCode", code)?;
    }
    if let Some(reason) = &ac.reason {
        w.text_element("cbc:AllowanceChargeReason", reason)?;
    }
    w.amount_element("cbc:Amount", ac.amount, currency)?;
    if let Some(base) = &ac.base_amount {
        w.amount_element("cbc:BaseAmount", *base, currency)?;
    }
    w.start_element("cac:TaxCategory")?;
    w.text_element("cbc:ID", ac.tax_category.code())?;
    w.text_element(
        "cbc:Percent",
        &super::xml_utils::format_decimal(ac.tax_rate),
    )?;
    w.start_element("cac:TaxScheme")?;
    w.text_element("cbc:ID", "VAT")?;
    w.end_element("cac:TaxScheme")?;
    w.end_element("cac:TaxCategory")?;
    w.end_element("cac:AllowanceCharge")?;
    Ok(())
}

fn write_ubl_line(w: &mut XmlWriter, line: &LineItem, currency: &str) -> Result<(), RechnungError> {
    w.start_element("cac:InvoiceLine")?;
    // BT-126: Line ID
    w.text_element("cbc:ID", &line.id)?;
    // BT-129/130: Quantity with unit
    w.quantity_element("cbc:InvoicedQuantity", line.quantity, &line.unit)?;
    // BT-131: Line extension amount
    if let Some(amt) = line.line_amount {
        w.amount_element("cbc:LineExtensionAmount", amt, currency)?;
    }

    // BG-26: Line-level invoicing period
    if let Some(period) = &line.invoicing_period {
        w.start_element("cac:InvoicePeriod")?;
        w.text_element("cbc:StartDate", &period.start.to_string())?;
        w.text_element("cbc:EndDate", &period.end.to_string())?;
        w.end_element("cac:InvoicePeriod")?;
    }

    // BG-27: Line allowances
    for ac in &line.allowances {
        write_ubl_allowance_charge(w, ac, currency)?;
    }
    // BG-28: Line charges
    for ac in &line.charges {
        write_ubl_allowance_charge(w, ac, currency)?;
    }

    // Item
    w.start_element("cac:Item")?;
    if let Some(desc) = &line.description {
        w.text_element("cbc:Description", desc)?;
    }
    w.text_element("cbc:Name", &line.item_name)?;
    if let Some(sid) = &line.seller_item_id {
        w.start_element("cac:SellersItemIdentification")?;
        w.text_element("cbc:ID", sid)?;
        w.end_element("cac:SellersItemIdentification")?;
    }
    if let Some(std_id) = &line.standard_item_id {
        w.start_element("cac:StandardItemIdentification")?;
        w.text_element_with_attrs("cbc:ID", std_id, &[("schemeID", "0160")])?;
        w.end_element("cac:StandardItemIdentification")?;
    }
    // Line tax category
    w.start_element("cac:ClassifiedTaxCategory")?;
    w.text_element("cbc:ID", line.tax_category.code())?;
    w.text_element(
        "cbc:Percent",
        &super::xml_utils::format_decimal(line.tax_rate),
    )?;
    w.start_element("cac:TaxScheme")?;
    w.text_element("cbc:ID", "VAT")?;
    w.end_element("cac:TaxScheme")?;
    w.end_element("cac:ClassifiedTaxCategory")?;
    // BT-160/BT-161: Item attributes
    for attr in &line.attributes {
        w.start_element("cac:AdditionalItemProperty")?;
        w.text_element("cbc:Name", &attr.name)?;
        w.text_element("cbc:Value", &attr.value)?;
        w.end_element("cac:AdditionalItemProperty")?;
    }
    w.end_element("cac:Item")?;

    // Price
    w.start_element("cac:Price")?;
    w.amount_element("cbc:PriceAmount", line.unit_price, currency)?;
    if let Some(gp) = line.gross_price {
        w.amount_element("cbc:BaseQuantity", gp, currency)?;
    }
    w.end_element("cac:Price")?;

    w.end_element("cac:InvoiceLine")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

/// Parse a UBL Invoice XML string into an Invoice struct.
pub fn from_ubl_xml(xml: &str) -> Result<Invoice, RechnungError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut invoice = ParsedInvoice::default();
    let mut path: Vec<String> = Vec::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                let name = std::str::from_utf8(e.name().as_ref())
                    .unwrap_or("")
                    .to_string();

                // Capture attributes for elements that need them
                if name == "cbc:EndpointID"
                    || name == "cbc:InvoicedQuantity"
                    || name == "cbc:CreditedQuantity"
                    || name == "cbc:TaxAmount"
                    || name == "cbc:EmbeddedDocumentBinaryObject"
                {
                    for attr in e.attributes().flatten() {
                        let key = std::str::from_utf8(attr.key.as_ref()).unwrap_or("");
                        let val = std::str::from_utf8(&attr.value).unwrap_or("");
                        match key {
                            "schemeID" => {
                                invoice.current_scheme_id = Some(val.to_string());
                            }
                            "unitCode" => {
                                invoice.current_unit_code = Some(val.to_string());
                            }
                            "currencyID" => {
                                invoice.current_currency_id = Some(val.to_string());
                            }
                            "mimeCode" => {
                                if let Some(att) = invoice.current_attachment.as_mut() {
                                    att.mime_type = Some(val.to_string());
                                }
                            }
                            "filename" => {
                                if let Some(att) = invoice.current_attachment.as_mut() {
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
                    invoice.handle_ubl_text(&path, &text);
                }
            }
            Ok(Event::End(_)) => {
                let ended = path.pop().unwrap_or_default();
                // When we close a line item, push the current line
                if ended == "cac:InvoiceLine" || ended == "cac:CreditNoteLine" {
                    if let Some(line) = invoice.current_line.take() {
                        invoice.lines.push(line);
                    }
                }
                // When we close a TaxSubtotal, push the current breakdown
                if ended == "cac:TaxSubtotal" {
                    if let Some(bd) = invoice.current_breakdown.take() {
                        invoice.vat_breakdown.push(bd);
                    }
                }
                // When we close a BillingReference, push the current preceding invoice
                if ended == "cac:BillingReference" {
                    if let Some(pi) = invoice.current_preceding.take() {
                        invoice.preceding_invoices.push(pi);
                    }
                }
                // When we close an AdditionalDocumentReference, push the current attachment
                if ended == "cac:AdditionalDocumentReference" {
                    if let Some(att) = invoice.current_attachment.take() {
                        invoice.attachments.push(att);
                    }
                }
                // When we close an AdditionalItemProperty, push the attribute
                if ended == "cac:AdditionalItemProperty" {
                    if let Some(line) = invoice.current_line.as_mut() {
                        if let Some(name) = line.current_attr_name.take() {
                            // value was already set in handle_ubl_text
                            // find the last attribute with this name or push a placeholder
                            if let Some(last) = line.attributes.last_mut() {
                                if last.0 == name && last.1.is_empty() {
                                    // placeholder already there
                                }
                            }
                            // handled inline during text parsing
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(RechnungError::Builder(format!("XML parse error: {e}")));
            }
            _ => {}
        }
    }

    invoice.into_invoice()
}

// ---------------------------------------------------------------------------
// Parsing helpers
// ---------------------------------------------------------------------------

#[derive(Default)]
struct ParsedInvoice {
    number: Option<String>,
    issue_date: Option<String>,
    due_date: Option<String>,
    type_code: Option<String>,
    currency_code: Option<String>,
    tax_currency_code: Option<String>,
    buyer_reference: Option<String>,
    order_reference: Option<String>,
    notes: Vec<String>,
    tax_point_date: Option<String>,
    preceding_invoices: Vec<ParsedPrecedingInvoice>,
    current_preceding: Option<ParsedPrecedingInvoice>,
    attachments: Vec<ParsedAttachment>,
    current_attachment: Option<ParsedAttachment>,
    invoicing_period_start: Option<String>,
    invoicing_period_end: Option<String>,

    // Seller
    seller_name: Option<String>,
    seller_trading_name: Option<String>,
    seller_vat_id: Option<String>,
    seller_tax_number: Option<String>,
    seller_reg_id: Option<String>,
    seller_street: Option<String>,
    seller_additional: Option<String>,
    seller_city: Option<String>,
    seller_postal: Option<String>,
    seller_country: Option<String>,
    seller_subdivision: Option<String>,
    seller_contact_name: Option<String>,
    seller_contact_phone: Option<String>,
    seller_contact_email: Option<String>,
    seller_endpoint_scheme: Option<String>,
    seller_endpoint_value: Option<String>,

    // Buyer
    buyer_name: Option<String>,
    buyer_trading_name: Option<String>,
    buyer_vat_id: Option<String>,
    buyer_street: Option<String>,
    buyer_additional: Option<String>,
    buyer_city: Option<String>,
    buyer_postal: Option<String>,
    buyer_country: Option<String>,
    buyer_subdivision: Option<String>,
    buyer_endpoint_scheme: Option<String>,
    buyer_endpoint_value: Option<String>,

    // Payment
    payment_means_code: Option<String>,
    payment_iban: Option<String>,
    payment_bic: Option<String>,
    payment_account_name: Option<String>,
    payment_terms: Option<String>,

    // Totals
    line_extension_amount: Option<String>,
    tax_exclusive_amount: Option<String>,
    tax_inclusive_amount: Option<String>,
    payable_amount: Option<String>,
    prepaid_amount: Option<String>,
    allowance_total_amount: Option<String>,
    charge_total_amount: Option<String>,
    tax_amount: Option<String>,

    // VAT breakdown
    vat_breakdown: Vec<ParsedVatBreakdown>,
    current_breakdown: Option<ParsedVatBreakdown>,

    // Lines
    lines: Vec<ParsedLine>,
    current_line: Option<ParsedLine>,

    tax_total_in_tax_currency: Option<String>,

    // Temp parsing state
    current_scheme_id: Option<String>,
    current_unit_code: Option<String>,
    current_currency_id: Option<String>,
    in_seller_tax_scheme: bool,
}

#[derive(Default, Clone)]
struct ParsedVatBreakdown {
    taxable_amount: Option<String>,
    tax_amount: Option<String>,
    category_id: Option<String>,
    percent: Option<String>,
    exemption_reason: Option<String>,
    exemption_reason_code: Option<String>,
}

#[derive(Default, Clone)]
struct ParsedLine {
    id: Option<String>,
    quantity: Option<String>,
    unit: Option<String>,
    line_amount: Option<String>,
    item_name: Option<String>,
    description: Option<String>,
    seller_item_id: Option<String>,
    standard_item_id: Option<String>,
    unit_price: Option<String>,
    tax_category: Option<String>,
    tax_rate: Option<String>,
    attributes: Vec<(String, String)>,
    current_attr_name: Option<String>,
    invoicing_period_start: Option<String>,
    invoicing_period_end: Option<String>,
}

#[derive(Default, Clone)]
struct ParsedPrecedingInvoice {
    number: Option<String>,
    issue_date: Option<String>,
}

#[derive(Default, Clone)]
struct ParsedAttachment {
    id: Option<String>,
    description: Option<String>,
    content: Option<String>,
    mime_type: Option<String>,
    filename: Option<String>,
    external_uri: Option<String>,
}

/// Check if a parent element name is a UBL root (with or without `ubl:` prefix).
fn is_ubl_root(name: &str) -> bool {
    matches!(
        name,
        "ubl:Invoice" | "ubl:CreditNote" | "Invoice" | "CreditNote"
    )
}

impl ParsedInvoice {
    fn handle_ubl_text(&mut self, path: &[String], text: &str) {
        let leaf = path.last().map(|s| s.as_str()).unwrap_or("");
        let parent = if path.len() >= 2 {
            path[path.len() - 2].as_str()
        } else {
            ""
        };
        let grandparent = if path.len() >= 3 {
            path[path.len() - 3].as_str()
        } else {
            ""
        };
        let great_gp = if path.len() >= 4 {
            path[path.len() - 4].as_str()
        } else {
            ""
        };

        // Determine context
        let in_seller = path.iter().any(|p| p == "cac:AccountingSupplierParty");
        let in_buyer = path.iter().any(|p| p == "cac:AccountingCustomerParty");
        let in_line = path
            .iter()
            .any(|p| p == "cac:InvoiceLine" || p == "cac:CreditNoteLine");
        let in_tax_subtotal = path.iter().any(|p| p == "cac:TaxSubtotal");
        let in_tax_total = path.iter().any(|p| p == "cac:TaxTotal") && !in_line;

        let in_billing_ref = path.iter().any(|p| p == "cac:BillingReference");
        let in_additional_doc_ref =
            path.iter().any(|p| p == "cac:AdditionalDocumentReference") && !in_line;

        // Invoice-level fields
        if !in_seller
            && !in_buyer
            && !in_line
            && !in_tax_total
            && !in_billing_ref
            && !in_additional_doc_ref
        {
            match leaf {
                "cbc:ID" if is_ubl_root(parent) => {
                    self.number = Some(text.to_string());
                }
                "cbc:IssueDate" if is_ubl_root(parent) => self.issue_date = Some(text.to_string()),
                "cbc:DueDate" => self.due_date = Some(text.to_string()),
                "cbc:InvoiceTypeCode" | "cbc:CreditNoteTypeCode" => {
                    self.type_code = Some(text.to_string())
                }
                "cbc:DocumentCurrencyCode" => self.currency_code = Some(text.to_string()),
                "cbc:TaxCurrencyCode" => self.tax_currency_code = Some(text.to_string()),
                "cbc:BuyerReference" => self.buyer_reference = Some(text.to_string()),
                "cbc:Note" if is_ubl_root(parent) => {
                    self.notes.push(text.to_string());
                }
                "cbc:TaxPointDate" => self.tax_point_date = Some(text.to_string()),
                "cbc:ID" if parent == "cac:OrderReference" => {
                    self.order_reference = Some(text.to_string());
                }
                "cbc:StartDate" if parent == "cac:InvoicePeriod" => {
                    self.invoicing_period_start = Some(text.to_string());
                }
                "cbc:EndDate" if parent == "cac:InvoicePeriod" => {
                    self.invoicing_period_end = Some(text.to_string());
                }
                _ => {}
            }
        }

        // BG-3: Preceding invoice references
        if in_billing_ref {
            let pi = self.current_preceding.get_or_insert_with(Default::default);
            match leaf {
                "cbc:ID" if parent == "cac:InvoiceDocumentReference" => {
                    pi.number = Some(text.to_string());
                }
                "cbc:IssueDate" if parent == "cac:InvoiceDocumentReference" => {
                    pi.issue_date = Some(text.to_string());
                }
                _ => {}
            }
        }

        // BG-24: Document attachments
        if in_additional_doc_ref && !in_line {
            let att = self.current_attachment.get_or_insert_with(Default::default);
            match leaf {
                "cbc:ID" if parent == "cac:AdditionalDocumentReference" => {
                    att.id = Some(text.to_string());
                }
                "cbc:DocumentDescription" => {
                    att.description = Some(text.to_string());
                }
                "cbc:EmbeddedDocumentBinaryObject" => {
                    att.content = Some(text.to_string());
                }
                "cbc:URI" if parent == "cac:Attachment" => {
                    att.external_uri = Some(text.to_string());
                }
                _ => {}
            }
        }

        // Payment means
        if !in_line && parent == "cac:PaymentMeans" && leaf == "cbc:PaymentMeansCode" {
            self.payment_means_code = Some(text.to_string());
        }
        if !in_line && leaf == "cbc:ID" && parent == "cac:PayeeFinancialAccount" {
            self.payment_iban = Some(text.to_string());
        }
        if !in_line && leaf == "cbc:Name" && parent == "cac:PayeeFinancialAccount" {
            self.payment_account_name = Some(text.to_string());
        }
        if !in_line && leaf == "cbc:ID" && parent == "cac:FinancialInstitutionBranch" {
            self.payment_bic = Some(text.to_string());
        }
        if !in_line && leaf == "cbc:Note" && parent == "cac:PaymentTerms" {
            self.payment_terms = Some(text.to_string());
        }

        // Totals
        if parent == "cac:LegalMonetaryTotal" {
            match leaf {
                "cbc:LineExtensionAmount" => self.line_extension_amount = Some(text.to_string()),
                "cbc:TaxExclusiveAmount" => self.tax_exclusive_amount = Some(text.to_string()),
                "cbc:TaxInclusiveAmount" => self.tax_inclusive_amount = Some(text.to_string()),
                "cbc:PayableAmount" => self.payable_amount = Some(text.to_string()),
                "cbc:PrepaidAmount" => self.prepaid_amount = Some(text.to_string()),
                "cbc:AllowanceTotalAmount" => self.allowance_total_amount = Some(text.to_string()),
                "cbc:ChargeTotalAmount" => self.charge_total_amount = Some(text.to_string()),
                _ => {}
            }
        }

        // Tax total (not in line)
        if in_tax_total && !in_line {
            if leaf == "cbc:TaxAmount" && parent == "cac:TaxTotal" {
                // Check if this is the tax currency TaxTotal
                let is_tax_currency = self.current_currency_id.as_ref()
                    != Some(&self.currency_code.clone().unwrap_or_default())
                    && self.tax_currency_code.is_some()
                    && self.current_currency_id.as_ref() == self.tax_currency_code.as_ref();
                if is_tax_currency {
                    self.tax_total_in_tax_currency = Some(text.to_string());
                } else {
                    self.tax_amount = Some(text.to_string());
                }
                self.current_currency_id = None;
            }
            if in_tax_subtotal {
                let bd = self.current_breakdown.get_or_insert_with(Default::default);
                match leaf {
                    "cbc:TaxableAmount" if parent == "cac:TaxSubtotal" => {
                        bd.taxable_amount = Some(text.to_string())
                    }
                    "cbc:TaxAmount" if parent == "cac:TaxSubtotal" => {
                        bd.tax_amount = Some(text.to_string())
                    }
                    "cbc:ID" if parent == "cac:TaxCategory" => {
                        bd.category_id = Some(text.to_string())
                    }
                    "cbc:Percent" if parent == "cac:TaxCategory" => {
                        bd.percent = Some(text.to_string())
                    }
                    "cbc:TaxExemptionReason" => bd.exemption_reason = Some(text.to_string()),
                    "cbc:TaxExemptionReasonCode" => {
                        bd.exemption_reason_code = Some(text.to_string())
                    }
                    _ => {}
                }
            }
        }

        // Seller
        if in_seller && !in_line {
            match leaf {
                "cbc:EndpointID" => {
                    self.seller_endpoint_value = Some(text.to_string());
                    self.seller_endpoint_scheme = self.current_scheme_id.take();
                }
                "cbc:RegistrationName" if parent == "cac:PartyLegalEntity" => {
                    self.seller_name = Some(text.to_string());
                }
                "cbc:Name" if parent == "cac:PartyName" => {
                    self.seller_trading_name = Some(text.to_string());
                }
                "cbc:CompanyID" if parent == "cac:PartyLegalEntity" => {
                    self.seller_reg_id = Some(text.to_string());
                }
                "cbc:CompanyID" if parent == "cac:PartyTaxScheme" => {
                    // Determine VAT vs FC by looking at TaxScheme/ID
                    // We'll handle this in the TaxScheme/ID element
                    // For now store temporarily
                    if self.in_seller_tax_scheme {
                        // second tax scheme = tax number
                        self.seller_tax_number = Some(text.to_string());
                    } else {
                        self.seller_vat_id = Some(text.to_string());
                    }
                }
                "cbc:ID" if parent == "cac:TaxScheme" && grandparent == "cac:PartyTaxScheme" => {
                    // If the TaxScheme ID is FC, the CompanyID we just stored is actually the tax number
                    if text == "FC" {
                        // Move the last stored vat_id to tax_number if seller_tax_number is None
                        if self.seller_tax_number.is_none() {
                            self.seller_tax_number = self.seller_vat_id.take();
                        }
                        self.in_seller_tax_scheme = true;
                    }
                }
                "cbc:StreetName" if grandparent == "cac:Party" || great_gp == "cac:Party" => {
                    self.seller_street = Some(text.to_string());
                }
                "cbc:AdditionalStreetName" => self.seller_additional = Some(text.to_string()),
                "cbc:CityName" => self.seller_city = Some(text.to_string()),
                "cbc:PostalZone" => self.seller_postal = Some(text.to_string()),
                "cbc:IdentificationCode" if parent == "cac:Country" => {
                    self.seller_country = Some(text.to_string());
                }
                "cbc:CountrySubentity" => self.seller_subdivision = Some(text.to_string()),
                "cbc:Name" if parent == "cac:Contact" => {
                    self.seller_contact_name = Some(text.to_string());
                }
                "cbc:Telephone" => self.seller_contact_phone = Some(text.to_string()),
                "cbc:ElectronicMail" => self.seller_contact_email = Some(text.to_string()),
                _ => {}
            }
        }

        // Buyer
        if in_buyer && !in_line {
            match leaf {
                "cbc:EndpointID" => {
                    self.buyer_endpoint_value = Some(text.to_string());
                    self.buyer_endpoint_scheme = self.current_scheme_id.take();
                }
                "cbc:RegistrationName" if parent == "cac:PartyLegalEntity" => {
                    self.buyer_name = Some(text.to_string());
                }
                "cbc:Name" if parent == "cac:PartyName" => {
                    self.buyer_trading_name = Some(text.to_string());
                }
                "cbc:CompanyID" if parent == "cac:PartyTaxScheme" => {
                    self.buyer_vat_id = Some(text.to_string());
                }
                "cbc:StreetName" => self.buyer_street = Some(text.to_string()),
                "cbc:AdditionalStreetName" => self.buyer_additional = Some(text.to_string()),
                "cbc:CityName" => self.buyer_city = Some(text.to_string()),
                "cbc:PostalZone" => self.buyer_postal = Some(text.to_string()),
                "cbc:IdentificationCode" if parent == "cac:Country" => {
                    self.buyer_country = Some(text.to_string());
                }
                "cbc:CountrySubentity" => self.buyer_subdivision = Some(text.to_string()),
                _ => {}
            }
        }

        // Invoice lines
        if in_line {
            let line = self.current_line.get_or_insert_with(Default::default);
            match leaf {
                "cbc:ID" if parent == "cac:InvoiceLine" || parent == "cac:CreditNoteLine" => {
                    line.id = Some(text.to_string())
                }
                "cbc:InvoicedQuantity" | "cbc:CreditedQuantity" => {
                    line.quantity = Some(text.to_string());
                    line.unit = self.current_unit_code.take();
                }
                "cbc:LineExtensionAmount"
                    if parent == "cac:InvoiceLine" || parent == "cac:CreditNoteLine" =>
                {
                    line.line_amount = Some(text.to_string());
                }
                "cbc:Name" if parent == "cac:Item" => line.item_name = Some(text.to_string()),
                "cbc:Description" if parent == "cac:Item" => {
                    line.description = Some(text.to_string())
                }
                "cbc:ID" if parent == "cac:SellersItemIdentification" => {
                    line.seller_item_id = Some(text.to_string());
                }
                "cbc:ID" if parent == "cac:StandardItemIdentification" => {
                    line.standard_item_id = Some(text.to_string());
                }
                "cbc:PriceAmount" => line.unit_price = Some(text.to_string()),
                "cbc:ID" if parent == "cac:ClassifiedTaxCategory" => {
                    line.tax_category = Some(text.to_string());
                }
                "cbc:Percent" if parent == "cac:ClassifiedTaxCategory" => {
                    line.tax_rate = Some(text.to_string());
                }
                // BT-160/BT-161: Item attributes
                "cbc:Name" if parent == "cac:AdditionalItemProperty" => {
                    line.current_attr_name = Some(text.to_string());
                }
                "cbc:Value" if parent == "cac:AdditionalItemProperty" => {
                    let name = line.current_attr_name.take().unwrap_or_default();
                    line.attributes.push((name, text.to_string()));
                }
                // BG-26: Line invoicing period
                "cbc:StartDate" if parent == "cac:InvoicePeriod" => {
                    line.invoicing_period_start = Some(text.to_string());
                }
                "cbc:EndDate" if parent == "cac:InvoicePeriod" => {
                    line.invoicing_period_end = Some(text.to_string());
                }
                _ => {}
            }
        }
    }

    fn into_invoice(self) -> Result<Invoice, RechnungError> {
        let parse_date = |s: &str| -> Result<NaiveDate, RechnungError> {
            NaiveDate::parse_from_str(s, "%Y-%m-%d")
                .map_err(|e| RechnungError::Builder(format!("invalid date '{s}': {e}")))
        };

        let parse_decimal = |s: &str| -> Result<Decimal, RechnungError> {
            Decimal::from_str(s)
                .map_err(|e| RechnungError::Builder(format!("invalid decimal '{s}': {e}")))
        };

        let issue_date = parse_date(
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
                subdivision: self.seller_subdivision,
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
            trading_name: self.buyer_trading_name,
            address: Address {
                street: self.buyer_street,
                additional: self.buyer_additional,
                city: self.buyer_city.unwrap_or_default(),
                postal_code: self.buyer_postal.unwrap_or_default(),
                country_code: self.buyer_country.unwrap_or_default(),
                subdivision: self.buyer_subdivision,
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
            let qty = parse_decimal(pl.quantity.as_deref().unwrap_or("1"))?;
            let price = parse_decimal(pl.unit_price.as_deref().unwrap_or("0"))?;
            let tax_cat = TaxCategory::from_code(pl.tax_category.as_deref().unwrap_or("S"))
                .unwrap_or(TaxCategory::StandardRate);
            let tax_rate = parse_decimal(pl.tax_rate.as_deref().unwrap_or("0"))?;
            let line_amount = if let Some(la) = &pl.line_amount {
                Some(parse_decimal(la)?)
            } else {
                None
            };

            let attributes = pl
                .attributes
                .into_iter()
                .map(|(n, v)| ItemAttribute { name: n, value: v })
                .collect();
            let line_period = match (pl.invoicing_period_start, pl.invoicing_period_end) {
                (Some(s), Some(e)) => {
                    let start = parse_date(&s).ok();
                    let end = parse_date(&e).ok();
                    match (start, end) {
                        (Some(s), Some(e)) => Some(Period { start: s, end: e }),
                        _ => None,
                    }
                }
                _ => None,
            };

            lines.push(LineItem {
                id: pl.id.unwrap_or_default(),
                quantity: qty,
                unit: pl.unit.unwrap_or_else(|| "C62".to_string()),
                unit_price: price,
                gross_price: None,
                allowances: Vec::new(),
                charges: Vec::new(),
                tax_category: tax_cat,
                tax_rate,
                item_name: pl.item_name.unwrap_or_default(),
                description: pl.description,
                seller_item_id: pl.seller_item_id,
                standard_item_id: pl.standard_item_id,
                line_amount,
                attributes,
                invoicing_period: line_period,
            });
        }

        let mut vat_breakdown = Vec::new();
        for bd in self.vat_breakdown {
            let cat = TaxCategory::from_code(bd.category_id.as_deref().unwrap_or("S"))
                .unwrap_or(TaxCategory::StandardRate);
            vat_breakdown.push(VatBreakdown {
                category: cat,
                rate: parse_decimal(bd.percent.as_deref().unwrap_or("0"))?,
                taxable_amount: parse_decimal(bd.taxable_amount.as_deref().unwrap_or("0"))?,
                tax_amount: parse_decimal(bd.tax_amount.as_deref().unwrap_or("0"))?,
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

        let vat_total_in_tax_currency = self
            .tax_total_in_tax_currency
            .as_deref()
            .and_then(|s| parse_decimal(s).ok());

        let totals = Some(Totals {
            line_net_total: parse_decimal(self.line_extension_amount.as_deref().unwrap_or("0"))?,
            allowances_total: parse_decimal(self.allowance_total_amount.as_deref().unwrap_or("0"))?,
            charges_total: parse_decimal(self.charge_total_amount.as_deref().unwrap_or("0"))?,
            net_total: parse_decimal(self.tax_exclusive_amount.as_deref().unwrap_or("0"))?,
            vat_total: parse_decimal(self.tax_amount.as_deref().unwrap_or("0"))?,
            vat_total_in_tax_currency,
            gross_total: parse_decimal(self.tax_inclusive_amount.as_deref().unwrap_or("0"))?,
            prepaid: parse_decimal(self.prepaid_amount.as_deref().unwrap_or("0"))?,
            amount_due: parse_decimal(self.payable_amount.as_deref().unwrap_or("0"))?,
            vat_breakdown,
        });

        let preceding_invoices = self
            .preceding_invoices
            .into_iter()
            .filter_map(|pi| {
                Some(PrecedingInvoiceReference {
                    number: pi.number?,
                    issue_date: pi.issue_date.as_deref().and_then(|d| parse_date(d).ok()),
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
                let start = parse_date(&s).ok();
                let end = parse_date(&e).ok();
                match (start, end) {
                    (Some(s), Some(e)) => Some(Period { start: s, end: e }),
                    _ => None,
                }
            }
            _ => None,
        };

        Ok(Invoice {
            number: self.number.unwrap_or_default(),
            issue_date,
            due_date: self.due_date.as_deref().and_then(|d| parse_date(d).ok()),
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
            vat_scenario: VatScenario::Domestic, // Cannot be determined from XML alone
            allowances: Vec::new(),
            charges: Vec::new(),
            totals,
            payment_terms: self.payment_terms,
            payment,
            tax_point_date: self
                .tax_point_date
                .as_deref()
                .and_then(|d| parse_date(d).ok()),
            invoicing_period,
            preceding_invoices,
            attachments,
        })
    }
}
