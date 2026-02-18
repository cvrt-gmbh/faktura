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
    notes: Vec<String>,
    buyer_reference: Option<String>,
    order_reference: Option<String>,
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
    prepaid: Decimal,
}

impl InvoiceBuilder {
    pub fn new(number: impl Into<String>, issue_date: NaiveDate) -> Self {
        Self {
            number: number.into(),
            issue_date,
            due_date: None,
            type_code: InvoiceTypeCode::Invoice,
            currency_code: "EUR".to_string(),
            notes: Vec::new(),
            buyer_reference: None,
            order_reference: None,
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
            prepaid: Decimal::ZERO,
        }
    }

    pub fn due_date(mut self, date: NaiveDate) -> Self {
        self.due_date = Some(date);
        self
    }

    pub fn type_code(mut self, code: InvoiceTypeCode) -> Self {
        self.type_code = code;
        self
    }

    pub fn currency(mut self, code: impl Into<String>) -> Self {
        self.currency_code = code.into();
        self
    }

    pub fn note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub fn buyer_reference(mut self, reference: impl Into<String>) -> Self {
        self.buyer_reference = Some(reference.into());
        self
    }

    pub fn order_reference(mut self, reference: impl Into<String>) -> Self {
        self.order_reference = Some(reference.into());
        self
    }

    pub fn seller(mut self, party: Party) -> Self {
        self.seller = Some(party);
        self
    }

    pub fn buyer(mut self, party: Party) -> Self {
        self.buyer = Some(party);
        self
    }

    pub fn add_line(mut self, line: LineItem) -> Self {
        self.lines.push(line);
        self
    }

    pub fn vat_scenario(mut self, scenario: VatScenario) -> Self {
        self.vat_scenario = scenario;
        self
    }

    pub fn add_allowance(mut self, allowance: AllowanceCharge) -> Self {
        self.allowances.push(AllowanceCharge {
            is_charge: false,
            ..allowance
        });
        self
    }

    pub fn add_charge(mut self, charge: AllowanceCharge) -> Self {
        self.charges.push(AllowanceCharge {
            is_charge: true,
            ..charge
        });
        self
    }

    pub fn payment_terms(mut self, terms: impl Into<String>) -> Self {
        self.payment_terms = Some(terms.into());
        self
    }

    pub fn payment(mut self, payment: PaymentInstructions) -> Self {
        self.payment = Some(payment);
        self
    }

    pub fn tax_point_date(mut self, date: NaiveDate) -> Self {
        self.tax_point_date = Some(date);
        self
    }

    pub fn invoicing_period(mut self, start: NaiveDate, end: NaiveDate) -> Self {
        self.invoicing_period = Some(Period { start, end });
        self
    }

    pub fn prepaid(mut self, amount: Decimal) -> Self {
        self.prepaid = amount;
        self
    }

    /// Build the invoice, calculating totals and running validation.
    /// Returns all validation errors (not just the first).
    pub fn build(self) -> Result<Invoice, RechnungError> {
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

        let mut invoice = Invoice {
            number: self.number,
            issue_date: self.issue_date,
            due_date: self.due_date,
            type_code: self.type_code,
            currency_code: self.currency_code,
            notes: self.notes,
            buyer_reference: self.buyer_reference,
            order_reference: self.order_reference,
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
        };

        // Calculate totals
        validation::calculate_totals(&mut invoice, self.prepaid);

        // Run §14 UStG validation
        let errors = validation::validate_14_ustg(&invoice);
        if !errors.is_empty() {
            let msg = errors
                .iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join("; ");
            return Err(RechnungError::Validation(msg));
        }

        Ok(invoice)
    }

    /// Build without validation — useful for testing or importing external data.
    pub fn build_unchecked(self) -> Result<Invoice, RechnungError> {
        let seller = self
            .seller
            .ok_or_else(|| RechnungError::Builder("seller is required".into()))?;
        let buyer = self
            .buyer
            .ok_or_else(|| RechnungError::Builder("buyer is required".into()))?;

        let mut invoice = Invoice {
            number: self.number,
            issue_date: self.issue_date,
            due_date: self.due_date,
            type_code: self.type_code,
            currency_code: self.currency_code,
            notes: self.notes,
            buyer_reference: self.buyer_reference,
            order_reference: self.order_reference,
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
        };

        validation::calculate_totals(&mut invoice, self.prepaid);
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

    pub fn vat_id(mut self, id: impl Into<String>) -> Self {
        self.vat_id = Some(id.into());
        self
    }

    pub fn tax_number(mut self, num: impl Into<String>) -> Self {
        self.tax_number = Some(num.into());
        self
    }

    pub fn registration_id(mut self, id: impl Into<String>) -> Self {
        self.registration_id = Some(id.into());
        self
    }

    pub fn trading_name(mut self, name: impl Into<String>) -> Self {
        self.trading_name = Some(name.into());
        self
    }

    pub fn contact(
        mut self,
        name: Option<String>,
        phone: Option<String>,
        email: Option<String>,
    ) -> Self {
        self.contact = Some(Contact { name, phone, email });
        self
    }

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

    pub fn street(mut self, street: impl Into<String>) -> Self {
        self.street = Some(street.into());
        self
    }

    pub fn additional(mut self, additional: impl Into<String>) -> Self {
        self.additional = Some(additional.into());
        self
    }

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
    standard_item_id: Option<String>,
}

impl LineItemBuilder {
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
            standard_item_id: None,
        }
    }

    pub fn tax(mut self, category: TaxCategory, rate: Decimal) -> Self {
        self.tax_category = category;
        self.tax_rate = rate;
        self
    }

    pub fn gross_price(mut self, price: Decimal) -> Self {
        self.gross_price = Some(price);
        self
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn seller_item_id(mut self, id: impl Into<String>) -> Self {
        self.seller_item_id = Some(id.into());
        self
    }

    pub fn standard_item_id(mut self, id: impl Into<String>) -> Self {
        self.standard_item_id = Some(id.into());
        self
    }

    pub fn add_allowance(mut self, allowance: AllowanceCharge) -> Self {
        self.allowances.push(allowance);
        self
    }

    pub fn add_charge(mut self, charge: AllowanceCharge) -> Self {
        self.charges.push(charge);
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
            standard_item_id: self.standard_item_id,
            line_amount: None,
        }
    }
}
