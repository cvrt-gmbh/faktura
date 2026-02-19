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

    // BG-10: Payee party
    if let Some(payee) = &invoice.payee {
        w.start_element("cac:PayeeParty")?;
        w.start_element("cac:PartyName")?;
        w.text_element("cbc:Name", &payee.name)?;
        w.end_element("cac:PartyName")?;
        if let Some(id) = &payee.identifier {
            w.start_element("cac:PartyIdentification")?;
            w.text_element("cbc:ID", id)?;
            w.end_element("cac:PartyIdentification")?;
        }
        if let Some(reg_id) = &payee.legal_registration_id {
            w.start_element("cac:PartyLegalEntity")?;
            w.text_element("cbc:CompanyID", reg_id)?;
            w.end_element("cac:PartyLegalEntity")?;
        }
        w.end_element("cac:PayeeParty")?;
    }

    // BG-11: Seller tax representative party
    if let Some(tax_rep) = &invoice.tax_representative {
        w.start_element("cac:TaxRepresentativeParty")?;
        w.start_element("cac:PartyName")?;
        w.text_element("cbc:Name", &tax_rep.name)?;
        w.end_element("cac:PartyName")?;
        w.start_element("cac:PostalAddress")?;
        if let Some(street) = &tax_rep.address.street {
            w.text_element("cbc:StreetName", street)?;
        }
        if let Some(additional) = &tax_rep.address.additional {
            w.text_element("cbc:AdditionalStreetName", additional)?;
        }
        w.text_element("cbc:CityName", &tax_rep.address.city)?;
        w.text_element("cbc:PostalZone", &tax_rep.address.postal_code)?;
        if let Some(sub) = &tax_rep.address.subdivision {
            w.text_element("cbc:CountrySubentity", sub)?;
        }
        w.start_element("cac:Country")?;
        w.text_element("cbc:IdentificationCode", &tax_rep.address.country_code)?;
        w.end_element("cac:Country")?;
        w.end_element("cac:PostalAddress")?;
        w.start_element("cac:PartyTaxScheme")?;
        w.text_element("cbc:CompanyID", &tax_rep.vat_id)?;
        w.start_element("cac:TaxScheme")?;
        w.text_element("cbc:ID", "VAT")?;
        w.end_element("cac:TaxScheme")?;
        w.end_element("cac:PartyTaxScheme")?;
        w.end_element("cac:TaxRepresentativeParty")?;
    }

    // BG-13: Delivery information (BT-72 actual delivery date, BG-14/BG-15 party/address)
    if invoice.delivery.is_some()
        || invoice.tax_point_date.is_some()
        || invoice.invoicing_period.is_some()
    {
        w.start_element("cac:Delivery")?;

        // BT-72: Actual delivery date
        if let Some(delivery) = &invoice.delivery {
            if let Some(actual_delivery_date) = delivery.actual_delivery_date {
                w.text_element("cbc:ActualDeliveryDate", &actual_delivery_date.to_string())?;
            }

            // BG-15: Deliver-to party (BT-70 name, BT-71 location_id)
            if let Some(delivery_party) = &delivery.delivery_party {
                w.start_element("cac:DeliveryParty")?;
                w.start_element("cac:PartyName")?;
                w.text_element("cbc:Name", &delivery_party.name)?;
                w.end_element("cac:PartyName")?;
                if let Some(location_id) = &delivery_party.location_id {
                    w.start_element("cac:PartyIdentification")?;
                    w.text_element("cbc:ID", location_id)?;
                    w.end_element("cac:PartyIdentification")?;
                }
                w.end_element("cac:DeliveryParty")?;
            }

            // BG-15: Delivery address (BT-75-80)
            if let Some(delivery_address) = &delivery.delivery_address {
                w.start_element("cac:DeliveryLocation")?;
                w.start_element("cac:Address")?;

                if let Some(street) = &delivery_address.street {
                    w.text_element("cbc:StreetName", street)?;
                }
                if let Some(additional) = &delivery_address.additional {
                    w.text_element("cbc:AdditionalStreetName", additional)?;
                }

                w.text_element("cbc:CityName", &delivery_address.city)?;
                w.text_element("cbc:PostalZone", &delivery_address.postal_code)?;

                if let Some(subdivision) = &delivery_address.subdivision {
                    w.text_element("cbc:CountrySubentity", subdivision)?;
                }

                w.start_element("cac:Country")?;
                w.text_element("cbc:IdentificationCode", &delivery_address.country_code)?;
                w.end_element("cac:Country")?;

                w.end_element("cac:Address")?;
                w.end_element("cac:DeliveryLocation")?;
            }
        } else if let Some(tpd) = &invoice.tax_point_date {
            // Fallback for tax_point_date only (legacy behavior)
            w.text_element("cbc:ActualDeliveryDate", &tpd.to_string())?;
        }

        w.end_element("cac:Delivery")?;
    }

    // BG-16: Payment means
    if let Some(payment) = &invoice.payment {
        w.start_element("cac:PaymentMeans")?;
        // BT-81: Payment means code, BT-82: Payment means text
        if let Some(text) = &payment.means_text {
            w.text_element_with_attrs(
                "cbc:PaymentMeansCode",
                &payment.means_code.code().to_string(),
                &[("name", text.as_str())],
            )?;
        } else {
            w.text_element(
                "cbc:PaymentMeansCode",
                &payment.means_code.code().to_string(),
            )?;
        }
        // BT-83: Remittance information
        if let Some(ri) = &payment.remittance_info {
            w.text_element("cbc:PaymentID", ri)?;
        }
        // BG-18: Card payment
        if let Some(card) = &payment.card_payment {
            w.start_element("cac:CardAccount")?;
            w.text_element("cbc:PrimaryAccountNumberID", &card.account_number)?;
            if let Some(holder) = &card.holder_name {
                w.text_element("cbc:HolderName", holder)?;
            }
            w.end_element("cac:CardAccount")?;
        }
        // BG-19: Direct debit (PaymentMandate)
        if let Some(dd) = &payment.direct_debit {
            w.start_element("cac:PaymentMandate")?;
            if let Some(mandate_id) = &dd.mandate_id {
                w.text_element("cbc:ID", mandate_id)?;
            }
            if let Some(account_id) = &dd.debited_account_id {
                w.start_element("cac:PayerFinancialAccount")?;
                w.text_element("cbc:ID", account_id)?;
                w.end_element("cac:PayerFinancialAccount")?;
            }
            w.end_element("cac:PaymentMandate")?;
        }
        // BG-17: Credit transfer
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
    // BT-127: Line note
    if let Some(note) = &line.note {
        w.text_element("cbc:Note", note)?;
    }
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
    // BT-156: Buyer's item identifier
    if let Some(bid) = &line.buyer_item_id {
        w.start_element("cac:BuyersItemIdentification")?;
        w.text_element("cbc:ID", bid)?;
        w.end_element("cac:BuyersItemIdentification")?;
    }
    if let Some(std_id) = &line.standard_item_id {
        w.start_element("cac:StandardItemIdentification")?;
        w.text_element_with_attrs("cbc:ID", std_id, &[("schemeID", "0160")])?;
        w.end_element("cac:StandardItemIdentification")?;
    }
    // BT-159: Item country of origin
    if let Some(country) = &line.origin_country {
        w.start_element("cac:OriginCountry")?;
        w.text_element("cbc:IdentificationCode", country)?;
        w.end_element("cac:OriginCountry")?;
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

    // BG-29: Price details
    w.start_element("cac:Price")?;
    w.amount_element("cbc:PriceAmount", line.unit_price, currency)?;
    // BT-149/BT-150: Base quantity
    if let Some(bq) = line.base_quantity {
        let bq_unit = line.base_quantity_unit.as_deref().unwrap_or(&line.unit);
        w.quantity_element("cbc:BaseQuantity", bq, bq_unit)?;
    }
    if let Some(gp) = line.gross_price {
        // BT-147/BT-148: Price discount as AllowanceCharge inside Price
        let discount = gp - line.unit_price;
        if discount > Decimal::ZERO {
            w.start_element("cac:AllowanceCharge")?;
            w.text_element("cbc:ChargeIndicator", "false")?;
            w.amount_element("cbc:Amount", discount, currency)?;
            w.amount_element("cbc:BaseAmount", gp, currency)?;
            w.end_element("cac:AllowanceCharge")?;
        }
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
                    || name == "cbc:PaymentMeansCode"
                    || name == "cbc:BaseQuantity"
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
                            "name" if name == "cbc:PaymentMeansCode" => {
                                invoice.payment_means_text = Some(val.to_string());
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
                // When we close a line-level AllowanceCharge, push it
                if ended == "cac:AllowanceCharge" {
                    let in_line_ctx = path
                        .iter()
                        .any(|p| p == "cac:InvoiceLine" || p == "cac:CreditNoteLine");
                    let in_price_ctx = path.iter().any(|p| p == "cac:Price");
                    if in_line_ctx && !in_price_ctx {
                        if let Some(line) = invoice.current_line.as_mut() {
                            if let Some(ac) = line.current_ac.take() {
                                line.allowances_charges.push(ac);
                            }
                        }
                    } else if !in_line_ctx {
                        // Document-level allowance/charge
                        if let Some(ac) = invoice.current_doc_ac.take() {
                            invoice.doc_allowances_charges.push(ac);
                        }
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
    buyer_reg_id: Option<String>,
    buyer_street: Option<String>,
    buyer_additional: Option<String>,
    buyer_city: Option<String>,
    buyer_postal: Option<String>,
    buyer_country: Option<String>,
    buyer_subdivision: Option<String>,
    buyer_contact_name: Option<String>,
    buyer_contact_phone: Option<String>,
    buyer_contact_email: Option<String>,
    buyer_endpoint_scheme: Option<String>,
    buyer_endpoint_value: Option<String>,

    // Payee (BG-10)
    payee_name: Option<String>,
    payee_identifier: Option<String>,
    payee_legal_reg_id: Option<String>,

    // Tax representative (BG-11)
    tax_rep_name: Option<String>,
    tax_rep_vat_id: Option<String>,
    tax_rep_street: Option<String>,
    tax_rep_additional: Option<String>,
    tax_rep_city: Option<String>,
    tax_rep_postal: Option<String>,
    tax_rep_country: Option<String>,
    tax_rep_subdivision: Option<String>,

    // Payment
    payment_means_code: Option<String>,
    payment_means_text: Option<String>,
    payment_remittance_info: Option<String>,
    // BG-18: Card payment
    card_account_number: Option<String>,
    card_holder_name: Option<String>,
    // BG-19: Direct debit
    direct_debit_mandate_id: Option<String>,
    direct_debit_account_id: Option<String>,
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

    // Document-level allowances/charges
    doc_allowances_charges: Vec<ParsedAllowanceCharge>,
    current_doc_ac: Option<ParsedAllowanceCharge>,

    // Delivery information (BG-13/BG-14/BG-15)
    delivery_actual_date: Option<String>,
    delivery_party_name: Option<String>,
    delivery_party_location_id: Option<String>,
    delivery_street: Option<String>,
    delivery_additional: Option<String>,
    delivery_city: Option<String>,
    delivery_postal: Option<String>,
    delivery_country: Option<String>,
    delivery_subdivision: Option<String>,

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
    note: Option<String>,
    quantity: Option<String>,
    unit: Option<String>,
    line_amount: Option<String>,
    item_name: Option<String>,
    description: Option<String>,
    seller_item_id: Option<String>,
    buyer_item_id: Option<String>,
    standard_item_id: Option<String>,
    unit_price: Option<String>,
    gross_price: Option<String>,
    base_quantity: Option<String>,
    base_quantity_unit: Option<String>,
    tax_category: Option<String>,
    tax_rate: Option<String>,
    origin_country: Option<String>,
    attributes: Vec<(String, String)>,
    current_attr_name: Option<String>,
    invoicing_period_start: Option<String>,
    invoicing_period_end: Option<String>,
    allowances_charges: Vec<ParsedAllowanceCharge>,
    current_ac: Option<ParsedAllowanceCharge>,
}

#[derive(Default, Clone)]
struct ParsedAllowanceCharge {
    is_charge: Option<String>,
    amount: Option<String>,
    base_amount: Option<String>,
    reason: Option<String>,
    reason_code: Option<String>,
    tax_category: Option<String>,
    tax_rate: Option<String>,
    percentage: Option<String>,
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
        let in_delivery = path.iter().any(|p| p == "cac:Delivery");

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
        if !in_line && parent == "cac:PaymentMeans" && leaf == "cbc:PaymentID" {
            self.payment_remittance_info = Some(text.to_string());
        }
        // BG-18: Card payment
        if !in_line && leaf == "cbc:PrimaryAccountNumberID" && parent == "cac:CardAccount" {
            self.card_account_number = Some(text.to_string());
        }
        if !in_line && leaf == "cbc:HolderName" && parent == "cac:CardAccount" {
            self.card_holder_name = Some(text.to_string());
        }
        // BG-19: Direct debit
        if !in_line && leaf == "cbc:ID" && parent == "cac:PaymentMandate" {
            self.direct_debit_mandate_id = Some(text.to_string());
        }
        if !in_line
            && leaf == "cbc:ID"
            && parent == "cac:PayerFinancialAccount"
            && path.iter().any(|p| p == "cac:PaymentMandate")
        {
            self.direct_debit_account_id = Some(text.to_string());
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

        // BG-13/BG-14/BG-15: Delivery information
        if in_delivery && !in_line {
            match leaf {
                "cbc:ActualDeliveryDate" => self.delivery_actual_date = Some(text.to_string()),
                "cbc:Name" if parent == "cac:PartyName" && grandparent == "cac:DeliveryParty" => {
                    self.delivery_party_name = Some(text.to_string());
                }
                "cbc:ID"
                    if parent == "cac:PartyIdentification"
                        && grandparent == "cac:DeliveryParty" =>
                {
                    self.delivery_party_location_id = Some(text.to_string());
                }
                "cbc:StreetName" if parent == "cac:Address" => {
                    self.delivery_street = Some(text.to_string());
                }
                "cbc:AdditionalStreetName" if parent == "cac:Address" => {
                    self.delivery_additional = Some(text.to_string());
                }
                "cbc:CityName" if parent == "cac:Address" => {
                    self.delivery_city = Some(text.to_string());
                }
                "cbc:PostalZone" if parent == "cac:Address" => {
                    self.delivery_postal = Some(text.to_string());
                }
                "cbc:CountrySubentity" if parent == "cac:Address" => {
                    self.delivery_subdivision = Some(text.to_string());
                }
                "cbc:IdentificationCode"
                    if parent == "cac:Country" && grandparent == "cac:Address" =>
                {
                    self.delivery_country = Some(text.to_string());
                }
                _ => {}
            }
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
                "cbc:CompanyID" if parent == "cac:PartyLegalEntity" => {
                    self.buyer_reg_id = Some(text.to_string());
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
                "cbc:Name" if parent == "cac:Contact" => {
                    self.buyer_contact_name = Some(text.to_string());
                }
                "cbc:Telephone" => self.buyer_contact_phone = Some(text.to_string()),
                "cbc:ElectronicMail" => self.buyer_contact_email = Some(text.to_string()),
                _ => {}
            }
        }

        // BG-10: Payee party
        let in_payee = path.iter().any(|p| p == "cac:PayeeParty");
        if in_payee && !in_seller && !in_buyer && !in_line {
            match leaf {
                "cbc:Name" if parent == "cac:PartyName" => {
                    self.payee_name = Some(text.to_string());
                }
                "cbc:ID" if parent == "cac:PartyIdentification" => {
                    self.payee_identifier = Some(text.to_string());
                }
                "cbc:CompanyID" if parent == "cac:PartyLegalEntity" => {
                    self.payee_legal_reg_id = Some(text.to_string());
                }
                _ => {}
            }
        }

        // BG-11: Tax representative party
        let in_tax_rep = path.iter().any(|p| p == "cac:TaxRepresentativeParty");
        if in_tax_rep && !in_seller && !in_buyer && !in_line {
            match leaf {
                "cbc:Name" if parent == "cac:PartyName" => {
                    self.tax_rep_name = Some(text.to_string());
                }
                "cbc:CompanyID" if parent == "cac:PartyTaxScheme" => {
                    self.tax_rep_vat_id = Some(text.to_string());
                }
                "cbc:StreetName" => self.tax_rep_street = Some(text.to_string()),
                "cbc:AdditionalStreetName" => self.tax_rep_additional = Some(text.to_string()),
                "cbc:CityName" => self.tax_rep_city = Some(text.to_string()),
                "cbc:PostalZone" => self.tax_rep_postal = Some(text.to_string()),
                "cbc:IdentificationCode" if parent == "cac:Country" => {
                    self.tax_rep_country = Some(text.to_string());
                }
                "cbc:CountrySubentity" => self.tax_rep_subdivision = Some(text.to_string()),
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
                "cbc:Note"
                    if parent == "cac:InvoiceLine" || parent == "cac:CreditNoteLine" =>
                {
                    line.note = Some(text.to_string());
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
                "cbc:ID" if parent == "cac:BuyersItemIdentification" => {
                    line.buyer_item_id = Some(text.to_string());
                }
                "cbc:ID" if parent == "cac:StandardItemIdentification" => {
                    line.standard_item_id = Some(text.to_string());
                }
                "cbc:IdentificationCode" if parent == "cac:OriginCountry" => {
                    line.origin_country = Some(text.to_string());
                }
                "cbc:PriceAmount" => line.unit_price = Some(text.to_string()),
                "cbc:BaseQuantity" if path.iter().any(|p| p == "cac:Price") => {
                    line.base_quantity = Some(text.to_string());
                    line.base_quantity_unit = self.current_unit_code.take();
                }
                // BG-29: gross price from Price/AllowanceCharge/BaseAmount
                "cbc:BaseAmount"
                    if path.iter().any(|p| p == "cac:Price") && parent == "cac:AllowanceCharge" =>
                {
                    line.gross_price = Some(text.to_string());
                }
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

            // BG-27/BG-28: Line-level AllowanceCharge (not inside Price)
            let in_price = path.iter().any(|p| p == "cac:Price");
            if path.iter().any(|p| p == "cac:AllowanceCharge") && !in_price {
                let ac = line.current_ac.get_or_insert_with(Default::default);
                match leaf {
                    "cbc:ChargeIndicator" => ac.is_charge = Some(text.to_string()),
                    "cbc:Amount" if parent == "cac:AllowanceCharge" => {
                        ac.amount = Some(text.to_string())
                    }
                    "cbc:BaseAmount" => ac.base_amount = Some(text.to_string()),
                    "cbc:AllowanceChargeReason" => ac.reason = Some(text.to_string()),
                    "cbc:AllowanceChargeReasonCode" => ac.reason_code = Some(text.to_string()),
                    "cbc:MultiplierFactorNumeric" => ac.percentage = Some(text.to_string()),
                    "cbc:ID" if parent == "cac:TaxCategory" => {
                        ac.tax_category = Some(text.to_string())
                    }
                    "cbc:Percent" if parent == "cac:TaxCategory" => {
                        ac.tax_rate = Some(text.to_string())
                    }
                    _ => {}
                }
            }
        }

        // BG-20/BG-21: Document-level AllowanceCharge
        if !in_seller
            && !in_buyer
            && !in_line
            && !in_billing_ref
            && !in_additional_doc_ref
            && path.iter().any(|p| p == "cac:AllowanceCharge")
        {
            let ac = self.current_doc_ac.get_or_insert_with(Default::default);
            match leaf {
                "cbc:ChargeIndicator" => ac.is_charge = Some(text.to_string()),
                "cbc:Amount" if parent == "cac:AllowanceCharge" => {
                    ac.amount = Some(text.to_string())
                }
                "cbc:BaseAmount" => ac.base_amount = Some(text.to_string()),
                "cbc:AllowanceChargeReason" => ac.reason = Some(text.to_string()),
                "cbc:AllowanceChargeReasonCode" => ac.reason_code = Some(text.to_string()),
                "cbc:MultiplierFactorNumeric" => ac.percentage = Some(text.to_string()),
                "cbc:ID" if parent == "cac:TaxCategory" => ac.tax_category = Some(text.to_string()),
                "cbc:Percent" if parent == "cac:TaxCategory" => {
                    ac.tax_rate = Some(text.to_string())
                }
                _ => {}
            }
        }
    }

    fn convert_allowance_charges(
        parsed: Vec<ParsedAllowanceCharge>,
        parse_decimal: &dyn Fn(&str) -> Result<Decimal, RechnungError>,
    ) -> Result<(Vec<AllowanceCharge>, Vec<AllowanceCharge>), RechnungError> {
        let mut allowances = Vec::new();
        let mut charges = Vec::new();
        for pac in parsed {
            let is_charge = pac.is_charge.as_deref() == Some("true");
            let amount = parse_decimal(pac.amount.as_deref().unwrap_or("0"))?;
            let ac = AllowanceCharge {
                is_charge,
                amount,
                percentage: pac
                    .percentage
                    .as_deref()
                    .and_then(|s| parse_decimal(s).ok()),
                base_amount: pac
                    .base_amount
                    .as_deref()
                    .and_then(|s| parse_decimal(s).ok()),
                tax_category: TaxCategory::from_code(pac.tax_category.as_deref().unwrap_or("S"))
                    .unwrap_or(TaxCategory::StandardRate),
                tax_rate: parse_decimal(pac.tax_rate.as_deref().unwrap_or("0"))?,
                reason: pac.reason,
                reason_code: pac.reason_code,
            };
            if is_charge {
                charges.push(ac);
            } else {
                allowances.push(ac);
            }
        }
        Ok((allowances, charges))
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
            registration_id: self.buyer_reg_id,
            trading_name: self.buyer_trading_name,
            address: Address {
                street: self.buyer_street,
                additional: self.buyer_additional,
                city: self.buyer_city.unwrap_or_default(),
                postal_code: self.buyer_postal.unwrap_or_default(),
                country_code: self.buyer_country.unwrap_or_default(),
                subdivision: self.buyer_subdivision,
            },
            contact: if self.buyer_contact_name.is_some()
                || self.buyer_contact_phone.is_some()
                || self.buyer_contact_email.is_some()
            {
                Some(Contact {
                    name: self.buyer_contact_name,
                    phone: self.buyer_contact_phone,
                    email: self.buyer_contact_email,
                })
            } else {
                None
            },
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

            let gross_price = pl
                .gross_price
                .as_deref()
                .and_then(|s| parse_decimal(s).ok());
            let (line_allowances, line_charges) =
                Self::convert_allowance_charges(pl.allowances_charges, &parse_decimal)?;

            let base_quantity = pl
                .base_quantity
                .as_deref()
                .and_then(|s| parse_decimal(s).ok());

            lines.push(LineItem {
                id: pl.id.unwrap_or_default(),
                quantity: qty,
                unit: pl.unit.unwrap_or_else(|| "C62".to_string()),
                unit_price: price,
                gross_price,
                allowances: line_allowances,
                charges: line_charges,
                tax_category: tax_cat,
                tax_rate,
                item_name: pl.item_name.unwrap_or_default(),
                description: pl.description,
                seller_item_id: pl.seller_item_id,
                buyer_item_id: pl.buyer_item_id,
                standard_item_id: pl.standard_item_id,
                line_amount,
                note: pl.note,
                base_quantity,
                base_quantity_unit: pl.base_quantity_unit,
                origin_country: pl.origin_country,
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

        let payment = if self.payment_means_code.is_some()
            || self.payment_iban.is_some()
            || self.card_account_number.is_some()
            || self.direct_debit_mandate_id.is_some()
        {
            let code: u16 = self
                .payment_means_code
                .as_deref()
                .unwrap_or("58")
                .parse()
                .unwrap_or(58);
            Some(PaymentInstructions {
                means_code: PaymentMeansCode::from_code(code),
                means_text: self.payment_means_text,
                remittance_info: self.payment_remittance_info,
                credit_transfer: if self.payment_iban.is_some() {
                    Some(CreditTransfer {
                        iban: self.payment_iban.unwrap_or_default(),
                        bic: self.payment_bic,
                        account_name: self.payment_account_name,
                    })
                } else {
                    None
                },
                card_payment: self.card_account_number.map(|num| CardPayment {
                    account_number: num,
                    holder_name: self.card_holder_name,
                }),
                direct_debit: if self.direct_debit_mandate_id.is_some()
                    || self.direct_debit_account_id.is_some()
                {
                    Some(DirectDebit {
                        mandate_id: self.direct_debit_mandate_id,
                        creditor_id: None, // UBL doesn't carry creditor ID in PaymentMandate
                        debited_account_id: self.direct_debit_account_id,
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

        let (doc_allowances, doc_charges) =
            Self::convert_allowance_charges(self.doc_allowances_charges, &parse_decimal)?;

        // Build delivery information from parsed fields (BG-13/BG-14/BG-15)
        let delivery = {
            let actual_delivery_date = self
                .delivery_actual_date
                .as_deref()
                .and_then(|d| parse_date(d).ok());

            let delivery_party = self.delivery_party_name.as_ref().map(|name| DeliveryParty {
                name: name.clone(),
                location_id: self.delivery_party_location_id.clone(),
            });

            let delivery_address = match (&self.delivery_city, &self.delivery_country) {
                (Some(city), Some(country)) => Some(DeliveryAddress {
                    street: self.delivery_street.clone(),
                    additional: self.delivery_additional.clone(),
                    city: city.clone(),
                    postal_code: self.delivery_postal.clone().unwrap_or_default(),
                    subdivision: self.delivery_subdivision.clone(),
                    country_code: country.clone(),
                }),
                _ => None,
            };

            // Only construct DeliveryInformation if at least one field is present
            if actual_delivery_date.is_some()
                || delivery_party.is_some()
                || delivery_address.is_some()
            {
                Some(DeliveryInformation {
                    actual_delivery_date,
                    delivery_party,
                    delivery_address,
                })
            } else {
                None
            }
        };

        let payee = self.payee_name.map(|name| Payee {
            name,
            identifier: self.payee_identifier,
            legal_registration_id: self.payee_legal_reg_id,
        });

        let tax_representative =
            if let (Some(name), Some(vat_id)) = (self.tax_rep_name, self.tax_rep_vat_id) {
                Some(TaxRepresentative {
                    name,
                    vat_id,
                    address: Address {
                        street: self.tax_rep_street,
                        additional: self.tax_rep_additional,
                        city: self.tax_rep_city.unwrap_or_default(),
                        postal_code: self.tax_rep_postal.unwrap_or_default(),
                        country_code: self.tax_rep_country.unwrap_or_default(),
                        subdivision: self.tax_rep_subdivision,
                    },
                })
            } else {
                None
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
            allowances: doc_allowances,
            charges: doc_charges,
            totals,
            payment_terms: self.payment_terms,
            payment,
            tax_point_date: self
                .tax_point_date
                .as_deref()
                .and_then(|d| parse_date(d).ok()),
            invoicing_period,
            payee,
            tax_representative,
            preceding_invoices,
            attachments,
            delivery,
        })
    }
}
