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

        // BR-DE-27: Telephone must contain at least 3 digits (warning-level, but we enforce)
        if let Some(phone) = &contact.phone {
            let digit_count = phone.chars().filter(|c| c.is_ascii_digit()).count();
            if digit_count < 3 && !phone.trim().is_empty() {
                errors.push(ValidationError::with_rule(
                    "seller.contact.phone",
                    "Telephone number (BT-42) must contain at least 3 digits",
                    "BR-DE-27",
                ));
            }
        }

        // BR-DE-28-warning: Email must contain exactly one @ with text on both sides
        if let Some(email) = &contact.email {
            if !email.trim().is_empty() {
                let at_count = email.chars().filter(|c| *c == '@').count();
                let parts: Vec<&str> = email.splitn(2, '@').collect();
                if at_count != 1
                    || parts.len() != 2
                    || parts[0].trim().is_empty()
                    || parts[1].trim().is_empty()
                {
                    errors.push(ValidationError::with_rule(
                        "seller.contact.email",
                        "Email address (BT-43) must contain exactly one @ with non-empty local and domain parts",
                        "BR-DE-28",
                    ));
                }
            }
        }
    }

    // BR-DE-14: Each VAT breakdown must have a rate
    if let Some(totals) = &invoice.totals {
        for (i, breakdown) in totals.vat_breakdown.iter().enumerate() {
            if breakdown.rate.is_sign_negative() {
                errors.push(ValidationError::with_rule(
                    format!("totals.vat_breakdown[{}].rate", i),
                    "VAT category rate (BT-119) must be provided and non-negative",
                    "BR-DE-14",
                ));
            }
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
    let allowed_type_codes = [326, 380, 381, 384, 389, 875, 876, 877];
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

    // BR-DE-18: Payment terms (BT-20) Skonto format check
    // When present, must match pattern: #SKONTO#TAGE=N#PROZENT=N.NN# (or Netto)
    // We validate the basic structure if it contains #SKONTO#
    if let Some(terms) = &invoice.payment_terms {
        if terms.contains("#SKONTO#") && !is_valid_skonto_format(terms) {
            errors.push(ValidationError::with_rule(
                "payment_terms",
                "Payment terms containing #SKONTO# must follow XRechnung format: #SKONTO#TAGE=N#PROZENT=N.NN#",
                "BR-DE-18",
            ));
        }
    }

    // BR-DE-22: Unique embedded document filenames
    if invoice.attachments.len() > 1 {
        let mut filenames = std::collections::HashSet::new();
        for (i, att) in invoice.attachments.iter().enumerate() {
            if let Some(ref emb) = att.embedded_document {
                if !filenames.insert(&emb.filename) {
                    errors.push(ValidationError::with_rule(
                        format!("attachments[{}].embedded_document.filename", i),
                        format!(
                            "Embedded document filenames must be unique; duplicate: '{}'",
                            emb.filename
                        ),
                        "BR-DE-22",
                    ));
                }
            }
        }
    }

    // BR-DE-26-warning: Corrected invoices (384) should have preceding invoice reference
    if invoice.type_code.code() == 384 && invoice.preceding_invoices.is_empty() {
        errors.push(ValidationError::with_rule(
            "preceding_invoices",
            "Corrected invoice (type 384) should reference the preceding invoice (BG-3)",
            "BR-DE-26",
        ));
    }

    // Payment-related rules
    if let Some(payment) = &invoice.payment {
        let code = payment.means_code.code();

        // BR-DE-23: Payment means code restricted set
        let allowed_means = [30, 48, 54, 55, 58, 59];
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

        // BR-DE-23a/23b: Credit transfer codes (30, 58) require BG-17, must not have BG-18/BG-19
        if code == 30 || code == 58 {
            if payment.credit_transfer.is_none() {
                errors.push(ValidationError::with_rule(
                    "payment.credit_transfer",
                    "Credit transfer codes (30, 58) require credit transfer information (BG-17)",
                    "BR-DE-23",
                ));
            }
            if payment.card_payment.is_some() || payment.direct_debit.is_some() {
                errors.push(ValidationError::with_rule(
                    "payment",
                    "Credit transfer codes (30, 58) must not include card payment (BG-18) or direct debit (BG-19)",
                    "BR-DE-23",
                ));
            }
        }

        // BR-DE-24a/24b: Card codes (48, 54, 55) require BG-18, must not have BG-17/BG-19
        if code == 48 || code == 54 || code == 55 {
            if payment.card_payment.is_none() {
                errors.push(ValidationError::with_rule(
                    "payment.card_payment",
                    "Card payment codes (48, 54, 55) require card payment information (BG-18)",
                    "BR-DE-24",
                ));
            }
            if payment.credit_transfer.is_some() || payment.direct_debit.is_some() {
                errors.push(ValidationError::with_rule(
                    "payment",
                    "Card payment codes (48, 54, 55) must not include credit transfer (BG-17) or direct debit (BG-19)",
                    "BR-DE-24",
                ));
            }
        }

        // BR-DE-25a/25b: Direct debit code (59) requires BG-19, must not have BG-17/BG-18
        if code == 59 {
            if payment.direct_debit.is_none() {
                errors.push(ValidationError::with_rule(
                    "payment.direct_debit",
                    "Direct debit code (59) requires direct debit information (BG-19)",
                    "BR-DE-25",
                ));
            }
            if payment.credit_transfer.is_some() || payment.card_payment.is_some() {
                errors.push(ValidationError::with_rule(
                    "payment",
                    "Direct debit code (59) must not include credit transfer (BG-17) or card payment (BG-18)",
                    "BR-DE-25",
                ));
            }
        }

        // BR-DE-19: IBAN validation for credit transfer (codes 30, 58)
        if code == 30 || code == 58 {
            if let Some(ct) = &payment.credit_transfer {
                if !ct.iban.trim().is_empty() && !is_valid_iban_format(&ct.iban) {
                    errors.push(ValidationError::with_rule(
                        "payment.credit_transfer.iban",
                        "IBAN (BT-84) must start with 2 uppercase letters followed by digits",
                        "BR-DE-19",
                    ));
                }
            }
        }

        // BR-DE-20: IBAN validation for direct debit (code 59)
        if code == 59 {
            if let Some(dd) = &payment.direct_debit {
                if let Some(iban) = &dd.debited_account_id {
                    if !iban.trim().is_empty() && !is_valid_iban_format(iban) {
                        errors.push(ValidationError::with_rule(
                            "payment.direct_debit.debited_account_id",
                            "Debited account IBAN (BT-91) must start with 2 uppercase letters followed by digits",
                            "BR-DE-20",
                        ));
                    }
                }
            }
        }

        // BR-DE-30: Direct debit requires creditor ID (BT-90)
        if payment.direct_debit.is_some() {
            if let Some(dd) = &payment.direct_debit {
                if dd.creditor_id.as_ref().is_none_or(|s| s.trim().is_empty()) {
                    errors.push(ValidationError::with_rule(
                        "payment.direct_debit.creditor_id",
                        "Direct debit requires bank assigned creditor identifier (BT-90)",
                        "BR-DE-30",
                    ));
                }

                // BR-DE-31: Direct debit requires debited account (BT-91)
                if dd
                    .debited_account_id
                    .as_ref()
                    .is_none_or(|s| s.trim().is_empty())
                {
                    errors.push(ValidationError::with_rule(
                        "payment.direct_debit.debited_account_id",
                        "Direct debit requires debited account identifier (BT-91)",
                        "BR-DE-31",
                    ));
                }
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
    // Note: this reuses the BR-DE-28 ID but is about electronic address, not email format
    if invoice.buyer.electronic_address.is_none() {
        errors.push(ValidationError::with_rule(
            "buyer.electronic_address",
            "XRechnung requires buyer electronic address (BT-49)",
            "BR-DE-28",
        ));
    }

    errors
}

/// Basic IBAN format check: 2 uppercase letters + 2 digits + up to 30 alphanumeric chars.
fn is_valid_iban_format(iban: &str) -> bool {
    let s = iban.replace(' ', "");
    if s.len() < 5 || s.len() > 34 {
        return false;
    }
    let bytes = s.as_bytes();
    bytes[0].is_ascii_uppercase()
        && bytes[1].is_ascii_uppercase()
        && bytes[2].is_ascii_digit()
        && bytes[3].is_ascii_digit()
        && bytes[4..].iter().all(|b| b.is_ascii_alphanumeric())
}

/// Validate Skonto format: lines must be #SKONTO#TAGE=...#PROZENT=...# or end with #
fn is_valid_skonto_format(terms: &str) -> bool {
    for line in terms.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("#SKONTO#") {
            if !trimmed.contains("TAGE=") || !trimmed.contains("PROZENT=") {
                return false;
            }
            if !trimmed.ends_with('#') {
                return false;
            }
        }
    }
    true
}
