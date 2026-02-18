//! Auto-detect VAT scenario from invoice parties and tax categories.

use crate::core::{Invoice, TaxCategory, VatScenario};
use rust_decimal::Decimal;

/// EU member state country codes (ISO 3166-1 alpha-2).
const EU_COUNTRIES: &[&str] = &[
    "AT", "BE", "BG", "CY", "CZ", "DE", "DK", "EE", "ES", "FI", "FR", "GR", "HR", "HU", "IE", "IT",
    "LT", "LU", "LV", "MT", "NL", "PL", "PT", "RO", "SE", "SI", "SK",
];

fn is_eu(country: &str) -> bool {
    EU_COUNTRIES.contains(&country.to_uppercase().as_str())
}

/// Determine the VAT scenario for an invoice based on seller/buyer
/// country codes, VAT IDs, tax categories, and gross total.
///
/// This is a best-effort heuristic. The caller can always override
/// the result by setting `invoice.vat_scenario` manually.
///
/// # Logic
///
/// 1. If gross total ≤ €250 → `SmallInvoice`
/// 2. If any line uses `ReverseCharge` category → `ReverseCharge`
/// 3. If seller is EU, buyer is non-EU → `Export`
/// 4. If seller and buyer are in different EU countries and buyer has VAT ID
///    → `IntraCommunitySupply`
/// 5. If all lines use `Exempt` or `NotSubjectToVat` → `Kleinunternehmer`
/// 6. If lines have mixed standard/reduced rates → `Mixed`
/// 7. Otherwise → `Domestic`
pub fn determine_scenario(invoice: &Invoice) -> VatScenario {
    let seller_country = invoice.seller.address.country_code.to_uppercase();
    let buyer_country = invoice.buyer.address.country_code.to_uppercase();
    let seller_eu = is_eu(&seller_country);
    let buyer_eu = is_eu(&buyer_country);

    // Small invoice check (§33 UStDV)
    if let Some(ref totals) = invoice.totals {
        if totals.gross_total <= Decimal::from(250) && totals.gross_total > Decimal::ZERO {
            return VatScenario::SmallInvoice;
        }
    }

    // Collect tax categories from all lines
    let categories: Vec<TaxCategory> = invoice.lines.iter().map(|l| l.tax_category).collect();

    // Reverse charge (§13b UStG)
    if categories.contains(&TaxCategory::ReverseCharge) {
        return VatScenario::ReverseCharge;
    }

    // Export (§4 Nr. 1a UStG) — seller EU, buyer non-EU
    if seller_eu && !buyer_eu {
        return VatScenario::Export;
    }

    // Intra-community supply (§4 Nr. 1b UStG) — different EU countries, buyer has VAT ID
    if seller_eu && buyer_eu && seller_country != buyer_country && invoice.buyer.vat_id.is_some() {
        return VatScenario::IntraCommunitySupply;
    }

    // Kleinunternehmer (§19 UStG) — all lines exempt/not-subject
    if !categories.is_empty()
        && categories
            .iter()
            .all(|c| matches!(c, TaxCategory::Exempt | TaxCategory::NotSubjectToVat))
    {
        return VatScenario::Kleinunternehmer;
    }

    // Mixed rates — more than one distinct tax rate across lines
    let distinct_rates: std::collections::HashSet<String> = invoice
        .lines
        .iter()
        .map(|l| l.tax_rate.to_string())
        .collect();
    if distinct_rates.len() > 1 {
        return VatScenario::Mixed;
    }

    VatScenario::Domestic
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::*;
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn domestic_invoice() -> Invoice {
        InvoiceBuilder::new("TEST-001", date(2024, 6, 15))
            .tax_point_date(date(2024, 6, 15))
            .seller(
                PartyBuilder::new(
                    "Seller GmbH",
                    AddressBuilder::new("Berlin", "10115", "DE").build(),
                )
                .vat_id("DE123456789")
                .build(),
            )
            .buyer(
                PartyBuilder::new(
                    "Buyer AG",
                    AddressBuilder::new("München", "80331", "DE").build(),
                )
                .build(),
            )
            .add_line(
                LineItemBuilder::new("1", "Service", dec!(10), "HUR", dec!(100))
                    .tax(TaxCategory::StandardRate, dec!(19))
                    .build(),
            )
            .build()
            .unwrap()
    }

    #[test]
    fn domestic_detected() {
        let inv = domestic_invoice();
        assert_eq!(determine_scenario(&inv), VatScenario::Domestic);
    }

    #[test]
    fn export_detected() {
        let mut inv = domestic_invoice();
        inv.buyer.address.country_code = "US".into();
        assert_eq!(determine_scenario(&inv), VatScenario::Export);
    }

    #[test]
    fn intra_community_with_vat_id() {
        let mut inv = domestic_invoice();
        inv.buyer.address.country_code = "FR".into();
        inv.buyer.vat_id = Some("FR12345678901".into());
        assert_eq!(determine_scenario(&inv), VatScenario::IntraCommunitySupply);
    }

    #[test]
    fn intra_community_without_vat_id_falls_to_domestic() {
        let mut inv = domestic_invoice();
        inv.buyer.address.country_code = "FR".into();
        inv.buyer.vat_id = None;
        // Without VAT ID, can't be intra-community — treated as domestic
        assert_eq!(determine_scenario(&inv), VatScenario::Domestic);
    }

    #[test]
    fn reverse_charge_detected() {
        let mut inv = domestic_invoice();
        inv.lines[0].tax_category = TaxCategory::ReverseCharge;
        assert_eq!(determine_scenario(&inv), VatScenario::ReverseCharge);
    }

    #[test]
    fn kleinunternehmer_detected() {
        let mut inv = domestic_invoice();
        inv.lines[0].tax_category = TaxCategory::Exempt;
        inv.lines[0].tax_rate = dec!(0);
        assert_eq!(determine_scenario(&inv), VatScenario::Kleinunternehmer);
    }

    #[test]
    fn small_invoice_detected() {
        let inv = InvoiceBuilder::new("SMALL-001", date(2024, 6, 15))
            .tax_point_date(date(2024, 6, 15))
            .seller(
                PartyBuilder::new(
                    "Seller",
                    AddressBuilder::new("Berlin", "10115", "DE").build(),
                )
                .vat_id("DE123456789")
                .build(),
            )
            .buyer(
                PartyBuilder::new(
                    "Buyer",
                    AddressBuilder::new("Berlin", "10115", "DE").build(),
                )
                .build(),
            )
            .add_line(
                LineItemBuilder::new("1", "Coffee", dec!(2), "C62", dec!(4.50))
                    .tax(TaxCategory::StandardRate, dec!(19))
                    .build(),
            )
            .build()
            .unwrap();
        assert_eq!(determine_scenario(&inv), VatScenario::SmallInvoice);
    }

    #[test]
    fn mixed_rates_detected() {
        let inv = InvoiceBuilder::new("MIX-001", date(2024, 6, 15))
            .tax_point_date(date(2024, 6, 15))
            .seller(
                PartyBuilder::new(
                    "Seller",
                    AddressBuilder::new("Berlin", "10115", "DE").build(),
                )
                .vat_id("DE123456789")
                .build(),
            )
            .buyer(
                PartyBuilder::new(
                    "Buyer",
                    AddressBuilder::new("Berlin", "10115", "DE").build(),
                )
                .build(),
            )
            .add_line(
                LineItemBuilder::new("1", "Service", dec!(10), "HUR", dec!(100))
                    .tax(TaxCategory::StandardRate, dec!(19))
                    .build(),
            )
            .add_line(
                LineItemBuilder::new("2", "Food", dec!(5), "C62", dec!(10))
                    .tax(TaxCategory::StandardRate, dec!(7))
                    .build(),
            )
            .build()
            .unwrap();
        assert_eq!(determine_scenario(&inv), VatScenario::Mixed);
    }
}
