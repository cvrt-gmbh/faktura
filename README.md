# faktura

[![crates.io](https://img.shields.io/crates/v/faktura)](https://crates.io/crates/faktura)
[![docs.rs](https://img.shields.io/docsrs/faktura)](https://docs.rs/faktura)
[![CI](https://img.shields.io/github/actions/workflow/status/cvrt-gmbh/faktura/ci.yml?label=CI)](https://github.com/cvrt-gmbh/faktura/actions)
[![license](https://img.shields.io/crates/l/faktura)](LICENSE-MIT)

Comprehensive German e-invoicing library for Rust.

Covers the full invoice lifecycle: creation, validation, XML generation (XRechnung/ZUGFeRD/Peppol), accounting export (DATEV/GDPdU), and VAT handling.

## Features

| Feature | Description |
|---------|-------------|
| `core` (default) | Invoice types, EN 16931 semantic model, §14 UStG validation, totals calculation, numbering |
| `xrechnung` | XRechnung UBL 2.1 / CII generation and parsing |
| `zugferd` | ZUGFeRD 2.x PDF/A-3 embed and extract (Minimum through XRechnung profiles) |
| `datev` | DATEV Buchungsstapel EXTF CSV export with SKR03/SKR04 account mapping |
| `gdpdu` | GDPdU/IDEA tax audit export (index.xml + CSV) |
| `vat` | VAT ID format validation, VIES API client, Kleinunternehmer §19 tracking |
| `peppol` | Peppol BIS Billing 3.0 document generation and validation |
| `all` | All of the above |

## Quick Start

```toml
[dependencies]
faktura = { version = "0.1", features = ["all"] }
```

```rust
use chrono::NaiveDate;
use faktura::core::*;
use rust_decimal_macros::dec;

let invoice = InvoiceBuilder::new(
    "RE-2024-001",
    NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
)
.tax_point_date(NaiveDate::from_ymd_opt(2024, 6, 15).unwrap())
.seller(
    PartyBuilder::new(
        "ACME GmbH",
        AddressBuilder::new("Berlin", "10115", "DE")
            .street("Friedrichstraße 123")
            .build(),
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
    LineItemBuilder::new("1", "IT-Beratung", dec!(10), "HUR", dec!(150))
        .tax(TaxCategory::StandardRate, dec!(19))
        .build(),
)
.build()
.expect("valid invoice");

// Validate
let errors = validate_14_ustg(&invoice);
assert!(errors.is_empty());

// Generate XRechnung XML
let xml = faktura::xrechnung::to_ubl_xml(&invoice).unwrap();

// Export to DATEV
let config = faktura::datev::DatevConfigBuilder::new(12345, 99999)
    .fiscal_year_start(NaiveDate::from_ymd_opt(2024, 1, 1).unwrap())
    .build();
let csv = faktura::datev::to_extf(&[invoice], &config).unwrap();
```

## Architecture

All monetary values use `rust_decimal::Decimal` — never floating point. The core types follow the EN 16931 semantic model (BG/BT business groups and terms).

The builder pattern (`InvoiceBuilder`, `PartyBuilder`, `LineItemBuilder`) auto-calculates line amounts and totals on `.build()`, so you get a ready-to-export invoice in one step.

### Validation

Three levels of validation:

- **`validate_14_ustg()`** — German §14 UStG mandatory fields
- **`validate_en16931()`** — EN 16931 business rules (BR-01 through BR-65, category-specific rules)
- **`validate_arithmetic()`** — Totals consistency checks

### VAT Scenarios

Automatic scenario detection via `vat::determine_scenario()`:

| Scenario | Description |
|----------|-------------|
| `Domestic` | Standard German invoice |
| `SmallInvoice` | Kleinbetragsrechnung (gross ≤ 250 EUR) |
| `Kleinunternehmer` | Small business §19 UStG |
| `ReverseCharge` | §13b UStG reverse charge |
| `IntraCommunitySupply` | §4 Nr. 1b intra-EU delivery |
| `Export` | §4 Nr. 1a third-country export |
| `Mixed` | Multiple VAT rates/scenarios |

## Examples

```sh
cargo run --features all --example basic_invoice
cargo run --features all --example xrechnung
cargo run --features all --example zugferd
cargo run --features all --example datev_export
cargo run --features all --example gdpdu_export
cargo run --features all --example vat_check
cargo run --features all --example peppol_invoice
cargo run --features all --example validation
```

## EN 16931 Coverage

The library implements the following business groups and terms from the EN 16931 semantic model:

| Group | Description | Status |
|-------|-------------|--------|
| BG-1 | Invoice note | Supported |
| BG-2 | Process control | Supported |
| BG-3 | Preceding invoice reference | Supported |
| — | BT-11 Project reference | Supported |
| — | BT-12 Contract reference | Supported |
| — | BT-14 Sales order reference | Supported |
| — | BT-19 Buyer accounting reference | Supported |
| BG-4 | Seller | Supported (name, address, contact, VAT ID, tax number, registration ID, trading name, electronic address) |
| BG-7 | Buyer | Supported (name, address, contact, VAT ID, registration ID, trading name, electronic address) |
| BG-10 | Payee | Supported (name, identifier, legal registration ID) |
| BG-11 | Seller tax representative | Supported (name, VAT ID, address) |
| BG-13 | Delivery information | Supported (actual delivery date, delivery party, delivery address) |
| BG-14 | Invoicing period | Supported |
| BG-16 | Payment instructions | Supported (means code, means text, remittance info) |
| BG-17 | Credit transfer | Supported (IBAN, BIC, account name) |
| BG-18 | Card payment | Supported (account number, holder name) |
| BG-19 | Direct debit | Supported (mandate ID, debited account, creditor ID) |
| BG-20/21 | Document allowances/charges | Supported |
| BG-22 | Total amounts | Supported |
| BG-23 | VAT breakdown | Supported |
| BG-24 | Attachments | Supported (embedded base64 or external URI) |
| BG-25 | Invoice line | Supported (all standard fields) |
| BG-26 | Line invoicing period | Supported |
| BG-27/28 | Line allowances/charges | Supported |
| BG-29 | Price details | Supported (net price, gross price, base quantity) |
| BG-31 | Item information | Supported (name, description, seller/buyer/standard ID, attributes, origin country) |

## Standards Compliance

- [EN 16931](https://standards.cencenelec.eu/dyn/www/f?p=205:110:0::::FSP_PROJECT:60602) — European electronic invoicing
- [XRechnung 3.0](https://xeinkauf.de/xrechnung/) — German CIUS of EN 16931
- [ZUGFeRD 2.x](https://www.ferd-net.de/zugferd/definition/index.html) — Hybrid PDF/XML invoices
- [Peppol BIS Billing 3.0](https://docs.peppol.eu/poacc/billing/3.0/) — Pan-European invoicing
- [DATEV EXTF](https://developer.datev.de/) — German accounting software import
- [GDPdU](https://www.bundesfinanzministerium.de) — German tax audit data export

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT License](LICENSE-MIT) at your option.
