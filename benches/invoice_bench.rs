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

fn build_1000_line_invoice() -> Invoice {
    let mut builder = InvoiceBuilder::new("BENCH-BIG", test_date())
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

    for i in 1..=1000 {
        builder = builder.add_line(
            LineItemBuilder::new(
                &i.to_string(),
                &format!("Item {i}"),
                dec!(2),
                "C62",
                dec!(9.99),
            )
            .tax(TaxCategory::StandardRate, dec!(19))
            .build(),
        );
    }

    builder.build().unwrap()
}

fn build_100_invoices() -> Vec<Invoice> {
    (1..=100)
        .map(|n| {
            InvoiceBuilder::new(format!("RE-2024-{n:04}"), test_date())
                .tax_point_date(test_date())
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
                    LineItemBuilder::new("1", "Consulting", dec!(8), "HUR", dec!(150))
                        .tax(TaxCategory::StandardRate, dec!(19))
                        .build(),
                )
                .add_line(
                    LineItemBuilder::new("2", "Travel", dec!(1), "C62", dec!(250))
                        .tax(TaxCategory::StandardRate, dec!(19))
                        .build(),
                )
                .build()
                .unwrap()
        })
        .collect()
}

// ── Existing benchmarks ────────────────────────────────────────────

fn bench_build_invoice(c: &mut Criterion) {
    c.bench_function("build_invoice_10_lines", |b| {
        b.iter(|| black_box(build_10_line_invoice()));
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

// ── New benchmarks ─────────────────────────────────────────────────

fn bench_zugferd_embed_extract(c: &mut Criterion) {
    let invoice = build_10_line_invoice();
    let xml = faktura::zugferd::to_xml(&invoice, faktura::zugferd::ZugferdProfile::EN16931).unwrap();
    // Use a minimal valid PDF structure for embedding
    let pdf_bytes = include_bytes!("../tests/fixtures/zugferd-pdfs/MustangBeispiel20221026.pdf");
    let profile = faktura::zugferd::ZugferdProfile::EN16931;

    c.bench_function("zugferd_embed", |b| {
        b.iter(|| {
            black_box(faktura::zugferd::embed_in_pdf(
                black_box(pdf_bytes.as_slice()),
                black_box(&xml),
                black_box(profile),
            ))
        });
    });

    let embedded = faktura::zugferd::embed_in_pdf(pdf_bytes.as_slice(), &xml, profile).unwrap();
    c.bench_function("zugferd_extract", |b| {
        b.iter(|| black_box(faktura::zugferd::extract_from_pdf(black_box(&embedded))));
    });
}

fn bench_datev_export(c: &mut Criterion) {
    let invoices = build_100_invoices();
    let config = faktura::datev::DatevConfigBuilder::new(12345, 99999)
        .fiscal_year_start(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap())
        .exported_by("bench")
        .build();

    c.bench_function("datev_extf_100_invoices", |b| {
        b.iter(|| black_box(faktura::datev::to_extf(black_box(&invoices), black_box(&config))));
    });
}

fn bench_validation_pipeline(c: &mut Criterion) {
    let invoice = build_10_line_invoice();
    c.bench_function("validate_full_pipeline", |b| {
        b.iter(|| {
            let mut errors = validate_14_ustg(black_box(&invoice));
            errors.extend(validate_en16931(black_box(&invoice)));
            errors.extend(validate_arithmetic(black_box(&invoice)));
            errors.extend(faktura::xrechnung::validate_xrechnung(black_box(&invoice)));
            errors.extend(faktura::peppol::validate_peppol(black_box(&invoice)));
            black_box(errors)
        });
    });
}

fn bench_ubl_serialize_1000_lines(c: &mut Criterion) {
    let invoice = build_1000_line_invoice();
    c.bench_function("ubl_serialize_1000_lines", |b| {
        b.iter(|| black_box(xrechnung::to_ubl_xml(black_box(&invoice))));
    });
}

fn bench_ubl_parse_1000_lines(c: &mut Criterion) {
    let invoice = build_1000_line_invoice();
    let xml = xrechnung::to_ubl_xml(&invoice).unwrap();
    c.bench_function("ubl_parse_1000_lines", |b| {
        b.iter(|| black_box(xrechnung::from_ubl_xml(black_box(&xml))));
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
    bench_zugferd_embed_extract,
    bench_datev_export,
    bench_validation_pipeline,
    bench_ubl_serialize_1000_lines,
    bench_ubl_parse_1000_lines,
);
criterion_main!(benches);
