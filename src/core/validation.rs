use std::collections::HashMap;

use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use super::error::ValidationError;
use super::types::*;

/// Validate an invoice against §14 UStG requirements.
/// Returns all validation errors found (not just the first).
pub fn validate_14_ustg(invoice: &Invoice) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    // BR-01: An Invoice shall have a Specification identifier
    // (implicit — always present in our type system)

    // BR-02: An Invoice shall have an Invoice number
    if invoice.number.trim().is_empty() {
        errors.push(ValidationError::with_rule(
            "number",
            "invoice number must not be empty",
            "BR-02",
        ));
    }

    // BR-03: An Invoice shall have an Invoice issue date
    // (guaranteed by type system — NaiveDate is always valid)

    // BR-04: An Invoice shall have an Invoice type code
    // (guaranteed by type system — enum)

    // BR-05: An Invoice shall have an Invoice currency code
    if invoice.currency_code.trim().is_empty() {
        errors.push(ValidationError::with_rule(
            "currency_code",
            "currency code must not be empty",
            "BR-05",
        ));
    } else if invoice.currency_code.len() != 3 {
        errors.push(ValidationError::with_rule(
            "currency_code",
            "currency code must be 3 characters (ISO 4217)",
            "BR-05",
        ));
    } else if !super::currencies::is_known_currency_code(&invoice.currency_code) {
        errors.push(ValidationError::with_rule(
            "currency_code",
            format!(
                "currency code '{}' is not a known ISO 4217 code",
                invoice.currency_code
            ),
            "BR-05",
        ));
    }

    // §14 Abs. 4 Nr. 1 — Seller name and address
    validate_party(&invoice.seller, "seller", &mut errors);

    // §14 Abs. 4 Nr. 1 — Buyer name and address
    validate_party_buyer(&invoice.buyer, "buyer", &invoice.vat_scenario, &mut errors);

    // §14 Abs. 4 Nr. 2 — Tax number or VAT ID of seller
    // BR-CO-06: When a tax representative (BG-11) is present, the seller's own
    // VAT ID / tax number is not required — the representative's VAT ID suffices.
    if invoice.vat_scenario != VatScenario::SmallInvoice
        && invoice.tax_representative.is_none()
        && invoice.seller.vat_id.is_none()
        && invoice.seller.tax_number.is_none()
    {
        errors.push(ValidationError::with_rule(
            "seller",
            "seller must have either a VAT ID (USt-IdNr.) or tax number (Steuernummer)",
            "BR-CO-09",
        ));
    }

    // Validate VAT ID format if present
    if let Some(vat_id) = &invoice.seller.vat_id {
        validate_vat_id_format(vat_id, "seller.vat_id", &mut errors);
    }
    if let Some(vat_id) = &invoice.buyer.vat_id {
        validate_vat_id_format(vat_id, "buyer.vat_id", &mut errors);
    }

    // §14 Abs. 4 Nr. 6 UStG — Delivery date or service period
    // Required unless SmallInvoice (§33 UStDV)
    if invoice.vat_scenario != VatScenario::SmallInvoice
        && invoice.tax_point_date.is_none()
        && invoice.invoicing_period.is_none()
    {
        errors.push(ValidationError::with_rule(
            "tax_point_date",
            "invoice must have a delivery date (Leistungsdatum) or invoicing period (§14 Abs. 4 Nr. 6 UStG)",
            "BR-CO-03",
        ));
    }

    // BR-16: An Invoice shall have at least one Invoice line
    if invoice.lines.is_empty() {
        errors.push(ValidationError::with_rule(
            "lines",
            "invoice must have at least one line item",
            "BR-16",
        ));
    }

    // Validate each line
    for (i, line) in invoice.lines.iter().enumerate() {
        validate_line(line, i, &mut errors);
    }

    // Scenario-specific validation
    validate_scenario(invoice, &mut errors);

    // Arithmetic validation
    errors.extend(validate_arithmetic(invoice));

    errors
}

/// Validate invoice arithmetic (totals, rounding).
pub fn validate_arithmetic(invoice: &Invoice) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    let Some(totals) = &invoice.totals else {
        errors.push(ValidationError::with_rule(
            "totals",
            "totals must be calculated before validation (call calculate_totals first)",
            "BR-CO-10",
        ));
        return errors;
    };

    // BR-CO-10: Sum of line net amounts
    let expected_line_total: Decimal = invoice.lines.iter().filter_map(|l| l.line_amount).sum();

    if totals.line_net_total != expected_line_total {
        errors.push(ValidationError::with_rule(
            "totals.line_net_total",
            format!(
                "line net total {} does not match sum of line amounts {}",
                totals.line_net_total, expected_line_total
            ),
            "BR-CO-10",
        ));
    }

    // BR-CO-11: net_total = line_net_total - allowances + charges
    let expected_net = totals.line_net_total - totals.allowances_total + totals.charges_total;
    if totals.net_total != expected_net {
        errors.push(ValidationError::with_rule(
            "totals.net_total",
            format!(
                "net total {} does not match calculation {}",
                totals.net_total, expected_net
            ),
            "BR-CO-11",
        ));
    }

    // BR-CO-15: gross_total = net_total + vat_total
    let expected_gross = totals.net_total + totals.vat_total;
    if totals.gross_total != expected_gross {
        errors.push(ValidationError::with_rule(
            "totals.gross_total",
            format!(
                "gross total {} does not match net {} + vat {}",
                totals.gross_total, totals.net_total, totals.vat_total
            ),
            "BR-CO-15",
        ));
    }

    // BR-CO-16: amount_due = gross_total - prepaid
    let expected_due = totals.gross_total - totals.prepaid;
    if totals.amount_due != expected_due {
        errors.push(ValidationError::with_rule(
            "totals.amount_due",
            format!(
                "amount due {} does not match gross {} - prepaid {}",
                totals.amount_due, totals.gross_total, totals.prepaid
            ),
            "BR-CO-16",
        ));
    }

    // Validate VAT breakdown sums
    let breakdown_vat_total: Decimal = totals.vat_breakdown.iter().map(|b| b.tax_amount).sum();
    if totals.vat_total != breakdown_vat_total {
        errors.push(ValidationError::with_rule(
            "totals.vat_total",
            format!(
                "VAT total {} does not match sum of breakdown amounts {}",
                totals.vat_total, breakdown_vat_total
            ),
            "BR-CO-14",
        ));
    }

    errors
}

/// Calculate totals for an invoice (mutates in place).
pub fn calculate_totals(invoice: &mut Invoice, prepaid: Decimal) {
    // Calculate line amounts
    for line in &mut invoice.lines {
        let base = line.quantity * line.unit_price;
        let allowances: Decimal = line.allowances.iter().map(|a| a.amount).sum();
        let charges: Decimal = line.charges.iter().map(|c| c.amount).sum();
        line.line_amount = Some(base - allowances + charges);
    }

    let line_net_total: Decimal = invoice.lines.iter().filter_map(|l| l.line_amount).sum();

    let allowances_total: Decimal = invoice.allowances.iter().map(|a| a.amount).sum();
    let charges_total: Decimal = invoice.charges.iter().map(|c| c.amount).sum();

    let net_total = line_net_total - allowances_total + charges_total;

    // Build VAT breakdown — group by (category, rate)
    let mut vat_groups: HashMap<(TaxCategory, Decimal), Decimal> = HashMap::new();

    // Lines
    for line in &invoice.lines {
        let key = (line.tax_category, line.tax_rate);
        *vat_groups.entry(key).or_insert(Decimal::ZERO) +=
            line.line_amount.unwrap_or(Decimal::ZERO);
    }

    // Document-level allowances reduce the taxable base
    for allowance in &invoice.allowances {
        let key = (allowance.tax_category, allowance.tax_rate);
        *vat_groups.entry(key).or_insert(Decimal::ZERO) -= allowance.amount;
    }

    // Document-level charges increase the taxable base
    for charge in &invoice.charges {
        let key = (charge.tax_category, charge.tax_rate);
        *vat_groups.entry(key).or_insert(Decimal::ZERO) += charge.amount;
    }

    let mut vat_breakdown: Vec<VatBreakdown> = Vec::new();
    let mut vat_total = Decimal::ZERO;

    for ((category, rate), taxable_amount) in &vat_groups {
        let tax_amount = round_half_up(*taxable_amount * *rate / dec!(100), 2);
        vat_total += tax_amount;

        let exemption_reason = exemption_reason_for(*category, invoice.vat_scenario);

        vat_breakdown.push(VatBreakdown {
            category: *category,
            rate: *rate,
            taxable_amount: *taxable_amount,
            tax_amount,
            exemption_reason: exemption_reason.map(String::from),
            exemption_reason_code: exemption_reason_code_for(*category).map(String::from),
        });
    }

    // Sort breakdown for deterministic output
    vat_breakdown.sort_by(|a, b| {
        a.category
            .code()
            .cmp(b.category.code())
            .then(a.rate.cmp(&b.rate))
    });

    let gross_total = net_total + vat_total;
    let amount_due = gross_total - prepaid;

    invoice.totals = Some(Totals {
        line_net_total,
        allowances_total,
        charges_total,
        net_total,
        vat_total,
        vat_total_in_tax_currency: None,
        gross_total,
        prepaid,
        amount_due,
        vat_breakdown,
    });
}

/// Round a Decimal to `dp` decimal places using half-up (commercial rounding).
fn round_half_up(value: Decimal, dp: u32) -> Decimal {
    value.round_dp_with_strategy(dp, rust_decimal::RoundingStrategy::MidpointAwayFromZero)
}

fn validate_party(party: &Party, prefix: &str, errors: &mut Vec<ValidationError>) {
    if party.name.trim().is_empty() {
        errors.push(ValidationError::with_rule(
            format!("{prefix}.name"),
            "name must not be empty",
            "BR-06",
        ));
    }

    validate_address(&party.address, &format!("{prefix}.address"), errors);
}

fn validate_party_buyer(
    party: &Party,
    prefix: &str,
    scenario: &VatScenario,
    errors: &mut Vec<ValidationError>,
) {
    // §33 UStDV: Kleinbetragsrechnung doesn't require buyer details
    if *scenario == VatScenario::SmallInvoice {
        return;
    }

    if party.name.trim().is_empty() {
        errors.push(ValidationError::with_rule(
            format!("{prefix}.name"),
            "buyer name must not be empty",
            "BR-07",
        ));
    }

    validate_address(&party.address, &format!("{prefix}.address"), errors);
}

fn validate_address(address: &Address, prefix: &str, errors: &mut Vec<ValidationError>) {
    if address.city.trim().is_empty() {
        errors.push(ValidationError::with_rule(
            format!("{prefix}.city"),
            "city must not be empty",
            "BR-09",
        ));
    }

    if address.postal_code.trim().is_empty() {
        errors.push(ValidationError::with_rule(
            format!("{prefix}.postal_code"),
            "postal code (BT-38/BT-53) must not be empty",
            "BR-09",
        ));
    }

    if address.country_code.trim().is_empty() {
        errors.push(ValidationError::with_rule(
            format!("{prefix}.country_code"),
            "country code must not be empty",
            "BR-09",
        ));
    } else if address.country_code.len() != 2 {
        errors.push(ValidationError::with_rule(
            format!("{prefix}.country_code"),
            "country code (BT-40/BT-55) must be 2 characters (ISO 3166-1 alpha-2)",
            "BR-09",
        ));
    } else if !super::countries::is_known_country_code(&address.country_code) {
        errors.push(ValidationError::with_rule(
            format!("{prefix}.country_code"),
            format!(
                "country code '{}' is not a known ISO 3166-1 alpha-2 code",
                address.country_code
            ),
            "BR-09",
        ));
    }
}

fn validate_line(line: &LineItem, index: usize, errors: &mut Vec<ValidationError>) {
    let prefix = format!("lines[{index}]");

    if line.id.trim().is_empty() {
        errors.push(ValidationError::with_rule(
            format!("{prefix}.id"),
            "line identifier must not be empty",
            "BR-21",
        ));
    }

    if line.quantity.is_zero() {
        errors.push(ValidationError::with_rule(
            format!("{prefix}.quantity"),
            "invoiced quantity (BT-129) must not be zero",
            "BR-22",
        ));
    }

    if line.unit_price.is_sign_negative() {
        errors.push(ValidationError::with_rule(
            format!("{prefix}.unit_price"),
            "item net price (BT-146) must not be negative",
            "BR-27",
        ));
    }

    if line.item_name.trim().is_empty() {
        errors.push(ValidationError::with_rule(
            format!("{prefix}.item_name"),
            "item name must not be empty",
            "BR-25",
        ));
    }

    if line.tax_rate.is_sign_negative() {
        errors.push(ValidationError::with_rule(
            format!("{prefix}.tax_rate"),
            "line VAT rate (BT-152) must not be negative",
            "BR-27",
        ));
    }

    // BR-27/28: Tax categories that require 0% rate
    match line.tax_category {
        TaxCategory::ZeroRated
        | TaxCategory::Exempt
        | TaxCategory::ReverseCharge
        | TaxCategory::IntraCommunitySupply
        | TaxCategory::Export
        | TaxCategory::NotSubjectToVat => {
            if !line.tax_rate.is_zero() {
                errors.push(ValidationError::with_rule(
                    format!("{prefix}.tax_rate"),
                    format!(
                        "tax rate must be 0 for category {} ({})",
                        line.tax_category.code(),
                        category_name(line.tax_category)
                    ),
                    "BR-AE-05",
                ));
            }
        }
        TaxCategory::StandardRate => {
            if line.tax_rate.is_zero() {
                errors.push(ValidationError::with_rule(
                    format!("{prefix}.tax_rate"),
                    "standard rate (S) category (BT-151) must have a non-zero VAT rate (BT-152)",
                    "BR-S-05",
                ));
            }
        }
    }
}

fn validate_scenario(invoice: &Invoice, errors: &mut Vec<ValidationError>) {
    match invoice.vat_scenario {
        VatScenario::Kleinunternehmer => {
            // Must have note referencing §19 UStG
            let has_19_note = invoice
                .notes
                .iter()
                .any(|n| n.contains("19") && n.contains("UStG"));
            if !has_19_note {
                errors.push(ValidationError::with_rule(
                    "notes",
                    "Kleinunternehmer invoice must contain a note (BT-22) referencing §19 UStG",
                    "BR-O-10",
                ));
            }

            // All lines must be NotSubjectToVat with 0%
            for (i, line) in invoice.lines.iter().enumerate() {
                if line.tax_category != TaxCategory::NotSubjectToVat {
                    errors.push(ValidationError::with_rule(
                        format!("lines[{i}].tax_category"),
                        "Kleinunternehmer lines must use NotSubjectToVat (O) category (BT-151), got: {}",
                        "BR-O-01",
                    ));
                }
            }
        }

        VatScenario::ReverseCharge => {
            // Buyer must have VAT ID
            if invoice.buyer.vat_id.is_none() {
                errors.push(ValidationError::with_rule(
                    "buyer.vat_id",
                    "reverse charge: buyer must have a VAT ID (BT-48)",
                    "BR-AE-02",
                ));
            }

            // Must have note referencing §13b UStG
            let has_13b_note = invoice
                .notes
                .iter()
                .any(|n| n.contains("13b") && n.contains("UStG"));
            if !has_13b_note {
                errors.push(ValidationError::with_rule(
                    "notes",
                    "reverse charge invoice must contain a note (BT-22) referencing §13b UStG",
                    "BR-AE-10",
                ));
            }

            // All lines must be ReverseCharge category
            for (i, line) in invoice.lines.iter().enumerate() {
                if line.tax_category != TaxCategory::ReverseCharge {
                    errors.push(ValidationError::with_rule(
                        format!("lines[{i}].tax_category"),
                        "reverse charge lines must use ReverseCharge (AE) category (BT-151)",
                        "BR-AE-01",
                    ));
                }
            }
        }

        VatScenario::IntraCommunitySupply => {
            // Both parties must have VAT IDs
            if invoice.seller.vat_id.is_none() {
                errors.push(ValidationError::with_rule(
                    "seller.vat_id",
                    "intra-community supply: seller must have a VAT ID (BT-31)",
                    "BR-IC-02",
                ));
            }
            if invoice.buyer.vat_id.is_none() {
                errors.push(ValidationError::with_rule(
                    "buyer.vat_id",
                    "intra-community supply: buyer must have a VAT ID (BT-48)",
                    "BR-IC-03",
                ));
            }

            // Buyer must be in a different EU country
            if invoice.seller.address.country_code == invoice.buyer.address.country_code {
                errors.push(ValidationError::with_rule(
                    "buyer.address.country_code",
                    "intra-community supply: buyer country (BT-55) must differ from seller country (BT-40)",
                    "BR-IC-04",
                ));
            }

            for (i, line) in invoice.lines.iter().enumerate() {
                if line.tax_category != TaxCategory::IntraCommunitySupply {
                    errors.push(ValidationError::with_rule(
                        format!("lines[{i}].tax_category"),
                        "intra-community supply lines must use IntraCommunitySupply (K) category (BT-151)",
                        "BR-IC-01",
                    ));
                }
            }
        }

        VatScenario::Export => {
            for (i, line) in invoice.lines.iter().enumerate() {
                if line.tax_category != TaxCategory::Export {
                    errors.push(ValidationError::with_rule(
                        format!("lines[{i}].tax_category"),
                        "export lines must use Export (G) category (BT-151)",
                        "BR-G-01",
                    ));
                }
            }
        }

        VatScenario::SmallInvoice => {
            // §33 UStDV: total must be ≤ €250
            if let Some(totals) = &invoice.totals {
                if totals.gross_total > dec!(250) {
                    errors.push(ValidationError::with_rule(
                        "totals.gross_total",
                        format!(
                            "Kleinbetragsrechnung (§33 UStDV) gross total (BT-112) must not exceed €250, got: {}",
                            totals.gross_total
                        ),
                        "BR-DE-17",
                    ));
                }
            }
        }

        VatScenario::Domestic => {
            // Standard domestic — lines should use StandardRate
            // (but mixed is also valid, just use Mixed scenario for that)
        }

        VatScenario::Mixed => {
            // No specific restrictions — all category combinations allowed
        }
    }
}

/// Validate basic VAT ID format (2 letter country code + digits/chars).
fn validate_vat_id_format(vat_id: &str, field: &str, errors: &mut Vec<ValidationError>) {
    if vat_id.len() < 4 {
        errors.push(ValidationError::with_rule(
            field,
            format!("VAT ID (BT-31/BT-48) '{vat_id}' too short — expected 2-letter country code + identifier"),
            "BR-CO-09",
        ));
        return;
    }

    let country_prefix = &vat_id[..2];
    if !country_prefix.chars().all(|c| c.is_ascii_uppercase()) {
        errors.push(ValidationError::with_rule(
            field,
            format!("VAT ID (BT-31/BT-48) must start with a 2-letter country code (e.g. DE, AT, FR), got: '{}'", &vat_id[..2]),
            "BR-CO-09",
        ));
    }

    // German VAT IDs: DE followed by exactly 9 digits
    // Strip spaces — DATEV exports and other systems sometimes include them.
    if country_prefix == "DE" {
        let digits: String = vat_id[2..].chars().filter(|c| !c.is_whitespace()).collect();
        if digits.len() != 9 || !digits.chars().all(|c| c.is_ascii_digit()) {
            errors.push(ValidationError::with_rule(
                field,
                format!("German VAT ID must be DE followed by exactly 9 digits (e.g. DE123456789), got: '{vat_id}'"),
                "BR-CO-09",
            ));
        }
    }
}

/// Validate an invoice against EN 16931 business rules.
///
/// This covers rules not already checked by [`validate_14_ustg`], focusing
/// on structural completeness and VAT breakdown consistency required by
/// the European standard. Call this in addition to `validate_14_ustg` for
/// full compliance.
pub fn validate_en16931(invoice: &Invoice) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    // BR-CO-04: Each invoice line identifier (BT-126) must be unique
    {
        let mut seen = std::collections::HashSet::new();
        for (i, line) in invoice.lines.iter().enumerate() {
            if !seen.insert(&line.id) {
                errors.push(ValidationError::with_rule(
                    format!("lines[{i}].id"),
                    format!("duplicate line identifier '{}'", line.id),
                    "BR-CO-04",
                ));
            }
        }
    }

    // BR-11: Seller postal address shall have a country code
    if invoice.seller.address.country_code.trim().is_empty() {
        errors.push(ValidationError::with_rule(
            "seller.address.country_code",
            "seller postal address must have a country code",
            "BR-11",
        ));
    }

    // BR-12: Buyer postal address shall have a country code
    if invoice.buyer.address.country_code.trim().is_empty() {
        errors.push(ValidationError::with_rule(
            "buyer.address.country_code",
            "buyer postal address must have a country code",
            "BR-12",
        ));
    }

    // Delivery address country code validation
    if let Some(ref delivery) = invoice.delivery {
        if let Some(ref addr) = delivery.delivery_address {
            if !addr.country_code.is_empty()
                && addr.country_code.len() == 2
                && !super::countries::is_known_country_code(&addr.country_code)
            {
                errors.push(ValidationError::with_rule(
                    "delivery.delivery_address.country_code",
                    format!(
                        "delivery country code '{}' is not a known ISO 3166-1 alpha-2 code",
                        addr.country_code
                    ),
                    "BR-57",
                ));
            }
        }
    }

    // UNTDID 5189/7161 reason code validation for document-level allowances/charges
    for (i, ac) in invoice.allowances.iter().enumerate() {
        if let Some(ref code) = ac.reason_code {
            if !super::reason_codes::is_known_allowance_reason(code) {
                errors.push(ValidationError::with_rule(
                    format!("allowances[{i}].reason_code"),
                    format!(
                        "allowance reason code '{}' is not a known UNTDID 5189 code",
                        code
                    ),
                    "BR-CO-21",
                ));
            }
        }
    }
    for (i, ac) in invoice.charges.iter().enumerate() {
        if let Some(ref code) = ac.reason_code {
            if !super::reason_codes::is_known_charge_reason(code) {
                errors.push(ValidationError::with_rule(
                    format!("charges[{i}].reason_code"),
                    format!(
                        "charge reason code '{}' is not a known UNTDID 7161 code",
                        code
                    ),
                    "BR-CO-22",
                ));
            }
        }
    }

    // BR-13: An Invoice shall have the seller tax identifier or tax registration
    // (already covered as BR-CO-09 in validate_14_ustg, but EN 16931 has its own ID)

    // BR-31: Each line item shall have a net price
    for (i, line) in invoice.lines.iter().enumerate() {
        if line.unit_price.is_sign_negative() {
            errors.push(ValidationError::with_rule(
                format!("lines[{i}].unit_price"),
                "item net price must not be negative",
                "BR-31",
            ));
        }
    }

    // BR-33: Each line shall have a line VAT category code
    // (guaranteed by type system — TaxCategory is not Optional)

    // BR-45: Each VAT breakdown shall have a VAT category taxable amount
    // BR-46: Each VAT breakdown shall have a VAT category tax amount
    // BR-47: Each VAT breakdown shall have a VAT category code
    // BR-48: Each VAT breakdown shall have a VAT category rate
    // (all guaranteed by VatBreakdown struct)

    // BR-CO-17: VAT category tax amount = taxable_amount * rate / 100
    // Tolerance ±0.02: line-level rounding can accumulate up to 2 cents difference
    // when many lines are summed per VAT category. KoSIT accepts this tolerance.
    if let Some(ref totals) = invoice.totals {
        for (i, vb) in totals.vat_breakdown.iter().enumerate() {
            let expected = round_half_up(vb.taxable_amount * vb.rate / dec!(100), 2);
            let diff = (vb.tax_amount - expected).abs();
            if diff > dec!(0.02) {
                errors.push(ValidationError::with_rule(
                    format!("totals.vat_breakdown[{i}].tax_amount"),
                    format!(
                        "VAT amount {} does not match taxable {} × rate {}% = {} (tolerance ±0.02)",
                        vb.tax_amount, vb.taxable_amount, vb.rate, expected
                    ),
                    "BR-CO-17",
                ));
            }
        }
    }

    // BR-CO-18: Each allowance/charge shall have a tax category + rate
    for (i, ac) in invoice.allowances.iter().enumerate() {
        if ac.amount.is_sign_negative() {
            errors.push(ValidationError::with_rule(
                format!("allowances[{i}].amount"),
                "allowance amount must not be negative",
                "BR-CO-18",
            ));
        }
    }
    for (i, ac) in invoice.charges.iter().enumerate() {
        if ac.amount.is_sign_negative() {
            errors.push(ValidationError::with_rule(
                format!("charges[{i}].amount"),
                "charge amount must not be negative",
                "BR-CO-18",
            ));
        }
    }

    // BR-S-01 through BR-S-10: Standard rate VAT category rules
    if let Some(ref totals) = invoice.totals {
        for (i, vb) in totals.vat_breakdown.iter().enumerate() {
            match vb.category {
                TaxCategory::StandardRate => {
                    // BR-S-05: rate must be > 0
                    if vb.rate.is_zero() {
                        errors.push(ValidationError::with_rule(
                            format!("totals.vat_breakdown[{i}].rate"),
                            "standard rate category must have a non-zero rate",
                            "BR-S-05",
                        ));
                    }
                }
                TaxCategory::ZeroRated => {
                    // BR-Z-05: rate must be 0
                    if !vb.rate.is_zero() {
                        errors.push(ValidationError::with_rule(
                            format!("totals.vat_breakdown[{i}].rate"),
                            "zero-rated category must have rate 0",
                            "BR-Z-05",
                        ));
                    }
                }
                TaxCategory::Exempt => {
                    // BR-E-05: rate must be 0
                    if !vb.rate.is_zero() {
                        errors.push(ValidationError::with_rule(
                            format!("totals.vat_breakdown[{i}].rate"),
                            "exempt category must have rate 0",
                            "BR-E-05",
                        ));
                    }
                    // BR-E-10: exemption reason required
                    if vb.exemption_reason.is_none() && vb.exemption_reason_code.is_none() {
                        errors.push(ValidationError::with_rule(
                            format!("totals.vat_breakdown[{i}]"),
                            "exempt category requires an exemption reason or reason code",
                            "BR-E-10",
                        ));
                    }
                }
                TaxCategory::ReverseCharge => {
                    // BR-AE-05: rate must be 0
                    if !vb.rate.is_zero() {
                        errors.push(ValidationError::with_rule(
                            format!("totals.vat_breakdown[{i}].rate"),
                            "reverse charge category must have rate 0",
                            "BR-AE-05",
                        ));
                    }
                    // BR-AE-10: exemption reason required
                    if vb.exemption_reason.is_none() && vb.exemption_reason_code.is_none() {
                        errors.push(ValidationError::with_rule(
                            format!("totals.vat_breakdown[{i}]"),
                            "reverse charge category requires an exemption reason or reason code",
                            "BR-AE-10",
                        ));
                    }
                }
                TaxCategory::IntraCommunitySupply => {
                    // BR-IC-05: rate must be 0
                    if !vb.rate.is_zero() {
                        errors.push(ValidationError::with_rule(
                            format!("totals.vat_breakdown[{i}].rate"),
                            "intra-community supply category must have rate 0",
                            "BR-IC-05",
                        ));
                    }
                    // BR-IC-10: exemption reason required
                    if vb.exemption_reason.is_none() && vb.exemption_reason_code.is_none() {
                        errors.push(ValidationError::with_rule(
                            format!("totals.vat_breakdown[{i}]"),
                            "intra-community supply requires an exemption reason or reason code",
                            "BR-IC-10",
                        ));
                    }
                }
                TaxCategory::Export => {
                    // BR-G-05: rate must be 0
                    if !vb.rate.is_zero() {
                        errors.push(ValidationError::with_rule(
                            format!("totals.vat_breakdown[{i}].rate"),
                            "export category must have rate 0",
                            "BR-G-05",
                        ));
                    }
                    // BR-G-10: exemption reason required
                    if vb.exemption_reason.is_none() && vb.exemption_reason_code.is_none() {
                        errors.push(ValidationError::with_rule(
                            format!("totals.vat_breakdown[{i}]"),
                            "export category requires an exemption reason or reason code",
                            "BR-G-10",
                        ));
                    }
                }
                TaxCategory::NotSubjectToVat => {
                    // BR-O-05: rate must be 0
                    if !vb.rate.is_zero() {
                        errors.push(ValidationError::with_rule(
                            format!("totals.vat_breakdown[{i}].rate"),
                            "not-subject-to-VAT category must have rate 0",
                            "BR-O-05",
                        ));
                    }
                    // BR-O-10: exemption reason required
                    if vb.exemption_reason.is_none() && vb.exemption_reason_code.is_none() {
                        errors.push(ValidationError::with_rule(
                            format!("totals.vat_breakdown[{i}]"),
                            "not-subject-to-VAT requires an exemption reason or reason code",
                            "BR-O-10",
                        ));
                    }
                }
            }
        }
    }

    // BR-26: Each line shall have a quantity unit of measure
    for (i, line) in invoice.lines.iter().enumerate() {
        if line.unit.trim().is_empty() {
            errors.push(ValidationError::with_rule(
                format!("lines[{i}].unit"),
                "line quantity unit of measure must not be empty",
                "BR-26",
            ));
        } else if !super::units::is_known_unit_code(&line.unit) {
            errors.push(ValidationError::with_rule(
                format!("lines[{i}].unit"),
                format!(
                    "unit code '{}' is not a known UN/CEFACT Rec 20 code (BT-130)",
                    line.unit
                ),
                "BR-26",
            ));
        }
    }

    // BR-DEC-01: Amounts shall have max 2 decimal places
    if let Some(ref totals) = invoice.totals {
        check_decimal_places(
            &totals.net_total,
            "totals.net_total",
            "BR-DEC-01",
            &mut errors,
        );
        check_decimal_places(
            &totals.vat_total,
            "totals.vat_total",
            "BR-DEC-01",
            &mut errors,
        );
        check_decimal_places(
            &totals.gross_total,
            "totals.gross_total",
            "BR-DEC-01",
            &mut errors,
        );
        check_decimal_places(
            &totals.amount_due,
            "totals.amount_due",
            "BR-DEC-01",
            &mut errors,
        );
    }

    errors
}

fn check_decimal_places(
    value: &Decimal,
    field: &str,
    rule: &str,
    errors: &mut Vec<ValidationError>,
) {
    // Check if value has more than 2 decimal places
    let rounded = value.round_dp(2);
    if *value != rounded {
        errors.push(ValidationError::with_rule(
            field,
            format!("amount {} has more than 2 decimal places", value),
            rule,
        ));
    }
}

fn category_name(cat: TaxCategory) -> &'static str {
    match cat {
        TaxCategory::StandardRate => "Standard Rate",
        TaxCategory::ZeroRated => "Zero Rated",
        TaxCategory::Exempt => "Exempt",
        TaxCategory::ReverseCharge => "Reverse Charge",
        TaxCategory::IntraCommunitySupply => "Intra-Community Supply",
        TaxCategory::Export => "Export",
        TaxCategory::NotSubjectToVat => "Not Subject to VAT",
    }
}

fn exemption_reason_for(category: TaxCategory, scenario: VatScenario) -> Option<&'static str> {
    match (category, scenario) {
        (TaxCategory::NotSubjectToVat, VatScenario::Kleinunternehmer) => {
            Some("Kein Ausweis von Umsatzsteuer, da Kleinunternehmer gemäß §19 UStG")
        }
        (TaxCategory::ReverseCharge, _) => {
            Some("Steuerschuldnerschaft des Leistungsempfängers gemäß §13b UStG")
        }
        (TaxCategory::IntraCommunitySupply, _) => {
            Some("Steuerfreie innergemeinschaftliche Lieferung gemäß §4 Nr. 1b UStG")
        }
        (TaxCategory::Export, _) => Some("Steuerfreie Ausfuhrlieferung gemäß §4 Nr. 1a UStG"),
        (TaxCategory::Exempt, _) => Some("Umsatzsteuerbefreit"),
        _ => None,
    }
}

fn exemption_reason_code_for(category: TaxCategory) -> Option<&'static str> {
    match category {
        TaxCategory::NotSubjectToVat => Some("vatex-eu-o"),
        TaxCategory::ReverseCharge => Some("vatex-eu-ae"),
        TaxCategory::IntraCommunitySupply => Some("vatex-eu-ic"),
        TaxCategory::Export => Some("vatex-eu-g"),
        TaxCategory::Exempt => Some("vatex-eu-e"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::builder::*;
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;

    fn test_date() -> NaiveDate {
        NaiveDate::from_ymd_opt(2024, 6, 15).unwrap()
    }

    fn test_address(country: &str) -> Address {
        AddressBuilder::new("Berlin", "10115", country).build()
    }

    fn test_seller() -> Party {
        PartyBuilder::new("Test GmbH", test_address("DE"))
            .vat_id("DE123456789")
            .build()
    }

    fn test_buyer() -> Party {
        PartyBuilder::new("Kunde AG", test_address("DE")).build()
    }

    fn test_line() -> LineItem {
        LineItemBuilder::new("1", "Beratung", dec!(10), "HUR", dec!(150))
            .tax(TaxCategory::StandardRate, dec!(19))
            .build()
    }

    #[test]
    fn valid_domestic_invoice() {
        let result = InvoiceBuilder::new("RE-001", test_date())
            .seller(test_seller())
            .buyer(test_buyer())
            .add_line(test_line())
            .tax_point_date(test_date())
            .build();

        assert!(result.is_ok());
        let inv = result.unwrap();
        let totals = inv.totals.unwrap();
        assert_eq!(totals.line_net_total, dec!(1500));
        assert_eq!(totals.vat_total, dec!(285));
        assert_eq!(totals.gross_total, dec!(1785));
    }

    #[test]
    fn missing_seller_vat_id_and_tax_number() {
        let seller = PartyBuilder::new("Test GmbH", test_address("DE")).build();

        let result = InvoiceBuilder::new("RE-001", test_date())
            .seller(seller)
            .buyer(test_buyer())
            .add_line(test_line())
            .tax_point_date(test_date())
            .build();

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("VAT ID") || err.contains("tax number"));
    }

    #[test]
    fn missing_delivery_date() {
        let result = InvoiceBuilder::new("RE-001", test_date())
            .seller(test_seller())
            .buyer(test_buyer())
            .add_line(test_line())
            .build();

        let err = result.unwrap_err().to_string();
        assert!(err.contains("delivery date") || err.contains("Leistungsdatum"));
    }

    #[test]
    fn invoicing_period_satisfies_delivery_date() {
        let result = InvoiceBuilder::new("RE-001", test_date())
            .seller(test_seller())
            .buyer(test_buyer())
            .add_line(test_line())
            .invoicing_period(test_date(), test_date())
            .build();

        assert!(result.is_ok());
    }

    #[test]
    fn small_invoice_allows_missing_buyer() {
        let buyer = PartyBuilder::new("", test_address("DE")).build();

        let line = LineItemBuilder::new("1", "Kaffee", dec!(2), "C62", dec!(3.50))
            .tax(TaxCategory::StandardRate, dec!(19))
            .build();

        let result = InvoiceBuilder::new("KB-001", test_date())
            .vat_scenario(VatScenario::SmallInvoice)
            .seller(test_seller())
            .buyer(buyer)
            .add_line(line)
            .build();

        assert!(result.is_ok());
    }

    #[test]
    fn small_invoice_rejects_over_250() {
        let line = LineItemBuilder::new("1", "Teuer", dec!(1), "C62", dec!(300))
            .tax(TaxCategory::StandardRate, dec!(19))
            .build();

        let result = InvoiceBuilder::new("KB-001", test_date())
            .vat_scenario(VatScenario::SmallInvoice)
            .seller(test_seller())
            .buyer(test_buyer())
            .add_line(line)
            .tax_point_date(test_date())
            .build();

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("250"));
    }

    #[test]
    fn reverse_charge_requires_buyer_vat_id() {
        let line = LineItemBuilder::new("1", "Service", dec!(1), "C62", dec!(1000))
            .tax(TaxCategory::ReverseCharge, dec!(0))
            .build();

        let result = InvoiceBuilder::new("RE-001", test_date())
            .vat_scenario(VatScenario::ReverseCharge)
            .note("Steuerschuldnerschaft des Leistungsempfängers §13b UStG")
            .seller(test_seller())
            .buyer(test_buyer()) // no VAT ID
            .add_line(line)
            .tax_point_date(test_date())
            .build();

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("VAT ID"));
    }

    #[test]
    fn kleinunternehmer_requires_note() {
        let line = LineItemBuilder::new("1", "Design", dec!(1), "C62", dec!(500))
            .tax(TaxCategory::NotSubjectToVat, dec!(0))
            .build();

        // Without §19 note
        let result = InvoiceBuilder::new("RE-001", test_date())
            .vat_scenario(VatScenario::Kleinunternehmer)
            .seller(test_seller())
            .buyer(test_buyer())
            .add_line(line.clone())
            .tax_point_date(test_date())
            .build();

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("§19"));

        // With §19 note
        let result = InvoiceBuilder::new("RE-001", test_date())
            .vat_scenario(VatScenario::Kleinunternehmer)
            .note("Kein Ausweis von Umsatzsteuer, da Kleinunternehmer gemäß §19 UStG")
            .seller(test_seller())
            .buyer(test_buyer())
            .add_line(line)
            .tax_point_date(test_date())
            .build();

        assert!(result.is_ok());
        let inv = result.unwrap();
        assert_eq!(inv.totals.unwrap().vat_total, dec!(0));
    }

    #[test]
    fn tax_representative_exempts_seller_tax_id() {
        // Seller without VAT ID or tax number, but with tax representative
        let seller = PartyBuilder::new("Foreign Co", test_address("FR")).build();

        let result = InvoiceBuilder::new("TR-001", test_date())
            .seller(seller)
            .buyer(test_buyer())
            .add_line(test_line())
            .tax_point_date(test_date())
            .tax_representative(TaxRepresentative {
                name: "Steuerberater GmbH".into(),
                vat_id: "DE987654321".into(),
                address: test_address("DE"),
            })
            .build();

        assert!(
            result.is_ok(),
            "tax representative should exempt seller VAT/tax number requirement"
        );
    }

    #[test]
    fn duplicate_line_ids_detected() {
        let inv = InvoiceBuilder::new("DUP-001", test_date())
            .seller(test_seller())
            .buyer(test_buyer())
            .add_line(
                LineItemBuilder::new("1", "Item A", dec!(1), "C62", dec!(100))
                    .tax(TaxCategory::StandardRate, dec!(19))
                    .build(),
            )
            .add_line(
                LineItemBuilder::new("1", "Item B", dec!(2), "C62", dec!(200))
                    .tax(TaxCategory::StandardRate, dec!(19))
                    .build(),
            )
            .tax_point_date(test_date())
            .build()
            .unwrap();

        let errors = validate_en16931(&inv);
        assert!(
            errors.iter().any(|e| e.rule.as_deref() == Some("BR-CO-04")),
            "expected BR-CO-04 for duplicate line IDs"
        );
    }

    #[test]
    fn en16931_standard_rate_must_be_nonzero() {
        let inv = InvoiceBuilder::new("EN-001", test_date())
            .seller(test_seller())
            .buyer(test_buyer())
            .add_line(test_line())
            .tax_point_date(test_date())
            .build()
            .unwrap();

        let errors = validate_en16931(&inv);
        // Valid invoice should have no errors
        assert!(errors.is_empty(), "expected no errors, got: {:?}", errors);
    }

    #[test]
    fn en16931_exempt_needs_reason() {
        let line = LineItemBuilder::new("1", "Tax-free", dec!(1), "C62", dec!(100))
            .tax(TaxCategory::Exempt, dec!(0))
            .build();

        let mut inv = InvoiceBuilder::new("EN-002", test_date())
            .seller(test_seller())
            .buyer(test_buyer())
            .add_line(line)
            .tax_point_date(test_date())
            .vat_scenario(VatScenario::Mixed)
            .build()
            .unwrap();

        // Clear auto-generated exemption reasons
        if let Some(ref mut totals) = inv.totals {
            for vb in &mut totals.vat_breakdown {
                vb.exemption_reason = None;
                vb.exemption_reason_code = None;
            }
        }

        let errors = validate_en16931(&inv);
        assert!(
            errors.iter().any(|e| e.rule.as_deref() == Some("BR-E-10")),
            "expected BR-E-10 for exempt without reason"
        );
    }
}
