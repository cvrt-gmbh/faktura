//! SKR03 / SKR04 account mappings.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::core::{TaxCategory, VatScenario};

/// Standard German chart of accounts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ChartOfAccounts {
    /// Standardkontenrahmen 03 (most common for SMBs).
    SKR03,
    /// Standardkontenrahmen 04 (used by larger companies, Bilanzrecht).
    SKR04,
}

impl ChartOfAccounts {
    /// SKR identifier for the EXTF header.
    pub fn code(&self) -> &'static str {
        match self {
            Self::SKR03 => "03",
            Self::SKR04 => "04",
        }
    }
}

/// Account mapping for a specific booking.
#[derive(Debug, Clone)]
pub struct AccountMapping {
    /// Revenue / expense account (Erlöskonto / Aufwandskonto).
    pub revenue_account: u32,
    /// Whether the revenue account is an Automatikkonto
    /// (auto-applies tax, so BU-Schlüssel can be omitted).
    pub is_automatik: bool,
}

/// Determine the revenue account for a given tax scenario, category, and rate.
pub fn revenue_account(
    chart: ChartOfAccounts,
    _scenario: VatScenario,
    category: TaxCategory,
    rate: Decimal,
) -> AccountMapping {
    match chart {
        ChartOfAccounts::SKR03 => skr03_revenue(category, rate),
        ChartOfAccounts::SKR04 => skr04_revenue(category, rate),
    }
}

fn skr03_revenue(category: TaxCategory, rate: Decimal) -> AccountMapping {
    match category {
        TaxCategory::StandardRate => {
            if rate == dec!(19) {
                AccountMapping {
                    revenue_account: 8400,
                    is_automatik: true,
                }
            } else if rate == dec!(7) {
                AccountMapping {
                    revenue_account: 8300,
                    is_automatik: true,
                }
            } else {
                // Non-standard rate — use generic revenue, needs BU key
                AccountMapping {
                    revenue_account: 8000,
                    is_automatik: false,
                }
            }
        }
        TaxCategory::ZeroRated | TaxCategory::Exempt | TaxCategory::NotSubjectToVat => {
            AccountMapping {
                revenue_account: 8200,
                is_automatik: false,
            }
        }
        TaxCategory::ReverseCharge => {
            // §13b — revenue account for reverse charge
            AccountMapping {
                revenue_account: 8337,
                is_automatik: false,
            }
        }
        TaxCategory::IntraCommunitySupply => {
            // Steuerfreie innergem. Lieferung §4 Nr. 1b
            AccountMapping {
                revenue_account: 8125,
                is_automatik: true,
            }
        }
        TaxCategory::Export => {
            // Steuerfreie Ausfuhrlieferung §4 Nr. 1a
            AccountMapping {
                revenue_account: 8120,
                is_automatik: true,
            }
        }
    }
}

fn skr04_revenue(category: TaxCategory, rate: Decimal) -> AccountMapping {
    match category {
        TaxCategory::StandardRate => {
            if rate == dec!(19) {
                AccountMapping {
                    revenue_account: 4400,
                    is_automatik: true,
                }
            } else if rate == dec!(7) {
                AccountMapping {
                    revenue_account: 4300,
                    is_automatik: true,
                }
            } else {
                AccountMapping {
                    revenue_account: 4000,
                    is_automatik: false,
                }
            }
        }
        TaxCategory::ZeroRated | TaxCategory::Exempt | TaxCategory::NotSubjectToVat => {
            AccountMapping {
                revenue_account: 4200,
                is_automatik: false,
            }
        }
        TaxCategory::ReverseCharge => AccountMapping {
            revenue_account: 4337,
            is_automatik: false,
        },
        TaxCategory::IntraCommunitySupply => AccountMapping {
            revenue_account: 4125,
            is_automatik: true,
        },
        TaxCategory::Export => AccountMapping {
            revenue_account: 4120,
            is_automatik: true,
        },
    }
}

/// Named account entry for SKR lookup by German name.
#[derive(Debug, Clone)]
pub struct NamedAccount {
    /// Account number.
    pub number: u32,
    /// German account name (e.g. "Erlöse 19% USt").
    pub name: &'static str,
    /// Whether this is an Automatikkonto.
    pub is_automatik: bool,
}

/// Common SKR03 revenue/expense accounts.
const SKR03_ACCOUNTS: &[NamedAccount] = &[
    NamedAccount {
        number: 8400,
        name: "Erlöse 19% USt",
        is_automatik: true,
    },
    NamedAccount {
        number: 8300,
        name: "Erlöse 7% USt",
        is_automatik: true,
    },
    NamedAccount {
        number: 8200,
        name: "Erlöse steuerfrei",
        is_automatik: false,
    },
    NamedAccount {
        number: 8000,
        name: "Erlöse",
        is_automatik: false,
    },
    NamedAccount {
        number: 8120,
        name: "Steuerfreie Ausfuhrlieferungen",
        is_automatik: true,
    },
    NamedAccount {
        number: 8125,
        name: "Steuerfreie innergem. Lieferungen §4 Nr. 1b",
        is_automatik: true,
    },
    NamedAccount {
        number: 8337,
        name: "Erlöse §13b UStG",
        is_automatik: false,
    },
    NamedAccount {
        number: 8150,
        name: "Sonstige steuerfreie Umsätze",
        is_automatik: false,
    },
    NamedAccount {
        number: 8190,
        name: "Erlöse Kleinunternehmer §19",
        is_automatik: false,
    },
    NamedAccount {
        number: 8500,
        name: "Provisionserlöse",
        is_automatik: true,
    },
    NamedAccount {
        number: 8700,
        name: "Erlöse aus Vermietung",
        is_automatik: true,
    },
    NamedAccount {
        number: 8800,
        name: "Erlöse Anlageverkäufe 19%",
        is_automatik: true,
    },
    NamedAccount {
        number: 4400,
        name: "Betriebsbedarf",
        is_automatik: false,
    },
    NamedAccount {
        number: 4600,
        name: "Werbekosten",
        is_automatik: false,
    },
    NamedAccount {
        number: 4900,
        name: "Sonstige betriebliche Aufwendungen",
        is_automatik: false,
    },
    NamedAccount {
        number: 4500,
        name: "Fahrzeugkosten",
        is_automatik: false,
    },
    NamedAccount {
        number: 4210,
        name: "Miete",
        is_automatik: false,
    },
    NamedAccount {
        number: 4830,
        name: "Reisekosten Arbeitnehmer",
        is_automatik: false,
    },
    NamedAccount {
        number: 4120,
        name: "Gehälter",
        is_automatik: false,
    },
    NamedAccount {
        number: 4100,
        name: "Löhne",
        is_automatik: false,
    },
];

/// Common SKR04 revenue/expense accounts.
const SKR04_ACCOUNTS: &[NamedAccount] = &[
    NamedAccount {
        number: 4400,
        name: "Erlöse 19% USt",
        is_automatik: true,
    },
    NamedAccount {
        number: 4300,
        name: "Erlöse 7% USt",
        is_automatik: true,
    },
    NamedAccount {
        number: 4200,
        name: "Erlöse steuerfrei",
        is_automatik: false,
    },
    NamedAccount {
        number: 4000,
        name: "Erlöse",
        is_automatik: false,
    },
    NamedAccount {
        number: 4120,
        name: "Steuerfreie Ausfuhrlieferungen",
        is_automatik: true,
    },
    NamedAccount {
        number: 4125,
        name: "Steuerfreie innergem. Lieferungen §4 Nr. 1b",
        is_automatik: true,
    },
    NamedAccount {
        number: 4337,
        name: "Erlöse §13b UStG",
        is_automatik: false,
    },
    NamedAccount {
        number: 4150,
        name: "Sonstige steuerfreie Umsätze",
        is_automatik: false,
    },
    NamedAccount {
        number: 4190,
        name: "Erlöse Kleinunternehmer §19",
        is_automatik: false,
    },
    NamedAccount {
        number: 4500,
        name: "Provisionserlöse",
        is_automatik: true,
    },
    NamedAccount {
        number: 4700,
        name: "Erlöse aus Vermietung",
        is_automatik: true,
    },
    NamedAccount {
        number: 4800,
        name: "Erlöse Anlageverkäufe 19%",
        is_automatik: true,
    },
    NamedAccount {
        number: 6300,
        name: "Betriebsbedarf",
        is_automatik: false,
    },
    NamedAccount {
        number: 6600,
        name: "Werbekosten",
        is_automatik: false,
    },
    NamedAccount {
        number: 6800,
        name: "Sonstige betriebliche Aufwendungen",
        is_automatik: false,
    },
    NamedAccount {
        number: 6500,
        name: "Fahrzeugkosten",
        is_automatik: false,
    },
    NamedAccount {
        number: 6310,
        name: "Miete",
        is_automatik: false,
    },
    NamedAccount {
        number: 6650,
        name: "Reisekosten Arbeitnehmer",
        is_automatik: false,
    },
    NamedAccount {
        number: 6020,
        name: "Gehälter",
        is_automatik: false,
    },
    NamedAccount {
        number: 6000,
        name: "Löhne",
        is_automatik: false,
    },
];

/// Look up an account by German name (case-insensitive substring match).
///
/// Returns all accounts whose name contains the search string.
///
/// # Example
///
/// ```
/// use faktura::datev::{ChartOfAccounts, account_by_name};
///
/// let results = account_by_name(ChartOfAccounts::SKR03, "Erlöse 19%");
/// assert_eq!(results[0].number, 8400);
/// ```
pub fn account_by_name(chart: ChartOfAccounts, search: &str) -> Vec<&'static NamedAccount> {
    let search_lower = search.to_lowercase();
    let accounts = match chart {
        ChartOfAccounts::SKR03 => SKR03_ACCOUNTS,
        ChartOfAccounts::SKR04 => SKR04_ACCOUNTS,
    };
    accounts
        .iter()
        .filter(|a| a.name.to_lowercase().contains(&search_lower))
        .collect()
}

/// Look up an account by number.
pub fn account_by_number(chart: ChartOfAccounts, number: u32) -> Option<&'static NamedAccount> {
    let accounts = match chart {
        ChartOfAccounts::SKR03 => SKR03_ACCOUNTS,
        ChartOfAccounts::SKR04 => SKR04_ACCOUNTS,
    };
    accounts.iter().find(|a| a.number == number)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skr03_standard_19() {
        let m = revenue_account(
            ChartOfAccounts::SKR03,
            VatScenario::Domestic,
            TaxCategory::StandardRate,
            dec!(19),
        );
        assert_eq!(m.revenue_account, 8400);
        assert!(m.is_automatik);
    }

    #[test]
    fn skr04_standard_19() {
        let m = revenue_account(
            ChartOfAccounts::SKR04,
            VatScenario::Domestic,
            TaxCategory::StandardRate,
            dec!(19),
        );
        assert_eq!(m.revenue_account, 4400);
        assert!(m.is_automatik);
    }

    #[test]
    fn skr03_reduced_7() {
        let m = revenue_account(
            ChartOfAccounts::SKR03,
            VatScenario::Domestic,
            TaxCategory::StandardRate,
            dec!(7),
        );
        assert_eq!(m.revenue_account, 8300);
        assert!(m.is_automatik);
    }

    #[test]
    fn skr03_export() {
        let m = revenue_account(
            ChartOfAccounts::SKR03,
            VatScenario::Export,
            TaxCategory::Export,
            dec!(0),
        );
        assert_eq!(m.revenue_account, 8120);
        assert!(m.is_automatik);
    }

    #[test]
    fn lookup_by_name_exact() {
        let results = account_by_name(ChartOfAccounts::SKR03, "Erlöse 19% USt");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].number, 8400);
    }

    #[test]
    fn lookup_by_name_partial() {
        let results = account_by_name(ChartOfAccounts::SKR03, "Erlöse");
        assert!(results.len() >= 5); // multiple Erlöse accounts
    }

    #[test]
    fn lookup_by_name_case_insensitive() {
        let results = account_by_name(ChartOfAccounts::SKR03, "erlöse 19%");
        assert_eq!(results[0].number, 8400);
    }

    #[test]
    fn lookup_by_name_skr04() {
        let results = account_by_name(ChartOfAccounts::SKR04, "Erlöse 19% USt");
        assert_eq!(results[0].number, 4400);
    }

    #[test]
    fn lookup_by_name_no_match() {
        let results = account_by_name(ChartOfAccounts::SKR03, "nonexistent");
        assert!(results.is_empty());
    }

    #[test]
    fn lookup_by_number() {
        let acc = account_by_number(ChartOfAccounts::SKR03, 8400).unwrap();
        assert_eq!(acc.name, "Erlöse 19% USt");
        assert!(acc.is_automatik);
    }

    #[test]
    fn lookup_by_number_not_found() {
        assert!(account_by_number(ChartOfAccounts::SKR03, 9999).is_none());
    }
}
