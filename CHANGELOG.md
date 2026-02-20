# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2026-02-20

### Changed

- **release**: First stable release. API is frozen — breaking changes only in 2.0.
- Battle-tested against 452 real-world DATEV invoice PDFs (369 with embedded ZUGFeRD XML), all validated via KoSIT Schematron.
- 500+ tests, 6 fuzz targets (40M+ runs, 0 crashes), 16 property-based tests, 21 edge case tests, 11 criterion benchmarks.
- DATEV EXTF and GDPdU exports verified against production data.

## [0.2.1] - 2026-02-20

### Fixed

- **validation**: BR-CO-17 VAT amount tolerance widened from ±0.01 to ±0.02 — real-world invoices with many line items accumulate per-line rounding differences that exceed 1 cent
- **validation**: BR-CO-09 German VAT ID validation now strips whitespace before checking digit count — DATEV exports and other systems sometimes include spaces (e.g. `DE 123 456 789`)
- **parser**: CII parser now populates `tax_point_date` (BT-7) from `ActualDeliverySupplyChainEvent` (BT-72) as fallback, fixing BR-CO-03 failures on ZUGFeRD/Factur-X invoices
- **units**: Added `LS` (Lump sum) to known UN/CEFACT Rec 20 unit codes (now 88 codes)

### Added

- **test**: 21 edge case tests covering non-EUR currency, Skonto BR-DE-18, exempt VAT, XML special characters, credit notes, percentage allowances, rounding, SmallInvoice, ZUGFeRD credit note, seller/standard item IDs, multiple preceding refs, minimal invoice, delivery CII roundtrip, unicode addresses

## [0.2.0] - 2026-02-20

### Added

- **bench**: 11 criterion benchmarks — build, UBL/CII serialize/parse, ZUGFeRD embed/extract, DATEV EXTF, full validation pipeline, 1000-line stress tests
- **bench**: `BASELINE.md` with recorded performance numbers and scaling analysis
- **ci**: Benchmark regression detection on PRs via `critcmp` (15% threshold)
- **test**: Thread safety proof — `Send + Sync` compile-time check for all core types
- **test**: Concurrent invoice processing test (8 threads, build + serialize + validate)
- **test**: 16 property-based tests via `proptest` (UBL/CII roundtrip, arithmetic invariants, mandatory field checks)
- **test**: 12 edge case tests (Unicode, max-length strings, zero amounts, all payment means, all invoice types, attachments, allowances/charges, multi-currency)
- **ci**: `cargo-semver-checks` for semver compliance on every push
- **docs**: `SECURITY.md` — unsafe code policy, dependency audit, panic behavior, vulnerability reporting
- **docs**: `CONTRIBUTING.md` — dev setup, testing requirements, commit conventions, PR process
- **docs**: `ARCHITECTURE.md` — module overview, data flow, validation pipeline, design decisions
- **docs**: `MIGRATION.md` — upgrade guide for `#[non_exhaustive]` changes
- **example**: `error_handling.rs` — 4 error handling patterns

### Changed

- **api**: `#[non_exhaustive]` on all public enums (10) and core structs (`Invoice`, `Party`, `Address`, `LineItem`, `Totals`)
- **docs**: README expanded with Limitations, Recipes, MSRV Policy, and API Stability sections

### Fixed

- **bench**: ZUGFeRD embed benchmark uses correct PDF fixture path and 3-arg `embed_in_pdf` signature

## [0.1.12] - 2026-02-19

### Added

- **lib**: `#![forbid(unsafe_code)]` — compile-time guarantee of no unsafe code
- **lib**: `#![warn(missing_docs)]` — all public items now documented
- **fuzz**: 4 libFuzzer targets — `fuzz_ubl_parse`, `fuzz_cii_parse`, `fuzz_xml_autodetect`, `fuzz_ubl_roundtrip`
- **fuzz**: Corpus seeded from 86 KoSIT XRechnung test suite XML files
- **ci**: Fuzz job in GitHub Actions (60s per target on nightly)

### Fixed

- **datev**: `DatevConfig::default()` now uses `expect()` with clear message instead of bare `unwrap()`

## [0.1.11] - 2026-02-19

### Added

- **core**: ISO 4217 currency code validation — `is_known_currency_code()` with 44 common codes, integrated into `validate_14_ustg()` BR-05
- **core**: ISO 3166-1 alpha-2 country code validation — `is_known_country_code()` with all 249 codes, integrated into address validation
- **core**: UNTDID 5189/7161 reason code validation — `is_known_allowance_reason()` and `is_known_charge_reason()`, integrated into `validate_en16931()`
- **core**: Delivery address country code validation in `validate_en16931()`
- **peppol**: R080 — attachment total size limit (200 MB)
- **peppol**: R100 — per-line extension amount arithmetic check (qty × price ± allowances/charges, ±0.01 tolerance)
- **bench**: Criterion benchmarks for build, UBL/CII serialize/parse, and validation

## [0.1.10] - 2026-02-19

### Added

- **xrechnung**: `from_xml()` auto-detects UBL vs CII syntax by peeking at the root element, returns `(Invoice, XmlSyntax)`
- **core**: `InvoiceBuilder::build_strict()` — builds with full §14 UStG + EN 16931 validation (rejects duplicate line IDs, unknown unit codes, VAT breakdown issues)
- **core**: `is_known_unit_code()` — validates UN/CEFACT Rec 20 unit codes (85 common codes used in EN 16931 invoicing)
- **core**: Unit code validation integrated into `validate_en16931()` BR-26 check
- **core**: `RechnungError::Xml` variant for XML-related errors
- **peppol**: `validate_peppol_full()` convenience function combining `validate_14_ustg`, `validate_en16931`, and `validate_peppol`
- **tests**: 8 new tests — `from_xml` auto-detect (3), `build_strict` (2), unit code validation (2), unit code lookup (1)

## [0.1.9] - 2026-02-19

### Added

- **core**: Project reference (BT-11) — `project_reference: Option<String>` on `Invoice` with builder support
- **core**: Contract reference (BT-12) — `contract_reference: Option<String>` on `Invoice` with builder support
- **core**: Sales order reference (BT-14) — `sales_order_reference: Option<String>` on `Invoice` with builder support
- **core**: Buyer accounting reference (BT-19) — `buyer_accounting_reference: Option<String>` on `Invoice` with builder support
- **core**: Duplicate line ID validation (BR-CO-04) in `validate_en16931()`
- **xrechnung**: UBL/CII serialization and parsing for BT-11, BT-12, BT-14, BT-19
- **xrechnung**: `validate_xrechnung_full()` convenience function combining `validate_14_ustg`, `validate_en16931`, and `validate_xrechnung` into one call
- **xrechnung**: UBL creditor ID (BT-90) now serialized/parsed in `cac:PaymentMandate/cac:PayerParty/cac:PartyIdentification`
- **tests**: 11 new tests — tax representative exemption, duplicate line IDs, validate_en16931 rules, ZeroRated e2e, CII buyer Steuernummer roundtrip, UBL creditor_id roundtrip, BT references roundtrip (UBL + CII)

### Fixed

- **core**: Tax representative (BG-11) now exempts seller from VAT ID / tax number requirement (BR-CO-06/BR-CO-09)
- **xrechnung**: BR-DE-16 now accounts for tax representative exemption
- **zugferd**: BasicWL profile `write_cii_party` now emits tax number (FC scheme) in addition to VAT ID (VA scheme)
- **xrechnung**: UBL creditor ID (BT-90) no longer lost during roundtrip

## [0.1.8] - 2026-02-19

### Added

- **core**: Payee party (BG-10) — `Payee { name, identifier, legal_registration_id }` on `Invoice` with builder support
- **core**: Seller tax representative (BG-11) — `TaxRepresentative { name, vat_id, address }` on `Invoice` with builder support
- **core**: Payment means text (BT-82) — `means_text` on `PaymentInstructions`
- **core**: Line note (BT-127) — `note: Option<String>` on `LineItem`
- **core**: Base quantity (BT-149/BT-150) — `base_quantity` and `base_quantity_unit` on `LineItem` for price-per-unit pricing
- **core**: Buyer's item identifier (BT-156) — `buyer_item_id: Option<String>` on `LineItem`
- **core**: Item country of origin (BT-159) — `origin_country: Option<String>` on `LineItem`
- **xrechnung**: UBL/CII serialization and parsing for BG-10, BG-11, BG-18, BG-19, BT-82, BT-83, BT-127, BT-149/150, BT-156, BT-157, BT-159
- **xrechnung**: CII parsing now captures seller subdivision, buyer trading name, buyer contact, buyer registration ID, and standard item ID (GlobalID)
- **tests**: 31 new roundtrip tests — payee (4), tax representative (4), card payment (4), direct debit (4), payment means text (2), line note (2), base quantity (2), buyer item ID (2), origin country (2), seller subdivision (2), standard item ID CII (1), buyer contact CII (1), snapshot update (1)

## [0.1.7] - 2026-02-19

### Added

- **core**: Card payment (BG-18) — `CardPayment { account_number, holder_name }` on `PaymentInstructions`
- **core**: Direct debit (BG-19) — `DirectDebit { mandate_id, creditor_id, debited_account_id }` on `PaymentInstructions`
- **core**: `InvoiceTypeCode::Other(u16)` variant for non-standard UNTDID 1001 codes (e.g. 877)
- **core**: Delivery information (BG-13/BG-14/BG-15) — `DeliveryInformation`, `DeliveryParty`, `DeliveryAddress` types with full UBL/CII serialization and parsing
- **xrechnung**: UBL/CII serialization and parsing for line-level allowances/charges (BG-28), price details with gross price (BG-29), and document-level allowances/charges (BG-20/BG-21) roundtrip
- **xrechnung**: Comprehensive BR-DE-* Schematron validation — 13 new rules: BR-DE-14 (VAT rate), BR-DE-18 (Skonto format), BR-DE-19/20 (IBAN format), BR-DE-22 (unique filenames), BR-DE-23a/23b/24a/24b/25a/25b (payment means group exclusion), BR-DE-26 (preceding invoice for corrections), BR-DE-27 (phone format), BR-DE-28 (email format), BR-DE-30/31 (direct debit fields)
- **tests**: 16 new tests — delivery party roundtrip (5), line charges/price details (11), and 11 Schematron validation rule tests

### Fixed

- **xrechnung**: UBL `cac:Price` now correctly serializes price discount as `cac:AllowanceCharge` with `BaseAmount` (was incorrectly using `BaseQuantity` for gross price)

## [0.1.6] - 2026-02-18

### Added

- **core**: Item attributes (BT-160/BT-161) — `ItemAttribute { name, value }` on `LineItem` for product classification, serial numbers, etc.
- **core**: Line-level invoicing period (BG-26) — `invoicing_period: Option<Period>` on `LineItem` for per-line billing periods
- **core**: Preceding invoice reference (BT-25/BT-26) — `PrecedingInvoiceReference { number, issue_date }` for credit note and correction invoice references
- **core**: Tax currency support (BT-6/BT-111) — `tax_currency_code` on `Invoice` and `vat_total_in_tax_currency` on `Totals` for multi-currency VAT reporting
- **core**: Document attachments (BG-24) — `DocumentAttachment` with optional `EmbeddedDocument` (base64 binary) or external URI, limit 100 per invoice
- **xrechnung**: UBL serialization and parsing for all 5 new business groups
- **xrechnung**: CII serialization and parsing for all 5 new business groups
- **tests**: 21 new tests — builder, generation, and roundtrip tests for each feature in both UBL and CII

## [0.1.5] - 2026-02-18

### Added

- **tests**: Malformed XML input tests — empty string, non-XML, truncated XML, wrong root element, cross-format confusion (UBL↔CII)
- **tests**: Corrupt PDF input tests — empty bytes, non-PDF bytes, truncated PDF for both extract and embed
- **tests**: Peppol credit note generation, roundtrip, and validation

## [0.1.4] - 2026-02-18

### Added

- **tests**: KoSIT XRechnung testsuite integration — all 86 reference XML files (UBL + CII) parse and roundtrip correctly
- **tests**: ZUGFeRD reference PDF tests — extraction and embedding against real Mustang/EN16931/Extended PDFs
- **tests**: Credit note (type code 381) end-to-end tests — UBL and CII generation, roundtrip, and XRechnung validation
- **tests**: Document-level allowances/charges (BG-20/BG-21) end-to-end tests with totals verification
- **tests**: VIES async integration tests (ignored by default, `--ignored` to run)

### Fixed

- **zugferd**: PDF extraction now handles indirect `EF` references in filespec dictionaries — fixes extraction from MustangBeispiel and similar PDFs
- **xrechnung**: UBL parser now handles both prefixed (`ubl:Invoice`) and unprefixed (`Invoice`) root elements — fixes parsing of KoSIT 02.xx/03.xx series files
- **xrechnung**: UBL parser now supports `CreditNoteLine`, `CreditedQuantity`, and `CreditNoteTypeCode` elements

### Changed

- **core**: All validation errors now include EN 16931 BT/BG references and rule IDs (BR-xx) — previously ~20 errors used `ValidationError::new()` without rule context

## [0.1.3] - 2026-02-18

### Fixed

- **xrechnung**: CII element ordering — `SpecifiedTaxRegistration` now emitted after `PostalTradeAddress` and `URIUniversalCommunication` per CII schema (fixes KoSIT schema validation failure)
- **xrechnung**: UBL now emits `cac:Delivery/cbc:ActualDeliveryDate` for XRechnung compliance (BR-DE-TMP-32)
- **zugferd**: PDF/A-3 binary header comment (ISO 19005-3, clause 6.1.2) — 4 bytes > 127 after `%PDF` header
- **zugferd**: Embedded file MIME type now correctly `text/xml` instead of `text#2Fxml` (ISO 19005-3, clause 6.8)
- **zugferd**: Document ID in PDF trailer for PDF/A-3 compliance (ISO 19005-3, clause 6.1.3)

### Changed

- **zugferd**: PDF version upgraded to 1.7 (required for PDF/A-3)

## [0.1.2] - 2026-02-18

### Added

- **zugferd**: sRGB ICC profile and OutputIntent dictionary for PDF/A-3 compliance
- **zugferd**: Profile-specific CII XML generation — Minimum and BasicWL profiles now correctly omit line items per Factur-X specification
- **examples**: `gdpdu_export`, `vat_check`, `peppol_invoice`
- **docs**: All builder methods now have doc comments with EN 16931 BT/BG references

## [0.1.1] - 2026-02-18

### Fixed

- **core**: Enforce §14 Abs. 4 Nr. 6 UStG delivery date validation — `tax_point_date` or `invoicing_period` now required on all invoices (SmallInvoice exempt)
- **datev**: Fix UTF-8 truncation panic on multi-byte characters (e.g. German umlauts)
- **datev**: Escape double quotes in DATEV CSV fields to prevent field injection

### Added

- **core**: Input limits in builder — max 10,000 line items, 200-char invoice number, 100 notes

## [0.1.0] - 2026-02-18

### Added

- **core**: Invoice types following EN 16931 semantic model (BG/BT business groups)
- **core**: Builder pattern (`InvoiceBuilder`, `PartyBuilder`, `LineItemBuilder`, `AddressBuilder`)
- **core**: Auto-calculation of line amounts, VAT breakdown, and totals
- **core**: §14 UStG validation (`validate_14_ustg`)
- **core**: EN 16931 business rule validation (`validate_en16931`)
- **core**: Arithmetic consistency checks (`validate_arithmetic`)
- **core**: Gapless invoice number sequences with year rollover
- **core**: Support for 7 VAT scenarios (Domestic, Kleinunternehmer, ReverseCharge, etc.)
- **core**: Support for 6 tax categories (Standard, ZeroRated, Exempt, ReverseCharge, IntraCommunity, Export)
- **xrechnung**: XRechnung 3.0 UBL 2.1 XML generation (`to_ubl_xml`)
- **xrechnung**: XRechnung CII XML generation (`to_cii_xml`)
- **xrechnung**: UBL and CII XML parsing (`from_ubl_xml`, `from_cii_xml`)
- **zugferd**: ZUGFeRD 2.x XML generation for all profiles (Minimum through XRechnung)
- **zugferd**: PDF/A-3 embedding and extraction (`embed_in_pdf`, `extract_from_pdf`)
- **datev**: DATEV EXTF Buchungsstapel CSV export with BU-Schlüssel mapping
- **datev**: SKR03 and SKR04 account chart support with lookup functions
- **datev**: `DatevConfigBuilder` for fluent configuration
- **gdpdu**: GDPdU/IDEA tax audit export (index.xml + Rechnungsausgang/Kunden CSVs)
- **vat**: VAT ID format validation for all EU member states
- **vat**: VIES API client for VAT ID verification (async)
- **vat**: Kleinunternehmer §19 UStG revenue tracker
- **vat**: Automatic VAT scenario detection (`determine_scenario`)
- **peppol**: Peppol BIS Billing 3.0 UBL generation and parsing
- **peppol**: Peppol-specific validation rules
- **peppol**: EAS (Electronic Address Scheme) utilities
