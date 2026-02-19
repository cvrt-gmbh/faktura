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

    // BG-11: Seller tax representative party
    if let Some(tax_rep) = &invoice.tax_representative {
        w.start_element("ram:SellerTaxRepresentativeTradeParty")?;
        w.text_element("ram:Name", &tax_rep.name)?;
        w.start_element("ram:PostalTradeAddress")?;
        w.text_element("ram:PostcodeCode", &tax_rep.address.postal_code)?;
        if let Some(street) = &tax_rep.address.street {
            w.text_element("ram:LineOne", street)?;
        }
        if let Some(additional) = &tax_rep.address.additional {
            w.text_element("ram:LineTwo", additional)?;
        }
        w.text_element("ram:CityName", &tax_rep.address.city)?;
        w.text_element("ram:CountryID", &tax_rep.address.country_code)?;
        if let Some(sub) = &tax_rep.address.subdivision {
            w.text_element("ram:CountrySubDivisionName", sub)?;
        }
        w.end_element("ram:PostalTradeAddress")?;
        w.start_element("ram:SpecifiedTaxRegistration")?;
        w.text_element_with_attrs("ram:ID", &tax_rep.vat_id, &[("schemeID", "VA")])?;
        w.end_element("ram:SpecifiedTaxRegistration")?;
        w.end_element("ram:SellerTaxRepresentativeTradeParty")?;
    }
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

    // BT-72: Actual delivery date (BG-13)
    if let Some(delivery) = &invoice.delivery {
        if let Some(actual_delivery_date) = delivery.actual_delivery_date {
            w.start_element("ram:ActualDeliverySupplyChainEvent")?;
            w.start_element("ram:OccurrenceDateTime")?;
            w.text_element(
                "udt:DateTimeString",
                &format!("{}", actual_delivery_date.format("%Y%m%d")),
            )?;
            w.end_element("ram:OccurrenceDateTime")?;
            w.end_element("ram:ActualDeliverySupplyChainEvent")?;
        }

        // BG-15: Deliver-to party (BT-70 name, BT-71 location_id)
        if let Some(delivery_party) = &delivery.delivery_party {
            w.start_element("ram:ShipToTradeParty")?;
            w.text_element("ram:Name", &delivery_party.name)?;
            if let Some(location_id) = &delivery_party.location_id {
                w.text_element("ram:ID", location_id)?;
            }

            // BG-15: Delivery address (BT-75-80)
            if let Some(delivery_address) = &delivery.delivery_address {
                w.start_element("ram:PostalTradeAddress")?;

                if let Some(street) = &delivery_address.street {
                    w.text_element("ram:LineOne", street)?;
                }
                if let Some(additional) = &delivery_address.additional {
                    w.text_element("ram:LineTwo", additional)?;
                }

                w.text_element("ram:CityName", &delivery_address.city)?;
                w.text_element("ram:PostcodeCode", &delivery_address.postal_code)?;

                if let Some(subdivision) = &delivery_address.subdivision {
                    w.text_element("ram:CountrySubDivisionName", subdivision)?;
                }

                w.text_element("ram:CountryID", &delivery_address.country_code)?;

                w.end_element("ram:PostalTradeAddress")?;
            }

            w.end_element("ram:ShipToTradeParty")?;
        }
    } else if let Some(tpd) = &invoice.tax_point_date {
        // Fallback for tax_point_date only (legacy behavior)
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
    // BT-83: Payment reference (Verwendungszweck)
    if let Some(payment) = &invoice.payment {
        if let Some(ri) = &payment.remittance_info {
            w.text_element("ram:PaymentReference", ri)?;
        }
    }
    if let Some(tcc) = &invoice.tax_currency_code {
        w.text_element("ram:TaxCurrencyCode", tcc)?;
    }
    w.text_element("ram:InvoiceCurrencyCode", currency)?;

    // BG-10: Payee party
    if let Some(payee) = &invoice.payee {
        w.start_element("ram:PayeeTradeParty")?;
        if let Some(id) = &payee.identifier {
            w.text_element("ram:ID", id)?;
        }
        w.text_element("ram:Name", &payee.name)?;
        if let Some(reg_id) = &payee.legal_registration_id {
            w.start_element("ram:SpecifiedLegalOrganization")?;
            w.text_element("ram:ID", reg_id)?;
            w.end_element("ram:SpecifiedLegalOrganization")?;
        }
        w.end_element("ram:PayeeTradeParty")?;
    }

    // Payment means
    if let Some(payment) = &invoice.payment {
        w.start_element("ram:SpecifiedTradeSettlementPaymentMeans")?;
        w.text_element("ram:TypeCode", &payment.means_code.code().to_string())?;
        // BT-82: Payment means text
        if let Some(text) = &payment.means_text {
            w.text_element("ram:Information", text)?;
        }
        // BG-18: Card payment
        if let Some(card) = &payment.card_payment {
            w.start_element("ram:ApplicableTradeSettlementFinancialCard")?;
            w.text_element("ram:ID", &card.account_number)?;
            if let Some(holder) = &card.holder_name {
                w.text_element("ram:CardholderName", holder)?;
            }
            w.end_element("ram:ApplicableTradeSettlementFinancialCard")?;
        }
        // BG-19: Direct debit
        if let Some(dd) = &payment.direct_debit {
            if let Some(account_id) = &dd.debited_account_id {
                w.start_element("ram:PayerPartyDebtorFinancialAccount")?;
                w.text_element("ram:IBANID", account_id)?;
                w.end_element("ram:PayerPartyDebtorFinancialAccount")?;
            }
        }
        // BG-17: Credit transfer
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

    // BG-19: Direct debit mandate/creditor in settlement context
    if let Some(payment) = &invoice.payment {
        if let Some(dd) = &payment.direct_debit {
            if let Some(mandate_id) = &dd.mandate_id {
                w.text_element("ram:DirectDebitMandateID", mandate_id)?;
            }
            if let Some(creditor_id) = &dd.creditor_id {
                w.text_element("ram:CreditorReferenceID", creditor_id)?;
            }
        }
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
    // BT-127: Line note
    if let Some(note) = &line.note {
        w.start_element("ram:IncludedNote")?;
        w.text_element("ram:Content", note)?;
        w.end_element("ram:IncludedNote")?;
    }
    w.end_element("ram:AssociatedDocumentLineDocument")?;

    // Product
    w.start_element("ram:SpecifiedTradeProduct")?;
    if let Some(std_id) = &line.standard_item_id {
        w.text_element_with_attrs("ram:GlobalID", std_id, &[("schemeID", "0160")])?;
    }
    if let Some(sid) = &line.seller_item_id {
        w.text_element("ram:SellerAssignedID", sid)?;
    }
    // BT-156: Buyer's item identifier
    if let Some(bid) = &line.buyer_item_id {
        w.text_element("ram:BuyerAssignedID", bid)?;
    }
    w.text_element("ram:Name", &line.item_name)?;
    if let Some(desc) = &line.description {
        w.text_element("ram:Description", desc)?;
    }
    // BT-159: Item country of origin
    if let Some(country) = &line.origin_country {
        w.start_element("ram:OriginTradeCountry")?;
        w.text_element("ram:ID", country)?;
        w.end_element("ram:OriginTradeCountry")?;
    }
    // BT-160/BT-161: Item attributes
    for attr in &line.attributes {
        w.start_element("ram:ApplicableProductCharacteristic")?;
        w.text_element("ram:Description", &attr.name)?;
        w.text_element("ram:Value", &attr.value)?;
        w.end_element("ram:ApplicableProductCharacteristic")?;
    }
    w.end_element("ram:SpecifiedTradeProduct")?;

    // Trade agreement (BG-29: price details)
    w.start_element("ram:SpecifiedLineTradeAgreement")?;
    if let Some(gp) = line.gross_price {
        w.start_element("ram:GrossPriceProductTradePrice")?;
        w.text_element("ram:ChargeAmount", &format_decimal(gp))?;
        // BT-147: Price discount as AppliedTradeAllowanceCharge
        let discount = gp - line.unit_price;
        if discount > Decimal::ZERO {
            w.start_element("ram:AppliedTradeAllowanceCharge")?;
            w.text_element("ram:ChargeIndicator", "false")?;
            w.text_element("ram:ActualAmount", &format_decimal(discount))?;
            w.end_element("ram:AppliedTradeAllowanceCharge")?;
        }
        w.end_element("ram:GrossPriceProductTradePrice")?;
    }
    w.start_element("ram:NetPriceProductTradePrice")?;
    w.text_element("ram:ChargeAmount", &format_decimal(line.unit_price))?;
    // BT-149/BT-150: Base quantity
    if let Some(bq) = line.base_quantity {
        let bq_unit = line.base_quantity_unit.as_deref().unwrap_or(line.unit.as_str());
        w.text_element_with_attrs(
            "ram:BasisQuantity",
            &format_decimal(bq),
            &[("unitCode", bq_unit)],
        )?;
    }
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
    // BG-27/BG-28: Line allowances and charges
    for ac in line.allowances.iter().chain(line.charges.iter()) {
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
        w.end_element("ram:SpecifiedTradeAllowanceCharge")?;
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
                    || name == "ram:BasisQuantity"
                    || name == "ram:GlobalID"
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
                // Line-level or document-level SpecifiedTradeAllowanceCharge
                if ended == "ram:SpecifiedTradeAllowanceCharge" {
                    let in_line_ctx = path
                        .iter()
                        .any(|p| p == "ram:IncludedSupplyChainTradeLineItem");
                    if in_line_ctx {
                        if let Some(line) = p.current_line.as_mut() {
                            if let Some(ac) = line.current_ac.take() {
                                line.allowances_charges.push(ac);
                            }
                        }
                    } else if let Some(ac) = p.current_doc_ac.take() {
                        p.doc_allowances_charges.push(ac);
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
    seller_subdivision: Option<String>,
    seller_contact_name: Option<String>,
    seller_contact_phone: Option<String>,
    seller_contact_email: Option<String>,
    seller_endpoint_scheme: Option<String>,
    seller_endpoint_value: Option<String>,

    buyer_name: Option<String>,
    buyer_vat_id: Option<String>,
    buyer_trading_name: Option<String>,
    buyer_registration_id: Option<String>,
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

    // BG-10: Payee
    payee_name: Option<String>,
    payee_identifier: Option<String>,
    payee_legal_reg_id: Option<String>,

    // BG-11: Seller tax representative
    tax_rep_name: Option<String>,
    tax_rep_vat_id: Option<String>,
    tax_rep_street: Option<String>,
    tax_rep_additional: Option<String>,
    tax_rep_city: Option<String>,
    tax_rep_postal: Option<String>,
    tax_rep_country: Option<String>,
    tax_rep_subdivision: Option<String>,

    payment_means_code: Option<String>,
    payment_means_text: Option<String>,
    payment_remittance_info: Option<String>,
    payment_iban: Option<String>,
    payment_bic: Option<String>,
    payment_account_name: Option<String>,
    payment_terms: Option<String>,
    due_date: Option<String>,
    // BG-18: Card payment
    card_account_number: Option<String>,
    card_holder_name: Option<String>,
    // BG-19: Direct debit
    direct_debit_mandate_id: Option<String>,
    direct_debit_account_id: Option<String>,
    direct_debit_creditor_id: Option<String>,

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

    // Document-level allowances/charges
    doc_allowances_charges: Vec<CiiAllowanceCharge>,
    current_doc_ac: Option<CiiAllowanceCharge>,

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
    buyer_item_id: Option<String>,
    standard_item_id: Option<String>,
    standard_item_scheme: Option<String>,
    note: Option<String>,
    quantity: Option<String>,
    unit: Option<String>,
    price: Option<String>,
    gross_price: Option<String>,
    base_quantity: Option<String>,
    base_quantity_unit: Option<String>,
    line_total: Option<String>,
    tax_category: Option<String>,
    tax_rate: Option<String>,
    origin_country: Option<String>,
    attributes: Vec<(String, String)>,
    current_attr_name: Option<String>,
    invoicing_period_start: Option<String>,
    invoicing_period_end: Option<String>,
    allowances_charges: Vec<CiiAllowanceCharge>,
    current_ac: Option<CiiAllowanceCharge>,
}

#[derive(Default, Clone)]
struct CiiAllowanceCharge {
    is_charge: Option<String>,
    amount: Option<String>,
    reason: Option<String>,
    reason_code: Option<String>,
    tax_category: Option<String>,
    tax_rate: Option<String>,
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

        // BG-13: Delivery information (BT-72 actual delivery date)
        // Check if we're in ActualDeliverySupplyChainEvent (BT-72)
        let in_delivery_event = path
            .iter()
            .any(|p| p == "ram:ActualDeliverySupplyChainEvent");
        if in_delivery_event && leaf == "udt:DateTimeString" && parent == "ram:OccurrenceDateTime" {
            self.delivery_actual_date = Some(text.to_string());
        }

        // BG-15: Delivery address and party (BT-75-80, BT-82)
        // Delivery party information
        let in_header_delivery = path
            .iter()
            .any(|p| p == "ram:ApplicableHeaderTradeDelivery");
        let in_ship_to = path.iter().any(|p| p == "ram:ShipToTradeParty");
        if in_header_delivery && in_ship_to && !in_line {
            match leaf {
                "ram:Name" if parent == "ram:ShipToTradeParty" => {
                    self.delivery_party_name = Some(text.to_string());
                }
                "ram:ID" if parent == "ram:ShipToTradeParty" => {
                    self.delivery_party_location_id = Some(text.to_string());
                }
                _ => {}
            }
        }

        // Delivery address
        let in_postal_addr = path.iter().any(|p| p == "ram:PostalTradeAddress");
        if in_header_delivery && in_postal_addr && !in_line {
            match leaf {
                "ram:LineOne" => self.delivery_street = Some(text.to_string()),
                "ram:LineTwo" => self.delivery_additional = Some(text.to_string()),
                "ram:CityName" => self.delivery_city = Some(text.to_string()),
                "ram:PostcodeCode" => self.delivery_postal = Some(text.to_string()),
                "ram:CountrySubDivisionName" => self.delivery_subdivision = Some(text.to_string()),
                "ram:CountryID" => self.delivery_country = Some(text.to_string()),
                _ => {}
            }
        }

        // BG-14: Document-level invoicing period (in delivery)
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
                "ram:CountrySubDivisionName" => {
                    self.seller_subdivision = Some(text.to_string())
                }
                _ => {}
            }
        }

        // BG-11: Seller tax representative
        let in_tax_rep = path
            .iter()
            .any(|p| p == "ram:SellerTaxRepresentativeTradeParty");
        if in_tax_rep && !in_line {
            match leaf {
                "ram:Name" if parent == "ram:SellerTaxRepresentativeTradeParty" => {
                    self.tax_rep_name = Some(text.to_string());
                }
                "ram:ID" if parent == "ram:SpecifiedTaxRegistration" => {
                    let scheme = self.current_scheme_id.take().unwrap_or_default();
                    if scheme == "VA" {
                        self.tax_rep_vat_id = Some(text.to_string());
                    }
                }
                "ram:LineOne" => self.tax_rep_street = Some(text.to_string()),
                "ram:LineTwo" => self.tax_rep_additional = Some(text.to_string()),
                "ram:CityName" => self.tax_rep_city = Some(text.to_string()),
                "ram:PostcodeCode" => self.tax_rep_postal = Some(text.to_string()),
                "ram:CountryID" => self.tax_rep_country = Some(text.to_string()),
                "ram:CountrySubDivisionName" => {
                    self.tax_rep_subdivision = Some(text.to_string())
                }
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
                "ram:ID" if parent == "ram:SpecifiedLegalOrganization" => {
                    self.buyer_registration_id = Some(text.to_string());
                }
                "ram:TradingBusinessName" => {
                    self.buyer_trading_name = Some(text.to_string())
                }
                "ram:PersonName" => self.buyer_contact_name = Some(text.to_string()),
                "ram:CompleteNumber" => self.buyer_contact_phone = Some(text.to_string()),
                "ram:URIID" if parent == "ram:EmailURIUniversalCommunication" => {
                    self.buyer_contact_email = Some(text.to_string());
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
                "ram:CountrySubDivisionName" => {
                    self.buyer_subdivision = Some(text.to_string())
                }
                _ => {}
            }
        }

        // BG-10: Payee party
        let in_payee = path.iter().any(|p| p == "ram:PayeeTradeParty");
        if in_payee && !in_line {
            match leaf {
                "ram:Name" if parent == "ram:PayeeTradeParty" => {
                    self.payee_name = Some(text.to_string());
                }
                "ram:ID" if parent == "ram:PayeeTradeParty" => {
                    self.payee_identifier = Some(text.to_string());
                }
                "ram:ID" if parent == "ram:SpecifiedLegalOrganization" => {
                    self.payee_legal_reg_id = Some(text.to_string());
                }
                _ => {}
            }
        }

        // Payment
        if in_settlement && !in_line {
            if leaf == "ram:TypeCode" && parent == "ram:SpecifiedTradeSettlementPaymentMeans" {
                self.payment_means_code = Some(text.to_string());
            }
            // BT-82: Payment means text
            if leaf == "ram:Information" && parent == "ram:SpecifiedTradeSettlementPaymentMeans" {
                self.payment_means_text = Some(text.to_string());
            }
            // BT-83: Payment reference
            if leaf == "ram:PaymentReference" {
                self.payment_remittance_info = Some(text.to_string());
            }
            // BG-18: Card payment
            let in_card = path
                .iter()
                .any(|p| p == "ram:ApplicableTradeSettlementFinancialCard");
            if in_card {
                if leaf == "ram:ID" {
                    self.card_account_number = Some(text.to_string());
                }
                if leaf == "ram:CardholderName" {
                    self.card_holder_name = Some(text.to_string());
                }
            }
            // BG-19: Direct debit
            if leaf == "ram:DirectDebitMandateID" {
                self.direct_debit_mandate_id = Some(text.to_string());
            }
            if leaf == "ram:CreditorReferenceID" {
                self.direct_debit_creditor_id = Some(text.to_string());
            }
            if leaf == "ram:IBANID" {
                let in_debtor = path
                    .iter()
                    .any(|p| p == "ram:PayerPartyDebtorFinancialAccount");
                if in_debtor {
                    self.direct_debit_account_id = Some(text.to_string());
                } else {
                    self.payment_iban = Some(text.to_string());
                }
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

        // BG-20/BG-21: Document-level SpecifiedTradeAllowanceCharge
        let in_doc_ac = path
            .iter()
            .any(|p| p == "ram:SpecifiedTradeAllowanceCharge")
            && in_settlement
            && !in_line;
        if in_doc_ac {
            let ac = self.current_doc_ac.get_or_insert_with(Default::default);
            match leaf {
                "ram:ChargeIndicator" => ac.is_charge = Some(text.to_string()),
                "ram:ActualAmount" => ac.amount = Some(text.to_string()),
                "ram:Reason" => ac.reason = Some(text.to_string()),
                "ram:ReasonCode" => ac.reason_code = Some(text.to_string()),
                "ram:CategoryCode" if path.iter().any(|p| p == "ram:CategoryTradeTax") => {
                    ac.tax_category = Some(text.to_string())
                }
                "ram:RateApplicablePercent" if path.iter().any(|p| p == "ram:CategoryTradeTax") => {
                    ac.tax_rate = Some(text.to_string())
                }
                _ => {}
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
                // Check if we're inside a line-level SpecifiedTradeAllowanceCharge
                let in_line_ac = path
                    .iter()
                    .any(|p| p == "ram:SpecifiedTradeAllowanceCharge");

                if in_line_ac {
                    let ac = line.current_ac.get_or_insert_with(Default::default);
                    match leaf {
                        "ram:ChargeIndicator" => ac.is_charge = Some(text.to_string()),
                        "ram:ActualAmount" => ac.amount = Some(text.to_string()),
                        "ram:Reason" => ac.reason = Some(text.to_string()),
                        "ram:ReasonCode" => ac.reason_code = Some(text.to_string()),
                        "ram:CategoryCode" => ac.tax_category = Some(text.to_string()),
                        "ram:RateApplicablePercent" => ac.tax_rate = Some(text.to_string()),
                        _ => {}
                    }
                } else {
                    match leaf {
                        "ram:LineID" => line.id = Some(text.to_string()),
                        // BT-127: Line note
                        "ram:Content"
                            if parent == "ram:IncludedNote"
                                && path.iter().any(|p| {
                                    p == "ram:AssociatedDocumentLineDocument"
                                }) =>
                        {
                            line.note = Some(text.to_string());
                        }
                        "ram:Name" if parent == "ram:SpecifiedTradeProduct" => {
                            line.name = Some(text.to_string())
                        }
                        "ram:Description" if parent == "ram:SpecifiedTradeProduct" => {
                            line.description = Some(text.to_string())
                        }
                        "ram:SellerAssignedID" => line.seller_item_id = Some(text.to_string()),
                        // BT-156: Buyer's item identifier
                        "ram:BuyerAssignedID" => line.buyer_item_id = Some(text.to_string()),
                        // BT-157: Standard item identifier (flat GlobalID with schemeID)
                        "ram:GlobalID" => {
                            line.standard_item_id = Some(text.to_string());
                            line.standard_item_scheme = self.current_scheme_id.take();
                        }
                        "ram:BilledQuantity" => {
                            line.quantity = Some(text.to_string());
                            line.unit = self.current_unit_code.take();
                        }
                        // BT-149/BT-150: Base quantity
                        "ram:BasisQuantity" => {
                            line.base_quantity = Some(text.to_string());
                            line.base_quantity_unit = self.current_unit_code.take();
                        }
                        "ram:ChargeAmount" if parent == "ram:NetPriceProductTradePrice" => {
                            line.price = Some(text.to_string());
                        }
                        "ram:ChargeAmount" if parent == "ram:GrossPriceProductTradePrice" => {
                            line.gross_price = Some(text.to_string());
                        }
                        "ram:LineTotalAmount" => line.line_total = Some(text.to_string()),
                        "ram:CategoryCode"
                            if path.iter().any(|p| p == "ram:ApplicableTradeTax") =>
                        {
                            line.tax_category = Some(text.to_string());
                        }
                        "ram:RateApplicablePercent"
                            if path.iter().any(|p| p == "ram:ApplicableTradeTax") =>
                        {
                            line.tax_rate = Some(text.to_string());
                        }
                        // BT-159: Item country of origin
                        "ram:ID" if parent == "ram:OriginTradeCountry" => {
                            line.origin_country = Some(text.to_string());
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    fn convert_allowance_charges(
        parsed: Vec<CiiAllowanceCharge>,
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
                percentage: None,
                base_amount: None,
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
            registration_id: self.buyer_registration_id,
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

            let gross_price = pl
                .gross_price
                .as_deref()
                .and_then(|s| parse_decimal(s).ok());
            let (line_allowances, line_charges) =
                Self::convert_allowance_charges(pl.allowances_charges, &parse_decimal)?;

            lines.push(LineItem {
                id: pl.id.unwrap_or_default(),
                quantity: parse_decimal(pl.quantity.as_deref().unwrap_or("1"))?,
                unit: pl.unit.unwrap_or_else(|| "C62".to_string()),
                unit_price: parse_decimal(pl.price.as_deref().unwrap_or("0"))?,
                gross_price,
                allowances: line_allowances,
                charges: line_charges,
                tax_category: TaxCategory::from_code(pl.tax_category.as_deref().unwrap_or("S"))
                    .unwrap_or(TaxCategory::StandardRate),
                tax_rate: parse_decimal(pl.tax_rate.as_deref().unwrap_or("0"))?,
                item_name: pl.name.unwrap_or_default(),
                description: pl.description,
                seller_item_id: pl.seller_item_id,
                buyer_item_id: pl.buyer_item_id,
                standard_item_id: pl.standard_item_id,
                note: pl.note,
                base_quantity: pl
                    .base_quantity
                    .as_deref()
                    .and_then(|s| parse_decimal(s).ok()),
                base_quantity_unit: pl.base_quantity_unit,
                origin_country: pl.origin_country,
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
                card_payment: if self.card_account_number.is_some() {
                    Some(CardPayment {
                        account_number: self.card_account_number.unwrap_or_default(),
                        holder_name: self.card_holder_name,
                    })
                } else {
                    None
                },
                direct_debit: if self.direct_debit_mandate_id.is_some()
                    || self.direct_debit_account_id.is_some()
                    || self.direct_debit_creditor_id.is_some()
                {
                    Some(DirectDebit {
                        mandate_id: self.direct_debit_mandate_id,
                        debited_account_id: self.direct_debit_account_id,
                        creditor_id: self.direct_debit_creditor_id,
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

        let (doc_allowances, doc_charges) =
            Self::convert_allowance_charges(self.doc_allowances_charges, &parse_decimal)?;

        // Build delivery information from parsed fields (BG-13/BG-14/BG-15)
        let delivery = {
            let actual_delivery_date = self
                .delivery_actual_date
                .as_deref()
                .and_then(|d| parse_cii_date(d).ok());

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

        let tax_representative = self.tax_rep_name.map(|name| TaxRepresentative {
            name,
            vat_id: self.tax_rep_vat_id.unwrap_or_default(),
            address: Address {
                street: self.tax_rep_street,
                additional: self.tax_rep_additional,
                city: self.tax_rep_city.unwrap_or_default(),
                postal_code: self.tax_rep_postal.unwrap_or_default(),
                country_code: self.tax_rep_country.unwrap_or_default(),
                subdivision: self.tax_rep_subdivision,
            },
        });

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
            payee,
            tax_representative,
            lines,
            vat_scenario: VatScenario::Domestic,
            allowances: doc_allowances,
            charges: doc_charges,
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
            delivery,
        })
    }
}
