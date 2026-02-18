# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
