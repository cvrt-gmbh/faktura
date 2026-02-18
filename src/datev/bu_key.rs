//! BU-Schlüssel (Buchungsschlüssel / tax posting keys) for DATEV.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::core::TaxCategory;

/// DATEV BU-Schlüssel (tax posting key).
///
/// Used in field 9 of the Buchungsstapel to indicate the tax treatment.
/// When using Automatikkonten, the BU key can be omitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuSchluessel(pub u8);

impl BuSchluessel {
    /// USt 19% (output tax, standard rate).
    pub const UST_19: Self = Self(3);
    /// USt 7% (output tax, reduced rate).
    pub const UST_7: Self = Self(2);
    /// VSt 19% (input tax, standard rate).
    pub const VST_19: Self = Self(9);
    /// VSt 7% (input tax, reduced rate).
    pub const VST_7: Self = Self(8);
    /// Tax-free intra-community delivery.
    pub const EU_DELIVERY: Self = Self(10);
    /// Intra-community acquisition 19%.
    pub const EU_ACQUISITION_19: Self = Self(12);
    /// Intra-community acquisition 7%.
    pub const EU_ACQUISITION_7: Self = Self(13);
    /// Reverse charge §13b 19%.
    pub const REVERSE_CHARGE_19: Self = Self(44);
}

/// Determine the BU-Schlüssel for an output tax (sales) posting.
///
/// Returns `None` if the posting uses an Automatikkonto
/// and no explicit BU key is needed.
pub fn bu_schluessel(category: TaxCategory, rate: Decimal) -> Option<BuSchluessel> {
    match category {
        TaxCategory::StandardRate => {
            if rate == dec!(19) {
                Some(BuSchluessel::UST_19)
            } else if rate == dec!(7) {
                Some(BuSchluessel::UST_7)
            } else {
                // Non-standard rate — caller must handle manually
                None
            }
        }
        TaxCategory::IntraCommunitySupply => Some(BuSchluessel::EU_DELIVERY),
        TaxCategory::ReverseCharge => Some(BuSchluessel::REVERSE_CHARGE_19),
        // Tax-free / exempt / export: no BU key needed
        TaxCategory::ZeroRated
        | TaxCategory::Exempt
        | TaxCategory::Export
        | TaxCategory::NotSubjectToVat => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_19_returns_bu3() {
        assert_eq!(
            bu_schluessel(TaxCategory::StandardRate, dec!(19)),
            Some(BuSchluessel::UST_19)
        );
        assert_eq!(BuSchluessel::UST_19.0, 3);
    }

    #[test]
    fn standard_7_returns_bu2() {
        assert_eq!(
            bu_schluessel(TaxCategory::StandardRate, dec!(7)),
            Some(BuSchluessel::UST_7)
        );
        assert_eq!(BuSchluessel::UST_7.0, 2);
    }

    #[test]
    fn exempt_returns_none() {
        assert_eq!(bu_schluessel(TaxCategory::Exempt, dec!(0)), None);
    }

    #[test]
    fn eu_delivery_returns_bu10() {
        assert_eq!(
            bu_schluessel(TaxCategory::IntraCommunitySupply, dec!(0)),
            Some(BuSchluessel::EU_DELIVERY)
        );
    }

    #[test]
    fn reverse_charge_returns_bu44() {
        assert_eq!(
            bu_schluessel(TaxCategory::ReverseCharge, dec!(19)),
            Some(BuSchluessel::REVERSE_CHARGE_19)
        );
    }
}
