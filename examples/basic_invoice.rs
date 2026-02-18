use chrono::NaiveDate;
use faktura::core::*;
use rust_decimal_macros::dec;

fn main() {
    // Create a standard German domestic invoice
    let invoice = InvoiceBuilder::new("RE-2024-001", NaiveDate::from_ymd_opt(2024, 6, 15).unwrap())
        .due_date(NaiveDate::from_ymd_opt(2024, 7, 15).unwrap())
        .tax_point_date(NaiveDate::from_ymd_opt(2024, 6, 15).unwrap())
        .seller(
            PartyBuilder::new(
                "ACME GmbH",
                AddressBuilder::new("Berlin", "10115", "DE")
                    .street("Friedrichstraße 123")
                    .build(),
            )
            .vat_id("DE123456789")
            .contact(
                Some("Max Mustermann".into()),
                Some("+49 30 12345".into()),
                Some("max@acme.de".into()),
            )
            .electronic_address("EM", "max@acme.de")
            .build(),
        )
        .buyer(
            PartyBuilder::new(
                "Kunde AG",
                AddressBuilder::new("München", "80331", "DE")
                    .street("Marienplatz 1")
                    .build(),
            )
            .build(),
        )
        .add_line(
            LineItemBuilder::new("1", "Softwareentwicklung", dec!(80), "HUR", dec!(120))
                .tax(TaxCategory::StandardRate, dec!(19))
                .description("React Frontend Entwicklung")
                .build(),
        )
        .add_line(
            LineItemBuilder::new("2", "Hosting (monatlich)", dec!(1), "C62", dec!(49.90))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .payment(PaymentInstructions {
            means_code: PaymentMeansCode::SepaCreditTransfer,
            means_text: Some("SEPA Überweisung".into()),
            remittance_info: Some("RE-2024-001".into()),
            credit_transfer: Some(CreditTransfer {
                iban: "DE89370400440532013000".into(),
                bic: Some("COBADEFFXXX".into()),
                account_name: Some("ACME GmbH".into()),
            }),
        })
        .payment_terms("Zahlbar innerhalb von 30 Tagen ohne Abzug")
        .build()
        .expect("invoice should be valid");

    let totals = invoice.totals.as_ref().unwrap();
    println!("Invoice: {}", invoice.number);
    println!("Date:    {}", invoice.issue_date);
    println!("Seller:  {}", invoice.seller.name);
    println!("Buyer:   {}", invoice.buyer.name);
    println!("---");
    for line in &invoice.lines {
        println!(
            "  {} x {} {} @ {} = {}",
            line.quantity,
            line.unit,
            line.item_name,
            line.unit_price,
            line.line_amount.unwrap()
        );
    }
    println!("---");
    println!("Net:     {} {}", totals.net_total, invoice.currency_code);
    println!("VAT:     {} {}", totals.vat_total, invoice.currency_code);
    println!("Gross:   {} {}", totals.gross_total, invoice.currency_code);
    println!("Due:     {} {}", totals.amount_due, invoice.currency_code);
}
