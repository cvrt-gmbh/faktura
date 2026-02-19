//! Peppol BIS Billing 3.0 validation rules (PEPPOL-EN16931-Rxxx).
//!
//! These rules are stricter than base EN 16931 and XRechnung.

use rust_decimal::Decimal;

use crate::core::*;

/// Run all validation layers for Peppol BIS 3.0 compliance in one call.
///
/// Combines `validate_14_ustg`, `validate_en16931`, and `validate_peppol`
/// into a single convenience function. Returns all errors found.
pub fn validate_peppol_full(invoice: &Invoice) -> Vec<ValidationError> {
    let mut errors = crate::core::validate_14_ustg(invoice);
    errors.extend(crate::core::validate_en16931(invoice));
    errors.extend(validate_peppol(invoice));
    errors
}

/// Validate an invoice against Peppol BIS Billing 3.0 rules.
///
/// Returns a list of validation errors. An empty list means the invoice
/// passes all Peppol-specific checks.
pub fn validate_peppol(invoice: &Invoice) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    // PEPPOL-EN16931-R003: Buyer reference OR order reference must be present
    if invoice.buyer_reference.is_none() && invoice.order_reference.is_none() {
        errors.push(ValidationError {
            field: "buyer_reference".into(),
            message: "buyer reference or order reference is required".into(),
            rule: Some("PEPPOL-EN16931-R003".into()),
        });
    }

    // PEPPOL-EN16931-R020: Seller electronic address must be provided
    if invoice.seller.electronic_address.is_none() {
        errors.push(ValidationError {
            field: "seller.electronic_address".into(),
            message: "seller electronic address (EndpointID) is required".into(),
            rule: Some("PEPPOL-EN16931-R020".into()),
        });
    }

    // PEPPOL-EN16931-R010: Buyer electronic address must be provided
    if invoice.buyer.electronic_address.is_none() {
        errors.push(ValidationError {
            field: "buyer.electronic_address".into(),
            message: "buyer electronic address (EndpointID) is required".into(),
            rule: Some("PEPPOL-EN16931-R010".into()),
        });
    }

    // PEPPOL-EN16931-R008: No empty fields in required positions
    if invoice.number.is_empty() {
        errors.push(ValidationError {
            field: "number".into(),
            message: "invoice number must not be empty".into(),
            rule: Some("PEPPOL-EN16931-R008".into()),
        });
    }
    if invoice.seller.name.is_empty() {
        errors.push(ValidationError {
            field: "seller.name".into(),
            message: "seller name must not be empty".into(),
            rule: Some("PEPPOL-EN16931-R008".into()),
        });
    }
    if invoice.buyer.name.is_empty() {
        errors.push(ValidationError {
            field: "buyer.name".into(),
            message: "buyer name must not be empty".into(),
            rule: Some("PEPPOL-EN16931-R008".into()),
        });
    }

    // PEPPOL-EN16931-R051: All currency IDs must match DocumentCurrencyCode
    // (checked implicitly since we use a single currency_code field)

    // PEPPOL-EN16931-P0100/P0101: Invoice type code restrictions
    match invoice.type_code {
        InvoiceTypeCode::Invoice
        | InvoiceTypeCode::CreditNote
        | InvoiceTypeCode::Corrected
        | InvoiceTypeCode::Prepayment
        | InvoiceTypeCode::Partial => {}
        InvoiceTypeCode::Other(code) => {
            errors.push(ValidationError::with_rule(
                "type_code",
                format!("Peppol does not support invoice type code {}", code),
                "PEPPOL-EN16931-P0100",
            ));
        }
    }

    // PEPPOL-EN16931-P0112: Type codes 326/384 only when both parties are German
    if matches!(
        invoice.type_code,
        InvoiceTypeCode::Partial | InvoiceTypeCode::Corrected
    ) {
        let seller_de = invoice.seller.address.country_code == "DE";
        let buyer_de = invoice.buyer.address.country_code == "DE";
        if !seller_de || !buyer_de {
            errors.push(ValidationError {
                field: "type_code".into(),
                message: format!(
                    "invoice type code {} is only allowed when both seller and buyer are in Germany",
                    invoice.type_code.code()
                ),
                rule: Some("PEPPOL-EN16931-P0112".into()),
            });
        }
    }

    // PEPPOL-EN16931-R061: Mandate reference required for direct debit
    if let Some(ref payment) = invoice.payment {
        if matches!(
            payment.means_code,
            PaymentMeansCode::DirectDebit | PaymentMeansCode::SepaDirectDebit
        ) && payment.remittance_info.is_none()
        {
            errors.push(ValidationError {
                field: "payment.remittance_info".into(),
                message: "mandate reference is required for direct debit payments".into(),
                rule: Some("PEPPOL-EN16931-R061".into()),
            });
        }
    }

    // PEPPOL-EN16931-R053: Exactly one TaxTotal with subtotals
    if let Some(ref totals) = invoice.totals {
        if totals.vat_breakdown.is_empty() {
            errors.push(ValidationError {
                field: "totals.vat_breakdown".into(),
                message: "at least one tax subtotal is required".into(),
                rule: Some("PEPPOL-EN16931-R053".into()),
            });
        }
    }

    // PEPPOL-EN16931-R121: Line base quantities must be positive
    for (i, line) in invoice.lines.iter().enumerate() {
        if line.quantity <= Decimal::ZERO {
            errors.push(ValidationError {
                field: format!("lines[{i}].quantity"),
                message: "invoiced quantity must be positive".into(),
                rule: Some("PEPPOL-EN16931-R121".into()),
            });
        }
    }

    // PEPPOL-EN16931-R044: No charges at price level (only allowances)
    for (i, line) in invoice.lines.iter().enumerate() {
        for (j, ac) in line.charges.iter().enumerate() {
            if ac.is_charge {
                errors.push(ValidationError {
                    field: format!("lines[{i}].charges[{j}]"),
                    message: "charges at line price level are not allowed in Peppol".into(),
                    rule: Some("PEPPOL-EN16931-R044".into()),
                });
            }
        }
    }

    // PEPPOL-EN16931-R041/R042: Percentage and base amount must be paired
    for ac in invoice.allowances.iter().chain(invoice.charges.iter()) {
        if ac.percentage.is_some() && ac.base_amount.is_none() {
            errors.push(ValidationError {
                field: "allowances/charges".into(),
                message: "base amount is required when percentage is provided".into(),
                rule: Some("PEPPOL-EN16931-R041".into()),
            });
        }
        if ac.base_amount.is_some() && ac.percentage.is_none() {
            errors.push(ValidationError {
                field: "allowances/charges".into(),
                message: "percentage is required when base amount is provided".into(),
                rule: Some("PEPPOL-EN16931-R042".into()),
            });
        }
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn valid_peppol_invoice() -> Invoice {
        InvoiceBuilder::new("PEPPOL-001", date(2024, 6, 15))
            .buyer_reference("BR-123")
            .tax_point_date(date(2024, 6, 15))
            .seller(
                PartyBuilder::new(
                    "Seller GmbH",
                    AddressBuilder::new("Berlin", "10115", "DE").build(),
                )
                .vat_id("DE123456789")
                .electronic_address("EM", "seller@example.com")
                .build(),
            )
            .buyer(
                PartyBuilder::new(
                    "Buyer AG",
                    AddressBuilder::new("München", "80331", "DE").build(),
                )
                .electronic_address("EM", "buyer@example.com")
                .build(),
            )
            .add_line(
                LineItemBuilder::new("1", "Consulting", dec!(10), "HUR", dec!(100))
                    .tax(TaxCategory::StandardRate, dec!(19))
                    .build(),
            )
            .build()
            .unwrap()
    }

    #[test]
    fn valid_invoice_passes() {
        let errors = validate_peppol(&valid_peppol_invoice());
        assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
    }

    #[test]
    fn missing_buyer_reference_and_order_ref() {
        let mut inv = valid_peppol_invoice();
        inv.buyer_reference = None;
        inv.order_reference = None;
        let errors = validate_peppol(&inv);
        assert!(
            errors
                .iter()
                .any(|e| e.rule.as_deref() == Some("PEPPOL-EN16931-R003"))
        );
    }

    #[test]
    fn order_reference_satisfies_r003() {
        let mut inv = valid_peppol_invoice();
        inv.buyer_reference = None;
        inv.order_reference = Some("PO-123".into());
        let errors = validate_peppol(&inv);
        assert!(
            !errors
                .iter()
                .any(|e| e.rule.as_deref() == Some("PEPPOL-EN16931-R003"))
        );
    }

    #[test]
    fn missing_seller_endpoint() {
        let mut inv = valid_peppol_invoice();
        inv.seller.electronic_address = None;
        let errors = validate_peppol(&inv);
        assert!(
            errors
                .iter()
                .any(|e| e.rule.as_deref() == Some("PEPPOL-EN16931-R020"))
        );
    }

    #[test]
    fn missing_buyer_endpoint() {
        let mut inv = valid_peppol_invoice();
        inv.buyer.electronic_address = None;
        let errors = validate_peppol(&inv);
        assert!(
            errors
                .iter()
                .any(|e| e.rule.as_deref() == Some("PEPPOL-EN16931-R010"))
        );
    }

    #[test]
    fn partial_invoice_non_german_rejected() {
        let mut inv = valid_peppol_invoice();
        inv.type_code = InvoiceTypeCode::Partial;
        inv.buyer.address.country_code = "FR".into();
        let errors = validate_peppol(&inv);
        assert!(
            errors
                .iter()
                .any(|e| e.rule.as_deref() == Some("PEPPOL-EN16931-P0112"))
        );
    }

    #[test]
    fn partial_invoice_both_german_ok() {
        let mut inv = valid_peppol_invoice();
        inv.type_code = InvoiceTypeCode::Partial;
        let errors = validate_peppol(&inv);
        assert!(
            !errors
                .iter()
                .any(|e| e.rule.as_deref() == Some("PEPPOL-EN16931-P0112"))
        );
    }
}
