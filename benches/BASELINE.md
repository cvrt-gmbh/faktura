# Benchmark Baseline

Recorded on Apple Silicon (M-series), Rust 1.85, `--features all`, criterion 0.5.

## Results (v0.1.12)

| Benchmark | Median | Description |
|-----------|--------|-------------|
| `build_invoice_10_lines` | 3.14 µs | InvoiceBuilder with 10 line items |
| `ubl_serialize` | 11.14 µs | UBL XML generation (10 lines) |
| `ubl_parse` | 19.83 µs | UBL XML parsing (10 lines) |
| `cii_serialize` | 11.12 µs | CII XML generation (10 lines) |
| `cii_parse` | 25.90 µs | CII XML parsing (10 lines) |
| `validate_xrechnung_full` | 1.06 µs | §14 UStG + EN 16931 validation |
| `zugferd_embed` | 138.38 µs | Embed XML into PDF/A-3 |
| `zugferd_extract` | 83.47 µs | Extract XML from ZUGFeRD PDF |
| `datev_extf_100_invoices` | 36.36 µs | DATEV EXTF export (100 invoices) |
| `validate_full_pipeline` | 1.84 µs | Full validation stack (§14 + EN 16931 + arithmetic + XRechnung + Peppol) |
| `ubl_serialize_1000_lines` | 743.58 µs | UBL XML generation (1000 lines) |
| `ubl_parse_1000_lines` | 1.43 ms | UBL XML parsing (1000 lines) |

## Scaling Analysis

| Operation | 10 lines | 1000 lines | Ratio | Expected (linear) |
|-----------|----------|------------|-------|--------------------|
| UBL serialize | 11.14 µs | 743.58 µs | 66.7× | 100× |
| UBL parse | 19.83 µs | 1433.5 µs | 72.3× | 100× |

Both scale sub-linearly (better than O(n)) — no O(n²) paths detected.

## Key Observations

- **Validation is fast**: full pipeline under 2 µs — negligible overhead
- **Serialization ≈ parsing speed**: UBL/CII serialize and parse are in the same order of magnitude
- **CII parsing is ~30% slower than UBL**: expected due to deeper nesting in CrossIndustryInvoice
- **ZUGFeRD embed is PDF-bound**: 138 µs dominated by lopdf document manipulation
- **DATEV export is trivial**: 100 invoices in 36 µs (0.36 µs/invoice)
- **No bottlenecks**: all operations well under 1 ms for typical invoices

## Regression Threshold

Any benchmark regressing **>15%** from these baselines should be investigated.
