use chrono::NaiveDate;
use faktura::core::*;
use faktura::datev::*;
use rust_decimal_macros::dec;

fn main() {
    // Build two invoices
    let inv1 = InvoiceBuilder::new("RE-2024-001", NaiveDate::from_ymd_opt(2024, 3, 15).unwrap())
        .tax_point_date(NaiveDate::from_ymd_opt(2024, 3, 15).unwrap())
        .seller(
            PartyBuilder::new(
                "ACME GmbH",
                AddressBuilder::new("Berlin", "10115", "DE").build(),
            )
            .vat_id("DE123456789")
            .build(),
        )
        .buyer(
            PartyBuilder::new(
                "Kunde AG",
                AddressBuilder::new("München", "80331", "DE").build(),
            )
            .build(),
        )
        .add_line(
            LineItemBuilder::new("1", "Beratung", dec!(10), "HUR", dec!(100))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .build()
        .expect("invoice 1 valid");

    let inv2 = InvoiceBuilder::new("RE-2024-002", NaiveDate::from_ymd_opt(2024, 3, 20).unwrap())
        .tax_point_date(NaiveDate::from_ymd_opt(2024, 3, 20).unwrap())
        .seller(
            PartyBuilder::new(
                "ACME GmbH",
                AddressBuilder::new("Berlin", "10115", "DE").build(),
            )
            .vat_id("DE123456789")
            .build(),
        )
        .buyer(
            PartyBuilder::new(
                "Firma XY",
                AddressBuilder::new("Hamburg", "20095", "DE").build(),
            )
            .build(),
        )
        .add_line(
            LineItemBuilder::new("1", "Hosting", dec!(1), "C62", dec!(49.90))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .build()
        .expect("invoice 2 valid");

    // Configure DATEV export
    let config = DatevConfigBuilder::new(12345, 99999)
        .fiscal_year_start(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap())
        .chart(ChartOfAccounts::SKR03)
        .exported_by("faktura")
        .description("März 2024")
        .build();

    // Generate EXTF CSV
    let csv = to_extf(&[inv1, inv2], &config).expect("DATEV export failed");
    println!("=== DATEV EXTF Buchungsstapel ===");
    for (i, line) in csv.lines().enumerate().take(5) {
        println!("Line {}: {}", i + 1, &line[..100.min(line.len())]);
    }
    println!("... ({} bytes total)\n", csv.len());

    // Account lookup demo
    let accounts = account_by_name(ChartOfAccounts::SKR03, "Erlöse 19%");
    println!("SKR03 accounts matching 'Erlöse 19%':");
    for acc in &accounts {
        println!(
            "  {} — {} (Automatik: {})",
            acc.number, acc.name, acc.is_automatik
        );
    }
}
