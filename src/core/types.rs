use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// BG-0: Invoice — the top-level document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invoice {
    /// BT-1: Invoice number (unique, gapless within sequence).
    pub number: String,
    /// BT-2: Invoice issue date.
    pub issue_date: NaiveDate,
    /// BT-9: Payment due date.
    pub due_date: Option<NaiveDate>,
    /// BT-3: Invoice type code (UNTDID 1001).
    pub type_code: InvoiceTypeCode,
    /// BT-5: Invoice currency code (ISO 4217, e.g. "EUR").
    pub currency_code: String,
    /// BT-22: Note / free text.
    pub notes: Vec<String>,
    /// BT-10: Buyer reference (Leitweg-ID for XRechnung).
    pub buyer_reference: Option<String>,
    /// BT-13: Purchase order reference.
    pub order_reference: Option<String>,
    /// BG-4: Seller.
    pub seller: Party,
    /// BG-7: Buyer.
    pub buyer: Party,
    /// BG-25: Invoice lines.
    pub lines: Vec<LineItem>,
    /// German VAT scenario determining validation rules.
    pub vat_scenario: VatScenario,
    /// BG-20: Document-level allowances.
    pub allowances: Vec<AllowanceCharge>,
    /// BG-21: Document-level charges.
    pub charges: Vec<AllowanceCharge>,
    /// BG-22: Calculated totals (set by `calculate_totals()`).
    pub totals: Option<Totals>,
    /// BT-20: Payment terms free text.
    pub payment_terms: Option<String>,
    /// BG-16: Payment instructions.
    pub payment: Option<PaymentInstructions>,
    /// BT-8: Tax point date (Leistungsdatum).
    pub tax_point_date: Option<NaiveDate>,
    /// BG-14: Invoicing period.
    pub invoicing_period: Option<Period>,
}

/// BG-4 / BG-7: Party (seller or buyer).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Party {
    /// BT-27 / BT-44: Name.
    pub name: String,
    /// BT-31 / BT-48: VAT identifier (e.g. "DE123456789").
    pub vat_id: Option<String>,
    /// BT-32: Tax registration number (Steuernummer).
    pub tax_number: Option<String>,
    /// BT-30 / BT-47: Legal registration identifier.
    pub registration_id: Option<String>,
    /// BT-29 / BT-46: Trading name.
    pub trading_name: Option<String>,
    /// BG-5 / BG-8: Postal address.
    pub address: Address,
    /// BG-6 / BG-9: Contact information.
    pub contact: Option<Contact>,
    /// BT-34 / BT-49: Electronic address (e.g. email, Peppol ID).
    pub electronic_address: Option<ElectronicAddress>,
}

/// BG-5 / BG-8: Postal address.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Address {
    /// BT-35 / BT-50: Street + house number.
    pub street: Option<String>,
    /// BT-36 / BT-51: Additional address line.
    pub additional: Option<String>,
    /// BT-37 / BT-52: City.
    pub city: String,
    /// BT-38 / BT-53: Postal code.
    pub postal_code: String,
    /// BT-40 / BT-55: Country code (ISO 3166-1 alpha-2).
    pub country_code: String,
    /// BT-39 / BT-54: Country subdivision (Bundesland).
    pub subdivision: Option<String>,
}

/// BG-6 / BG-9: Contact information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    /// BT-41 / BT-56: Contact point name.
    pub name: Option<String>,
    /// BT-42 / BT-57: Telephone.
    pub phone: Option<String>,
    /// BT-43 / BT-58: Email.
    pub email: Option<String>,
}

/// Electronic address with scheme identifier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElectronicAddress {
    /// Scheme identifier (e.g. "EM" for email, "0088" for EAN).
    pub scheme: String,
    /// Address value.
    pub value: String,
}

/// BG-25: Invoice line item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineItem {
    /// BT-126: Line identifier.
    pub id: String,
    /// BT-129: Invoiced quantity.
    pub quantity: Decimal,
    /// BT-130: Unit of measure (UNECE Rec 20, e.g. "C62" for piece, "HUR" for hour).
    pub unit: String,
    /// BT-146: Item net price (per unit).
    pub unit_price: Decimal,
    /// BT-148: Item gross price (before discount, optional).
    pub gross_price: Option<Decimal>,
    /// BG-27: Line allowances.
    pub allowances: Vec<AllowanceCharge>,
    /// BG-28: Line charges.
    pub charges: Vec<AllowanceCharge>,
    /// BT-151: Tax category for this line.
    pub tax_category: TaxCategory,
    /// BT-152: Tax rate percentage for this line.
    pub tax_rate: Decimal,
    /// BT-153: Item name.
    pub item_name: String,
    /// BT-154: Item description.
    pub description: Option<String>,
    /// BT-155: Seller's item identifier.
    pub seller_item_id: Option<String>,
    /// BT-157: Item standard identifier (EAN/GTIN).
    pub standard_item_id: Option<String>,
    /// Calculated line extension amount (quantity * unit_price ± allowances/charges).
    /// Set by `calculate_totals()`.
    pub line_amount: Option<Decimal>,
}

/// UNTDID 5305 — Tax category codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TaxCategory {
    /// S — Standard rate (7% or 19% in Germany).
    StandardRate,
    /// Z — Zero rated.
    ZeroRated,
    /// E — Exempt from tax.
    Exempt,
    /// AE — Reverse charge.
    ReverseCharge,
    /// K — Intra-community supply (innergemeinschaftliche Lieferung).
    IntraCommunitySupply,
    /// G — Export (outside EU).
    Export,
    /// O — Not subject to VAT.
    NotSubjectToVat,
}

impl TaxCategory {
    /// UNTDID 5305 code letter.
    pub fn code(&self) -> &'static str {
        match self {
            Self::StandardRate => "S",
            Self::ZeroRated => "Z",
            Self::Exempt => "E",
            Self::ReverseCharge => "AE",
            Self::IntraCommunitySupply => "K",
            Self::Export => "G",
            Self::NotSubjectToVat => "O",
        }
    }

    /// Parse from UNTDID 5305 code string.
    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "S" => Some(Self::StandardRate),
            "Z" => Some(Self::ZeroRated),
            "E" => Some(Self::Exempt),
            "AE" => Some(Self::ReverseCharge),
            "K" => Some(Self::IntraCommunitySupply),
            "G" => Some(Self::Export),
            "O" => Some(Self::NotSubjectToVat),
            _ => None,
        }
    }
}

/// German-specific VAT scenario — determines which validation rules and
/// required fields apply to the invoice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VatScenario {
    /// Standard domestic invoice with German VAT.
    Domestic,
    /// §19 UStG Kleinunternehmerregelung — no VAT charged.
    Kleinunternehmer,
    /// §13b UStG — reverse charge, buyer pays VAT.
    ReverseCharge,
    /// §4 Nr. 1b UStG — intra-community supply, 0% VAT.
    IntraCommunitySupply,
    /// §4 Nr. 1a UStG — export to non-EU, 0% VAT.
    Export,
    /// §33 UStDV — simplified invoice under €250 (Kleinbetragsrechnung).
    SmallInvoice,
    /// Mixed scenarios (multiple tax categories on one invoice).
    Mixed,
}

/// UNTDID 1001 — Invoice type codes (subset relevant to German invoicing).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InvoiceTypeCode {
    /// 380 — Commercial invoice.
    Invoice,
    /// 381 — Credit note.
    CreditNote,
    /// 384 — Corrected invoice.
    Corrected,
    /// 386 — Prepayment invoice.
    Prepayment,
    /// 326 — Partial invoice.
    Partial,
}

impl InvoiceTypeCode {
    /// UNTDID 1001 numeric code.
    pub fn code(&self) -> u16 {
        match self {
            Self::Invoice => 380,
            Self::CreditNote => 381,
            Self::Corrected => 384,
            Self::Prepayment => 386,
            Self::Partial => 326,
        }
    }

    /// Parse from UNTDID 1001 numeric code.
    pub fn from_code(code: u16) -> Option<Self> {
        match code {
            380 => Some(Self::Invoice),
            381 => Some(Self::CreditNote),
            384 => Some(Self::Corrected),
            386 => Some(Self::Prepayment),
            326 => Some(Self::Partial),
            _ => None,
        }
    }
}

/// Document-level or line-level allowance/charge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllowanceCharge {
    /// True = charge, false = allowance.
    pub is_charge: bool,
    /// Amount.
    pub amount: Decimal,
    /// Percentage (if percentage-based).
    pub percentage: Option<Decimal>,
    /// Base amount for percentage calculation.
    pub base_amount: Option<Decimal>,
    /// Tax category.
    pub tax_category: TaxCategory,
    /// Tax rate.
    pub tax_rate: Decimal,
    /// Reason text.
    pub reason: Option<String>,
    /// Reason code (UNTDID 5189 for allowances, 7161 for charges).
    pub reason_code: Option<String>,
}

/// BG-22: Document totals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Totals {
    /// BT-106: Sum of all line net amounts.
    pub line_net_total: Decimal,
    /// BT-107: Sum of document-level allowances.
    pub allowances_total: Decimal,
    /// BT-108: Sum of document-level charges.
    pub charges_total: Decimal,
    /// BT-109: Invoice total without VAT = line_net_total - allowances + charges.
    pub net_total: Decimal,
    /// BT-110: Total VAT amount.
    pub vat_total: Decimal,
    /// BT-112: Invoice total with VAT = net_total + vat_total.
    pub gross_total: Decimal,
    /// BT-113: Paid amount (prepayments).
    pub prepaid: Decimal,
    /// BT-115: Amount due = gross_total - prepaid.
    pub amount_due: Decimal,
    /// BG-23: VAT breakdown by category.
    pub vat_breakdown: Vec<VatBreakdown>,
}

/// BG-23: VAT breakdown per category/rate combination.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VatBreakdown {
    /// BT-118: Tax category.
    pub category: TaxCategory,
    /// BT-119: Tax rate percentage.
    pub rate: Decimal,
    /// BT-116: Taxable amount (category base).
    pub taxable_amount: Decimal,
    /// BT-117: Tax amount.
    pub tax_amount: Decimal,
    /// BT-120: Exemption reason text.
    pub exemption_reason: Option<String>,
    /// BT-121: Exemption reason code (VATEX).
    pub exemption_reason_code: Option<String>,
}

/// Payment instructions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentInstructions {
    /// BT-81: Payment means type code (UNTDID 4461).
    pub means_code: PaymentMeansCode,
    /// BT-82: Payment means text.
    pub means_text: Option<String>,
    /// BT-83: Remittance information (Verwendungszweck).
    pub remittance_info: Option<String>,
    /// BG-17: Credit transfer (bank account).
    pub credit_transfer: Option<CreditTransfer>,
}

/// BG-17: Credit transfer / bank account.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditTransfer {
    /// BT-84: IBAN.
    pub iban: String,
    /// BT-86: BIC.
    pub bic: Option<String>,
    /// BT-85: Account name.
    pub account_name: Option<String>,
}

/// Common payment means codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaymentMeansCode {
    /// 10 — Cash.
    Cash,
    /// 30 — Credit transfer.
    CreditTransfer,
    /// 42 — Payment to bank account.
    PaymentToBankAccount,
    /// 48 — Bank card.
    BankCard,
    /// 49 — Direct debit.
    DirectDebit,
    /// 57 — Standing agreement.
    StandingAgreement,
    /// 58 — SEPA credit transfer.
    SepaCreditTransfer,
    /// 59 — SEPA direct debit.
    SepaDirectDebit,
    /// Other code value.
    Other(u16),
}

impl PaymentMeansCode {
    pub fn code(&self) -> u16 {
        match self {
            Self::Cash => 10,
            Self::CreditTransfer => 30,
            Self::PaymentToBankAccount => 42,
            Self::BankCard => 48,
            Self::DirectDebit => 49,
            Self::StandingAgreement => 57,
            Self::SepaCreditTransfer => 58,
            Self::SepaDirectDebit => 59,
            Self::Other(c) => *c,
        }
    }

    /// Parse from UNTDID 4461 numeric code.
    pub fn from_code(code: u16) -> Self {
        match code {
            10 => Self::Cash,
            30 => Self::CreditTransfer,
            42 => Self::PaymentToBankAccount,
            48 => Self::BankCard,
            49 => Self::DirectDebit,
            57 => Self::StandingAgreement,
            58 => Self::SepaCreditTransfer,
            59 => Self::SepaDirectDebit,
            c => Self::Other(c),
        }
    }
}

/// Invoicing period.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Period {
    /// BT-73: Start date.
    pub start: NaiveDate,
    /// BT-74: End date.
    pub end: NaiveDate,
}
