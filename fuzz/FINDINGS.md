# Fuzz Testing Findings

## Summary

**Result: ZERO crashes across all 6 fuzz targets.**

All parsers and serializers handle arbitrary input gracefully — no panics, no undefined behavior. The crate uses `#![forbid(unsafe_code)]`, so memory safety is guaranteed by the Rust compiler.

## Test Environment

- **Rust nightly**: 1.95.0-nightly (2025-06-15)
- **cargo-fuzz**: libFuzzer-based coverage-guided fuzzing
- **Platform**: macOS aarch64 (Apple Silicon)
- **Corpus**: Seeded with 86 XRechnung test suite XML files (KoSIT reference)

## Results Per Target

| Target | Duration | Iterations | Coverage | Crashes |
|--------|----------|------------|----------|---------|
| `fuzz_ubl_parse` | 10 min | 2,085,347 | 3,740 | **0** |
| `fuzz_cii_parse` | 10 min | 1,735,360 | 3,725 | **0** |
| `fuzz_xml_autodetect` | 10 min | 2,551,905 | 5,969 | **0** |
| `fuzz_ubl_roundtrip` | 10 min | 1,948,254 | 4,566 | **0** |
| `fuzz_cii_roundtrip` | 10 min | 16,063,844 | 960 | **0** |
| `fuzz_zugferd_extract` | 10 min | 47,665,108 | 176 | **0** |

**Total: 72M+ iterations, zero crashes.**

## Fuzz Targets

### fuzz_ubl_parse
Feeds arbitrary UTF-8 strings to `from_ubl_xml()`. Tests that the UBL parser never panics on malformed XML.

### fuzz_cii_parse
Feeds arbitrary UTF-8 strings to `from_cii_xml()`. Tests that the CII/Cross-Industry Invoice parser handles all input gracefully.

### fuzz_xml_autodetect
Feeds arbitrary bytes to `Invoice::from_xml()`. Tests the format auto-detection logic (UBL vs CII) plus subsequent parsing.

### fuzz_ubl_roundtrip
Parse UBL XML → serialize back to UBL XML → parse again. Tests that serialized output is always valid input (no round-trip divergence that causes panics).

### fuzz_cii_roundtrip
Parse CII XML → serialize back to CII XML → parse again. Same round-trip property test for the CII format.

### fuzz_zugferd_extract
Feeds arbitrary bytes as PDF input to `extract_from_pdf()`. Tests that the ZUGFeRD PDF extraction handles non-PDF and malformed PDF data without panicking.

## expect() Audit

Only one `expect()` call exists in all of `src/`:
- `src/datev/mod.rs` — `DatevConfig::default()` uses `expect()` on a hardcoded date string. This is safe (compile-time constant, can never fail in practice).

All other error handling uses `?` propagation or returns `Result`.

## CI Integration

The GitHub Actions CI workflow runs all 6 fuzz targets for 60 seconds each on every push/PR. This provides continuous regression coverage. See `.github/workflows/ci.yml` (fuzz job).

## Methodology

- Coverage-guided fuzzing with entropic power schedule (libFuzzer default)
- Corpus seeded from real-world XRechnung test suite XML files
- No sanitizer flags beyond default (AddressSanitizer is enabled by cargo-fuzz by default)
- Each target runs in its own process with isolated corpus
