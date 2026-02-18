use chrono::NaiveDate;
use faktura::core::*;
use faktura::gdpdu::*;
use rust_decimal_macros::dec;

fn main() {
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
        .expect("invoice valid");

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
        .add_line(
            LineItemBuilder::new("2", "Domain", dec!(1), "C62", dec!(12))
                .tax(TaxCategory::StandardRate, dec!(19))
                .build(),
        )
        .build()
        .expect("invoice valid");

    // Configure GDPdU export
    let config = GdpduConfig {
        company_name: "ACME GmbH".into(),
        ..Default::default()
    };

    let export = to_gdpdu(&[inv1, inv2], &config).expect("GDPdU export failed");

    println!("=== GDPdU Export ===\n");
    println!("--- index.xml (first 20 lines) ---");
    for line in export.index_xml.lines().take(20) {
        println!("{line}");
    }
    println!("...\n");

    for (name, content) in &export.files {
        println!("--- {name} ---");
        for line in content.lines().take(5) {
            println!("{line}");
        }
        println!("...\n");
    }

    println!("DTD included: {} bytes", export.dtd.len());
}
