# Fuzz Testing

Fuzz targets for faktura XML parsers using [cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz) (libFuzzer).

## Requirements

- Rust nightly toolchain (`rustup install nightly`)
- `cargo install cargo-fuzz`

## Targets

| Target | What it tests |
|--------|---------------|
| `fuzz_ubl_parse` | `from_ubl_xml()` — arbitrary bytes as UBL XML |
| `fuzz_cii_parse` | `from_cii_xml()` — arbitrary bytes as CII XML |
| `fuzz_xml_autodetect` | `from_xml()` — auto-detect + parse |
| `fuzz_ubl_roundtrip` | parse → serialize → parse must not panic |

## Running

```sh
# Run a single target for 60 seconds
cargo +nightly fuzz run fuzz_ubl_parse -- -max_total_time=60

# Run all targets
for target in $(cargo fuzz list); do
  cargo +nightly fuzz run "$target" -- -max_total_time=60
done
```

## Corpus

Seeded from the KoSIT XRechnung test suite (86 real-world XML files).
Corpus directories are gitignored — they grow as the fuzzer discovers new paths.
