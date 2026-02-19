use chrono::NaiveDate;
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use rust_decimal_macros::dec;

use faktura::core::*;
use faktura::xrechnung;

fn test_date() -> NaiveDate {
    NaiveDate::from_ymd_opt(2024, 6, 15).unwrap()
}

fn build_10_line_invoice() -> Invoice {
    let mut builder = InvoiceBuilder::new("BENCH-001", test_date())
        .tax_point_date(test_date())
        .seller(
            PartyBuilder::new(
                "Benchmark GmbH",
                AddressBuilder::new("Berlin", "10115", "DE")
                    .street("Hauptstr. 1")
                    .build(),
            )
            .vat_id("DE123456789")
            .build(),
        )
        .buyer(
            PartyBuilder::new(
                "Kunde AG",
                AddressBuilder::new("München", "80331", "DE")
                    .street("Leopoldstr. 42")
                    .build(),
            )
            .build(),
        );

    for i in 1..=10 {
        builder = builder.add_line(
            LineItemBuilder::new(
                &i.to_string(),
                &format!("Service item {i}"),
                dec!(5),
                "HUR",
                dec!(120),
            )
            .tax(TaxCategory::StandardRate, dec!(19))
            .build(),
        );
    }

    builder.build().unwrap()
}

fn bench_build_invoice(c: &mut Criterion) {
    c.bench_function("build_invoice_10_lines", |b| {
        b.iter(|| {
            let mut builder = InvoiceBuilder::new("BENCH-001", test_date())
                .tax_point_date(test_date())
                .seller(
                    PartyBuilder::new(
                        "Benchmark GmbH",
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
                );

            for i in 1..=10 {
                builder = builder.add_line(
                    LineItemBuilder::new(
                        &i.to_string(),
                        &format!("Service item {i}"),
                        dec!(5),
                        "HUR",
                        dec!(120),
                    )
                    .tax(TaxCategory::StandardRate, dec!(19))
                    .build(),
                );
            }

            black_box(builder.build().unwrap())
        });
    });
}

fn bench_ubl_serialize(c: &mut Criterion) {
    let invoice = build_10_line_invoice();
    c.bench_function("ubl_serialize", |b| {
        b.iter(|| black_box(xrechnung::to_ubl_xml(black_box(&invoice))));
    });
}

fn bench_ubl_parse(c: &mut Criterion) {
    let invoice = build_10_line_invoice();
    let xml = xrechnung::to_ubl_xml(&invoice).unwrap();
    c.bench_function("ubl_parse", |b| {
        b.iter(|| black_box(xrechnung::from_ubl_xml(black_box(&xml))));
    });
}

fn bench_cii_serialize(c: &mut Criterion) {
    let invoice = build_10_line_invoice();
    c.bench_function("cii_serialize", |b| {
        b.iter(|| black_box(xrechnung::to_cii_xml(black_box(&invoice))));
    });
}

fn bench_cii_parse(c: &mut Criterion) {
    let invoice = build_10_line_invoice();
    let xml = xrechnung::to_cii_xml(&invoice).unwrap();
    c.bench_function("cii_parse", |b| {
        b.iter(|| black_box(xrechnung::from_cii_xml(black_box(&xml))));
    });
}

fn bench_validate_full(c: &mut Criterion) {
    let invoice = build_10_line_invoice();
    c.bench_function("validate_xrechnung_full", |b| {
        b.iter(|| {
            let mut errors = validate_14_ustg(black_box(&invoice));
            errors.extend(validate_en16931(black_box(&invoice)));
            black_box(errors)
        });
    });
}

criterion_group!(
    benches,
    bench_build_invoice,
    bench_ubl_serialize,
    bench_ubl_parse,
    bench_cii_serialize,
    bench_cii_parse,
    bench_validate_full,
);
criterion_main!(benches);
