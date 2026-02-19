use chrono::NaiveDate;
use rust_decimal::Decimal;

use super::error::RechnungError;
use super::types::*;
use super::validation;

/// Builder for constructing valid invoices.
///
/// ```
/// use faktura::core::*;
/// use rust_decimal_macros::dec;
/// use chrono::NaiveDate;
///
/// let invoice = InvoiceBuilder::new("RE-2024-001", NaiveDate::from_ymd_opt(2024, 1, 15).unwrap())
///     .tax_point_date(NaiveDate::from_ymd_opt(2024, 1, 15).unwrap())
///     .seller(PartyBuilder::new("ACME GmbH", AddressBuilder::new("Berlin", "10115", "DE").build())
///         .vat_id("DE123456789")
///         .build())
///     .buyer(PartyBuilder::new("Kunde AG", AddressBuilder::new("München", "80331", "DE").build())
///         .build())
///     .add_line(LineItemBuilder::new("1", "Beratung", dec!(10), "HUR", dec!(150.00))
///         .tax(TaxCategory::StandardRate, dec!(19))
///         .build())
///     .build();
/// ```
pub struct InvoiceBuilder {
    number: String,
    issue_date: NaiveDate,
    due_date: Option<NaiveDate>,
    type_code: InvoiceTypeCode,
    currency_code: String,
    tax_currency_code: Option<String>,
    vat_total_in_tax_currency: Option<Decimal>,
    notes: Vec<String>,
    buyer_reference: Option<String>,
    project_reference: Option<String>,
    contract_reference: Option<String>,
    order_reference: Option<String>,
    sales_order_reference: Option<String>,
    buyer_accounting_reference: Option<String>,
    seller: Option<Party>,
    buyer: Option<Party>,
    lines: Vec<LineItem>,
    vat_scenario: VatScenario,
    allowances: Vec<AllowanceCharge>,
    charges: Vec<AllowanceCharge>,
    payment_terms: Option<String>,
    payment: Option<PaymentInstructions>,
    tax_point_date: Option<NaiveDate>,
    invoicing_period: Option<Period>,
    delivery: Option<DeliveryInformation>,
    prepaid: Decimal,
    preceding_invoices: Vec<PrecedingInvoiceReference>,
    attachments: Vec<DocumentAttachment>,
    payee: Option<Payee>,
    tax_representative: Option<TaxRepresentative>,
}

impl InvoiceBuilder {
    /// Create a new invoice builder with the required invoice number and issue date.
    pub fn new(number: impl Into<String>, issue_date: NaiveDate) -> Self {
        Self {
            number: number.into(),
            issue_date,
            due_date: None,
            type_code: InvoiceTypeCode::Invoice,
            currency_code: "EUR".to_string(),
            tax_currency_code: None,
            vat_total_in_tax_currency: None,
            notes: Vec::new(),
            buyer_reference: None,
            project_reference: None,
            contract_reference: None,
            order_reference: None,
            sales_order_reference: None,
            buyer_accounting_reference: None,
            seller: None,
            buyer: None,
            lines: Vec::new(),
            vat_scenario: VatScenario::Domestic,
            allowances: Vec::new(),
            charges: Vec::new(),
            payment_terms: None,
            payment: None,
            tax_point_date: None,
            invoicing_period: None,
            delivery: None,
            prepaid: Decimal::ZERO,
            preceding_invoices: Vec::new(),
            attachments: Vec::new(),
            payee: None,
            tax_representative: None,
        }
    }

    /// Set the payment due date (BT-9).
    pub fn due_date(mut self, date: NaiveDate) -> Self {
        self.due_date = Some(date);
        self
    }

    /// Set the invoice type code (BT-3). Defaults to `Invoice` (380).
    /// Use `CreditNote` (381) for credit notes.
    pub fn type_code(mut self, code: InvoiceTypeCode) -> Self {
        self.type_code = code;
        self
    }

    /// Set the currency code (BT-5). Defaults to `"EUR"`.
    /// Must be a valid ISO 4217 code.
    pub fn currency(mut self, code: impl Into<String>) -> Self {
        self.currency_code = code.into();
        self
    }

    /// Add a free-text note to the invoice (BT-22).
    pub fn note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    /// Set the buyer reference (BT-10). Required for XRechnung (Leitweg-ID).
    pub fn buyer_reference(mut self, reference: impl Into<String>) -> Self {
        self.buyer_reference = Some(reference.into());
        self
    }

    /// Set the project reference (BT-11).
    pub fn project_reference(mut self, reference: impl Into<String>) -> Self {
        self.project_reference = Some(reference.into());
        self
    }

    /// Set the contract reference (BT-12).
    pub fn contract_reference(mut self, reference: impl Into<String>) -> Self {
        self.contract_reference = Some(reference.into());
        self
    }

    /// Set the purchase order reference (BT-13).
    pub fn order_reference(mut self, reference: impl Into<String>) -> Self {
        self.order_reference = Some(reference.into());
        self
    }

    /// Set the sales order reference (BT-14).
    pub fn sales_order_reference(mut self, reference: impl Into<String>) -> Self {
        self.sales_order_reference = Some(reference.into());
        self
    }

    /// Set the buyer accounting reference (BT-19).
    pub fn buyer_accounting_reference(mut self, reference: impl Into<String>) -> Self {
        self.buyer_accounting_reference = Some(reference.into());
        self
    }

    /// Set the seller party (BG-4). Required.
    pub fn seller(mut self, party: Party) -> Self {
        self.seller = Some(party);
        self
    }

    /// Set the buyer party (BG-7). Required.
    pub fn buyer(mut self, party: Party) -> Self {
        self.buyer = Some(party);
        self
    }

    /// Add a line item (BG-25). At least one is required.
    pub fn add_line(mut self, line: LineItem) -> Self {
        self.lines.push(line);
        self
    }

    /// Set the VAT scenario. Defaults to `Domestic`.
    /// This affects DATEV account mapping and validation rules.
    pub fn vat_scenario(mut self, scenario: VatScenario) -> Self {
        self.vat_scenario = scenario;
        self
    }

    /// Add a document-level allowance (BG-20).
    pub fn add_allowance(mut self, allowance: AllowanceCharge) -> Self {
        self.allowances.push(AllowanceCharge {
            is_charge: false,
            ..allowance
        });
        self
    }

    /// Add a document-level charge (BG-21).
    pub fn add_charge(mut self, charge: AllowanceCharge) -> Self {
        self.charges.push(AllowanceCharge {
            is_charge: true,
            ..charge
        });
        self
    }

    /// Set the payment terms description (BT-20), e.g. "Net 30 days".
    pub fn payment_terms(mut self, terms: impl Into<String>) -> Self {
        self.payment_terms = Some(terms.into());
        self
    }

    /// Set the payment instructions (BG-16) including means code and bank details.
    pub fn payment(mut self, payment: PaymentInstructions) -> Self {
        self.payment = Some(payment);
        self
    }

    /// Set the delivery date / tax point date (BT-7, §14 Abs. 4 Nr. 6 UStG).
    /// Either this or `invoicing_period` is required (except for Kleinbetragsrechnungen).
    pub fn tax_point_date(mut self, date: NaiveDate) -> Self {
        self.tax_point_date = Some(date);
        self
    }

    /// Set the invoicing period (BG-14) as an alternative to `tax_point_date`.
    /// Satisfies the §14 Abs. 4 Nr. 6 UStG delivery date requirement.
    pub fn invoicing_period(mut self, start: NaiveDate, end: NaiveDate) -> Self {
        self.invoicing_period = Some(Period { start, end });
        self
    }

    /// Set the prepaid amount to deduct from the grand total.
    pub fn prepaid(mut self, amount: Decimal) -> Self {
        self.prepaid = amount;
        self
    }

    /// Add a preceding invoice reference (BG-3, BT-25/BT-26).
    /// Used for credit notes and corrected invoices.
    pub fn add_preceding_invoice(
        mut self,
        number: impl Into<String>,
        issue_date: Option<NaiveDate>,
    ) -> Self {
        self.preceding_invoices.push(PrecedingInvoiceReference {
            number: number.into(),
            issue_date,
        });
        self
    }

    /// Set the tax currency code (BT-6) and VAT total in that currency (BT-111).
    /// Use when VAT must be reported in a currency different from the document currency.
    pub fn tax_currency(mut self, code: impl Into<String>, tax_total: Decimal) -> Self {
        self.tax_currency_code = Some(code.into());
        self.vat_total_in_tax_currency = Some(tax_total);
        self
    }

    /// Add a document attachment (BG-24). Maximum 100 attachments.
    pub fn add_attachment(mut self, attachment: DocumentAttachment) -> Self {
        self.attachments.push(attachment);
        self
    }

    /// Set delivery information (BG-13).
    pub fn delivery(mut self, delivery: DeliveryInformation) -> Self {
        self.delivery = Some(delivery);
        self
    }

    /// Set the payee party (BG-10), when different from the seller.
    pub fn payee(mut self, payee: Payee) -> Self {
        self.payee = Some(payee);
        self
    }

    /// Set the seller tax representative party (BG-11).
    pub fn tax_representative(mut self, tax_rep: TaxRepresentative) -> Self {
        self.tax_representative = Some(tax_rep);
        self
    }

    /// Build the invoice, calculating totals and running §14 UStG validation.
    /// Returns all validation errors (not just the first).
    pub fn build(self) -> Result<Invoice, RechnungError> {
        let invoice = self.build_inner()?;

        let errors = validation::validate_14_ustg(&invoice);
        if !errors.is_empty() {
            return Err(errors_to_validation_error(&errors));
        }

        Ok(invoice)
    }

    /// Build with full EN 16931 validation (§14 UStG + EN 16931 rules).
    ///
    /// Stricter than [`Self::build`] — also checks duplicate line IDs, VAT breakdown
    /// consistency, decimal precision, and other EN 16931 business rules.
    pub fn build_strict(self) -> Result<Invoice, RechnungError> {
        let invoice = self.build_inner()?;

        let mut errors = validation::validate_14_ustg(&invoice);
        errors.extend(validation::validate_en16931(&invoice));
        if !errors.is_empty() {
            return Err(errors_to_validation_error(&errors));
        }

        Ok(invoice)
    }

    /// Build without validation — useful for testing or importing external data.
    pub fn build_unchecked(self) -> Result<Invoice, RechnungError> {
        self.build_inner()
    }

    /// Shared invoice construction logic.
    fn build_inner(self) -> Result<Invoice, RechnungError> {
        let seller = self
            .seller
            .ok_or_else(|| RechnungError::Builder("seller is required".into()))?;
        let buyer = self
            .buyer
            .ok_or_else(|| RechnungError::Builder("buyer is required".into()))?;

        if self.lines.is_empty() {
            return Err(RechnungError::Builder(
                "at least one line item is required".into(),
            ));
        }

        // Input limits to prevent abuse
        if self.lines.len() > 10_000 {
            return Err(RechnungError::Builder(
                "invoice cannot have more than 10,000 line items".into(),
            ));
        }
        if self.number.len() > 200 {
            return Err(RechnungError::Builder(
                "invoice number cannot exceed 200 characters".into(),
            ));
        }
        if self.notes.len() > 100 {
            return Err(RechnungError::Builder(
                "invoice cannot have more than 100 notes".into(),
            ));
        }
        if self.attachments.len() > 100 {
            return Err(RechnungError::Builder(
                "invoice cannot have more than 100 attachments".into(),
            ));
        }

        let vat_total_in_tax_currency = self.vat_total_in_tax_currency;
        let prepaid = self.prepaid;

        let mut invoice = Invoice {
            number: self.number,
            issue_date: self.issue_date,
            due_date: self.due_date,
            type_code: self.type_code,
            currency_code: self.currency_code,
            tax_currency_code: self.tax_currency_code,
            notes: self.notes,
            buyer_reference: self.buyer_reference,
            project_reference: self.project_reference,
            contract_reference: self.contract_reference,
            order_reference: self.order_reference,
            sales_order_reference: self.sales_order_reference,
            buyer_accounting_reference: self.buyer_accounting_reference,
            seller,
            buyer,
            lines: self.lines,
            vat_scenario: self.vat_scenario,
            allowances: self.allowances,
            charges: self.charges,
            totals: None,
            payment_terms: self.payment_terms,
            payment: self.payment,
            tax_point_date: self.tax_point_date,
            invoicing_period: self.invoicing_period,
            payee: self.payee,
            tax_representative: self.tax_representative,
            preceding_invoices: self.preceding_invoices,
            attachments: self.attachments,
            delivery: self.delivery,
        };

        validation::calculate_totals(&mut invoice, prepaid);

        if let (Some(totals), Some(tax_total)) =
            (invoice.totals.as_mut(), vat_total_in_tax_currency)
        {
            totals.vat_total_in_tax_currency = Some(tax_total);
        }

        Ok(invoice)
    }
}

/// Builder for Party (seller/buyer).
pub struct PartyBuilder {
    name: String,
    vat_id: Option<String>,
    tax_number: Option<String>,
    registration_id: Option<String>,
    trading_name: Option<String>,
    address: Address,
    contact: Option<Contact>,
    electronic_address: Option<ElectronicAddress>,
}

impl PartyBuilder {
    /// Create a new party with the required name and postal address.
    pub fn new(name: impl Into<String>, address: Address) -> Self {
        Self {
            name: name.into(),
            vat_id: None,
            tax_number: None,
            registration_id: None,
            trading_name: None,
            address,
            contact: None,
            electronic_address: None,
        }
    }

    /// Set the VAT identification number (BT-31/BT-48), e.g. `"DE123456789"`.
    pub fn vat_id(mut self, id: impl Into<String>) -> Self {
        self.vat_id = Some(id.into());
        self
    }

    /// Set the German tax number / Steuernummer (BT-32).
    pub fn tax_number(mut self, num: impl Into<String>) -> Self {
        self.tax_number = Some(num.into());
        self
    }

    /// Set the legal registration identifier (BT-30/BT-47), e.g. HRB number.
    pub fn registration_id(mut self, id: impl Into<String>) -> Self {
        self.registration_id = Some(id.into());
        self
    }

    /// Set the trading name / business name (BT-28).
    pub fn trading_name(mut self, name: impl Into<String>) -> Self {
        self.trading_name = Some(name.into());
        self
    }

    /// Set the contact person details (BG-6/BG-9).
    pub fn contact(
        mut self,
        name: Option<String>,
        phone: Option<String>,
        email: Option<String>,
    ) -> Self {
        self.contact = Some(Contact { name, phone, email });
        self
    }

    /// Set the electronic address (BT-34/BT-49) with EAS scheme code.
    /// For XRechnung, the seller must have one (typically Leitweg-ID "0204" or email "EM").
    pub fn electronic_address(
        mut self,
        scheme: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        self.electronic_address = Some(ElectronicAddress {
            scheme: scheme.into(),
            value: value.into(),
        });
        self
    }

    pub fn build(self) -> Party {
        Party {
            name: self.name,
            vat_id: self.vat_id,
            tax_number: self.tax_number,
            registration_id: self.registration_id,
            trading_name: self.trading_name,
            address: self.address,
            contact: self.contact,
            electronic_address: self.electronic_address,
        }
    }
}

/// Builder for Address.
pub struct AddressBuilder {
    street: Option<String>,
    additional: Option<String>,
    city: String,
    postal_code: String,
    country_code: String,
    subdivision: Option<String>,
}

impl AddressBuilder {
    /// Create a new address with city, postal code, and ISO 3166-1 alpha-2 country code.
    pub fn new(
        city: impl Into<String>,
        postal_code: impl Into<String>,
        country_code: impl Into<String>,
    ) -> Self {
        Self {
            street: None,
            additional: None,
            city: city.into(),
            postal_code: postal_code.into(),
            country_code: country_code.into(),
            subdivision: None,
        }
    }

    /// Set the street address line (BT-35).
    pub fn street(mut self, street: impl Into<String>) -> Self {
        self.street = Some(street.into());
        self
    }

    /// Set the additional address line (BT-36), e.g. building or floor.
    pub fn additional(mut self, additional: impl Into<String>) -> Self {
        self.additional = Some(additional.into());
        self
    }

    /// Set the country subdivision (BT-39), e.g. state or Bundesland.
    pub fn subdivision(mut self, subdivision: impl Into<String>) -> Self {
        self.subdivision = Some(subdivision.into());
        self
    }

    pub fn build(self) -> Address {
        Address {
            street: self.street,
            additional: self.additional,
            city: self.city,
            postal_code: self.postal_code,
            country_code: self.country_code,
            subdivision: self.subdivision,
        }
    }
}

/// Builder for LineItem.
pub struct LineItemBuilder {
    id: String,
    item_name: String,
    quantity: Decimal,
    unit: String,
    unit_price: Decimal,
    gross_price: Option<Decimal>,
    allowances: Vec<AllowanceCharge>,
    charges: Vec<AllowanceCharge>,
    tax_category: TaxCategory,
    tax_rate: Decimal,
    description: Option<String>,
    seller_item_id: Option<String>,
    buyer_item_id: Option<String>,
    standard_item_id: Option<String>,
    note: Option<String>,
    base_quantity: Option<Decimal>,
    base_quantity_unit: Option<String>,
    origin_country: Option<String>,
    attributes: Vec<ItemAttribute>,
    invoicing_period: Option<Period>,
}

impl LineItemBuilder {
    /// Create a new line item with ID, name, quantity, unit code, and net unit price.
    ///
    /// `unit` is a UN/ECE Recommendation 20 code: `"HUR"` (hours), `"C62"` (pieces),
    /// `"DAY"` (days), `"MON"` (months), `"KGM"` (kilograms), etc.
    pub fn new(
        id: impl Into<String>,
        item_name: impl Into<String>,
        quantity: Decimal,
        unit: impl Into<String>,
        unit_price: Decimal,
    ) -> Self {
        Self {
            id: id.into(),
            item_name: item_name.into(),
            quantity,
            unit: unit.into(),
            unit_price,
            gross_price: None,
            allowances: Vec::new(),
            charges: Vec::new(),
            tax_category: TaxCategory::StandardRate,
            tax_rate: Decimal::new(19, 0),
            description: None,
            seller_item_id: None,
            buyer_item_id: None,
            standard_item_id: None,
            note: None,
            base_quantity: None,
            base_quantity_unit: None,
            origin_country: None,
            attributes: Vec::new(),
            invoicing_period: None,
        }
    }

    /// Set the tax category and rate for this line.
    /// Defaults to `StandardRate` at 19%. Common rates: 19% (standard), 7% (reduced).
    pub fn tax(mut self, category: TaxCategory, rate: Decimal) -> Self {
        self.tax_category = category;
        self.tax_rate = rate;
        self
    }

    /// Set the gross (list) price before line-level allowances (BT-148).
    pub fn gross_price(mut self, price: Decimal) -> Self {
        self.gross_price = Some(price);
        self
    }

    /// Set the item description (BT-154).
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set the seller's item identifier (BT-155).
    pub fn seller_item_id(mut self, id: impl Into<String>) -> Self {
        self.seller_item_id = Some(id.into());
        self
    }

    /// Set the standard item identifier (BT-157), typically a GTIN/EAN.
    pub fn standard_item_id(mut self, id: impl Into<String>) -> Self {
        self.standard_item_id = Some(id.into());
        self
    }

    /// Set the buyer's item identifier (BT-156).
    pub fn buyer_item_id(mut self, id: impl Into<String>) -> Self {
        self.buyer_item_id = Some(id.into());
        self
    }

    /// Set the invoice line note (BT-127).
    pub fn note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }

    /// Set the base quantity (BT-149) and optionally the base quantity unit (BT-150).
    /// Use when the price refers to a different quantity than 1 (e.g. price per 100 units).
    pub fn base_quantity(mut self, qty: Decimal, unit: Option<String>) -> Self {
        self.base_quantity = Some(qty);
        self.base_quantity_unit = unit;
        self
    }

    /// Set the item country of origin (BT-159), ISO 3166-1 alpha-2 code.
    pub fn origin_country(mut self, code: impl Into<String>) -> Self {
        self.origin_country = Some(code.into());
        self
    }

    /// Add a line-level allowance (BG-27).
    pub fn add_allowance(mut self, allowance: AllowanceCharge) -> Self {
        self.allowances.push(allowance);
        self
    }

    /// Add a line-level charge (BG-28).
    pub fn add_charge(mut self, charge: AllowanceCharge) -> Self {
        self.charges.push(charge);
        self
    }

    /// Add an item attribute (BT-160/BT-161).
    pub fn add_attribute(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.push(ItemAttribute {
            name: name.into(),
            value: value.into(),
        });
        self
    }

    /// Set the line-level invoicing period (BG-26).
    pub fn invoicing_period(mut self, start: NaiveDate, end: NaiveDate) -> Self {
        self.invoicing_period = Some(Period { start, end });
        self
    }

    pub fn build(self) -> LineItem {
        LineItem {
            id: self.id,
            quantity: self.quantity,
            unit: self.unit,
            unit_price: self.unit_price,
            gross_price: self.gross_price,
            allowances: self.allowances,
            charges: self.charges,
            tax_category: self.tax_category,
            tax_rate: self.tax_rate,
            item_name: self.item_name,
            description: self.description,
            seller_item_id: self.seller_item_id,
            buyer_item_id: self.buyer_item_id,
            standard_item_id: self.standard_item_id,
            line_amount: None,
            note: self.note,
            base_quantity: self.base_quantity,
            base_quantity_unit: self.base_quantity_unit,
            origin_country: self.origin_country,
            attributes: self.attributes,
            invoicing_period: self.invoicing_period,
        }
    }
}

fn errors_to_validation_error(errors: &[super::error::ValidationError]) -> RechnungError {
    let msg = errors
        .iter()
        .map(|e| e.to_string())
        .collect::<Vec<_>>()
        .join("; ");
    RechnungError::Validation(msg)
}
