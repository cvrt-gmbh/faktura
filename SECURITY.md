# Security Policy

## Unsafe Code

This crate uses `#![forbid(unsafe_code)]`. No unsafe code exists anywhere in the library.

## Dependencies

All dependencies are well-maintained crates from the Rust ecosystem:

| Dependency | Purpose | Security Notes |
|------------|---------|----------------|
| `chrono` | Date handling | No unsafe, widely audited |
| `rust_decimal` | Decimal arithmetic | No floating-point precision issues |
| `serde` | Serialization | Industry standard |
| `thiserror` | Error derives | Proc macro only |
| `quick-xml` | XML parsing | Memory-safe, fuzz-tested |
| `lopdf` | PDF manipulation | Used for ZUGFeRD embed/extract |
| `reqwest` | HTTP client (VIES only) | TLS via rustls (no OpenSSL) |

## Panic Behavior

The library is designed to never panic on user input:

- All parsers (`from_ubl_xml`, `from_cii_xml`, `extract_from_pdf`) return `Result` types
- Fuzz testing has been run for 72M+ iterations across 6 targets with zero crashes
- The only `expect()` call is in `DatevConfig::default()` for a hardcoded valid date literal

## Input Handling

- XML parsing uses `quick-xml` with default limits (no billion-laughs vulnerability)
- PDF parsing uses `lopdf` on in-memory byte slices (no file system access)
- All monetary calculations use `rust_decimal::Decimal` (no floating-point rounding errors)
- Invoice number sequences are bounded by `u64` counter

## Network Access

Only the `vat` feature makes network calls (VIES API for VAT number validation). All other features are fully offline. The VIES client uses HTTPS (rustls) and does not send any sensitive data beyond the VAT number being checked.

## Reporting Vulnerabilities

If you discover a security vulnerability, please report it privately:

- Email: **security@cavort.de**
- Do NOT open a public GitHub issue for security vulnerabilities
- We will acknowledge receipt within 48 hours
- We aim to release a fix within 7 days for critical issues

## Supported Versions

| Version | Supported |
|---------|-----------|
| 0.1.x | Yes |
| < 0.1 | No |
