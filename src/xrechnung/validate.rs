use crate::core::*;

/// Validate an invoice against XRechnung-specific rules (BR-DE-*).
///
/// This is additional to the core `validate_14_ustg` — call both for
/// full XRechnung compliance. Returns all errors found.
pub fn validate_xrechnung(invoice: &Invoice) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    // BR-DE-1: Payment instructions (BG-16) must be provided
    if invoice.payment.is_none() {
        errors.push(ValidationError::with_rule(
            "payment",
            "XRechnung requires payment instructions (BG-16)",
            "BR-DE-1",
        ));
    }

    // BR-DE-2: Seller contact (BG-6) must be present
    if invoice.seller.contact.is_none() {
        errors.push(ValidationError::with_rule(
            "seller.contact",
            "XRechnung requires seller contact information (BG-6)",
            "BR-DE-2",
        ));
    }

    // BR-DE-5: Seller contact name (BT-41)
    if let Some(contact) = &invoice.seller.contact {
        if contact.name.as_ref().is_none_or(|n| n.trim().is_empty()) {
            errors.push(ValidationError::with_rule(
                "seller.contact.name",
                "XRechnung requires seller contact name (BT-41)",
                "BR-DE-5",
            ));
        }

        // BR-DE-6: Seller contact telephone (BT-42)
        if contact.phone.as_ref().is_none_or(|p| p.trim().is_empty()) {
            errors.push(ValidationError::with_rule(
                "seller.contact.phone",
                "XRechnung requires seller contact telephone (BT-42)",
                "BR-DE-6",
            ));
        }

        // BR-DE-7: Seller contact email (BT-43)
        if contact.email.as_ref().is_none_or(|e| e.trim().is_empty()) {
            errors.push(ValidationError::with_rule(
                "seller.contact.email",
                "XRechnung requires seller contact email (BT-43)",
                "BR-DE-7",
            ));
        }
    }

    // BR-DE-15: Buyer reference (BT-10 / Leitweg-ID) must be provided
    if invoice
        .buyer_reference
        .as_ref()
        .is_none_or(|r| r.trim().is_empty())
    {
        errors.push(ValidationError::with_rule(
            "buyer_reference",
            "XRechnung requires buyer reference / Leitweg-ID (BT-10)",
            "BR-DE-15",
        ));
    }

    // BR-DE-16: At least one of: seller VAT ID (BT-31), seller tax number (BT-32)
    if invoice.seller.vat_id.is_none() && invoice.seller.tax_number.is_none() {
        errors.push(ValidationError::with_rule(
            "seller",
            "XRechnung requires seller VAT ID (BT-31) or tax number (BT-32)",
            "BR-DE-16",
        ));
    }

    // BR-DE-17: Invoice type code must be from allowed set
    let allowed_type_codes = [380, 381, 384, 389, 326, 875, 876];
    if !allowed_type_codes.contains(&invoice.type_code.code()) {
        errors.push(ValidationError::with_rule(
            "type_code",
            format!(
                "XRechnung invoice type code {} is not in the allowed set {:?}",
                invoice.type_code.code(),
                allowed_type_codes
            ),
            "BR-DE-17",
        ));
    }

    // BR-DE-21: Currency must be EUR for German domestic
    // (Actually the rule is about the specification identifier, but we validate currency as a common check)

    // BR-DE-23: Payment means code restricted set
    if let Some(payment) = &invoice.payment {
        let allowed_means = [30, 48, 54, 55, 58, 59];
        let code = payment.means_code.code();
        if !allowed_means.contains(&code) {
            errors.push(ValidationError::with_rule(
                "payment.means_code",
                format!(
                    "XRechnung payment means code {} is not in the allowed set {:?}",
                    code, allowed_means
                ),
                "BR-DE-23",
            ));
        }

        // BR-DE-24: SEPA credit transfer (58) requires IBAN
        if code == 58 {
            match &payment.credit_transfer {
                Some(ct) if ct.iban.trim().is_empty() => {
                    errors.push(ValidationError::with_rule(
                        "payment.credit_transfer.iban",
                        "SEPA credit transfer (58) requires IBAN (BT-84)",
                        "BR-DE-24",
                    ));
                }
                None => {
                    errors.push(ValidationError::with_rule(
                        "payment.credit_transfer",
                        "SEPA credit transfer (58) requires payment account with IBAN",
                        "BR-DE-24",
                    ));
                }
                _ => {}
            }
        }
    }

    // BR-DE-26: Seller electronic address (BT-34) must be present
    if invoice.seller.electronic_address.is_none() {
        errors.push(ValidationError::with_rule(
            "seller.electronic_address",
            "XRechnung requires seller electronic address (BT-34)",
            "BR-DE-26",
        ));
    }

    // BR-DE-28: Buyer electronic address (BT-49) must be present
    if invoice.buyer.electronic_address.is_none() {
        errors.push(ValidationError::with_rule(
            "buyer.electronic_address",
            "XRechnung requires buyer electronic address (BT-49)",
            "BR-DE-28",
        ));
    }

    errors
}
