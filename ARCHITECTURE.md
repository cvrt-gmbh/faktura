# Architecture

## Module Overview

```
faktura/
├── src/
│   ├── lib.rs              # Crate root, feature gates, re-exports
│   ├── core/               # Always available (default feature)
│   │   ├── types.rs        # EN 16931 semantic model (Invoice, Party, LineItem, ...)
│   │   ├── builder.rs      # InvoiceBuilder, PartyBuilder, LineItemBuilder, AddressBuilder
│   │   ├── validation.rs   # §14 UStG, EN 16931, arithmetic validation
│   │   ├── error.rs        # RechnungError, ValidationError
│   │   ├── numbering.rs    # InvoiceNumberSequence (gapless §14 UStG)
│   │   ├── countries.rs    # ISO 3166-1 alpha-2 lookup
│   │   ├── currencies.rs   # ISO 4217 currency code lookup
│   │   ├── units.rs        # UN/CEFACT Rec 20 unit code lookup
│   │   └── reason_codes.rs # UNTDID 5189/7161 allowance/charge reason codes
│   ├── xrechnung/          # Feature: xrechnung
│   │   ├── ubl.rs          # UBL 2.1 XML generation and parsing
│   │   ├── cii.rs          # CII XML generation and parsing
│   │   ├── validate.rs     # XRechnung BR-DE-* rules
│   │   └── xml_utils.rs    # Shared XML helpers
│   ├── zugferd/            # Feature: zugferd (depends on xrechnung)
│   │   ├── profile.rs      # ZUGFeRD profile XML generation
│   │   ├── embed.rs        # PDF/A-3 embedding
│   │   ├── extract.rs      # XML extraction from PDF
│   │   └── xmp.rs          # XMP metadata for PDF/A-3
│   ├── datev/              # Feature: datev
│   │   ├── extf.rs         # EXTF CSV generation
│   │   ├── accounts.rs     # SKR03/SKR04 account mappings
│   │   └── bu_key.rs       # BU-Schlüssel (tax key) determination
│   ├── gdpdu/              # Feature: gdpdu
│   │   ├── index_xml.rs    # GDPdU index.xml generation
│   │   └── csv_export.rs   # Customer + invoice CSV files
│   ├── vat/                # Feature: vat
│   │   ├── format.rs       # VAT ID format validation (regex-free)
│   │   ├── vies.rs         # EU VIES REST API client
│   │   ├── kleinunternehmer.rs # §19 UStG threshold checks
│   │   └── scenario.rs     # Automatic VAT scenario detection
│   └── peppol/             # Feature: peppol (depends on xrechnung)
│       ├── validate.rs     # Peppol BIS 3.0 validation rules
│       └── eas.rs          # Electronic Address Scheme codes
```

## Data Flow

### Invoice Creation

```
InvoiceBuilder::new()
    .seller(PartyBuilder::new().build())
    .buyer(PartyBuilder::new().build())
    .add_line(LineItemBuilder::new().build())
    .build()
        │
        ├── calculate line amounts (qty × price ± allowances/charges)
        ├── calculate totals (net, VAT, gross, prepaid, due)
        └── return Invoice
```

### Validation Pipeline

```
Invoice
    │
    ├── validate_14_ustg()        §14 UStG mandatory fields
    ├── validate_en16931()        EN 16931 business rules + code lists
    ├── validate_arithmetic()     Totals consistency
    │
    ├── validate_xrechnung()      XRechnung BR-DE-* rules
    │   └── validate_xrechnung_full() = all of the above + XRechnung
    │
    └── validate_peppol()         Peppol PEPPOL-EN16931-* rules
        └── validate_peppol_full() = all of the above + Peppol
```

### XML Generation / Parsing

```
Invoice ──→ to_ubl_xml()  ──→ UBL 2.1 XML string
Invoice ──→ to_cii_xml()  ──→ CII XML string
Invoice ──→ to_xml() (ZUGFeRD) ──→ CII XML with profile URN

UBL XML ──→ from_ubl_xml() ──→ Invoice
CII XML ──→ from_cii_xml() ──→ Invoice
Any XML ──→ from_xml()     ──→ (Invoice, XmlSyntax)
```

### Export Pipelines

```
[Invoice] ──→ to_extf(&config)  ──→ DATEV EXTF CSV string
[Invoice] ──→ to_gdpdu(&config) ──→ GdpduExport { index_xml, files, dtd }
Invoice   ──→ embed_in_pdf()    ──→ PDF/A-3 bytes with embedded XML
PDF bytes ──→ extract_from_pdf() ──→ Invoice
```

## Key Design Decisions

1. **`Decimal` everywhere** — All monetary values use `rust_decimal::Decimal`. No floating point. Rounding only happens at final output boundaries (XML serialization, CSV export).

2. **Builder + calculate** — Builders auto-calculate line amounts and totals. Users never need to manually compute `line_amount = qty * price` or sum VAT breakdowns.

3. **Validation is separate from construction** — `InvoiceBuilder::build()` always succeeds if the structure is valid. Validation (§14 UStG, EN 16931, etc.) is opt-in. `build_strict()` combines both for convenience.

4. **Feature flags for optional formats** — Core types are always available. XML, PDF, DATEV, VAT, and Peppol are opt-in features to minimize dependency footprint.

5. **Code lists are static arrays** — Currency codes, country codes, unit codes, and reason codes are sorted `&[&str]` arrays with binary search. No runtime allocation, no external files needed.

6. **Roundtrip fidelity** — `to_ubl_xml() → from_ubl_xml()` and `to_cii_xml() → from_cii_xml()` preserve all fields. This is verified by property-based tests across millions of random invoices.
