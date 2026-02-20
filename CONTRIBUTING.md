# Contributing to faktura

## Development Setup

```sh
# Clone
git clone https://github.com/cvrt-gmbh/faktura.git
cd faktura

# Build
cargo build --all-features

# Test
cargo test --all-features

# Lint
cargo clippy --all-features -- -D warnings
cargo fmt --check
```

### MSRV

The minimum supported Rust version is **1.85**. Make sure your changes compile on this version. CI tests against both stable and 1.85.

### Fuzz Testing (optional, requires nightly)

```sh
rustup install nightly
cargo install cargo-fuzz
cargo +nightly fuzz run fuzz_ubl_parse -- -max_total_time=60
```

## Testing Requirements

Before submitting a PR, ensure:

1. `cargo test --all-features` passes (all 450+ tests)
2. `cargo clippy --all-features -- -D warnings` has zero warnings
3. `cargo fmt --check` passes
4. `cargo doc --all-features --no-deps` builds without warnings
5. New functionality has corresponding tests

### Test Organization

| Directory | Purpose |
|-----------|---------|
| `src/` (unit tests) | Module-level unit tests |
| `tests/core_tests.rs` | Core invoice building and validation |
| `tests/xrechnung_tests.rs` | XRechnung XML generation/parsing/roundtrip |
| `tests/peppol_tests.rs` | Peppol BIS 3.0 validation |
| `tests/datev_tests.rs` | DATEV EXTF export |
| `tests/gdpdu_tests.rs` | GDPdU export |
| `tests/vat_tests.rs` | VAT format validation |
| `tests/proptest_tests.rs` | Property-based tests and edge cases |
| `tests/validator_tests.rs` | External KoSIT validator (ignored by default) |
| `tests/kosit_testsuite.rs` | KoSIT reference file parsing |
| `fuzz/` | Fuzz targets (requires nightly) |

## Commit Conventions

We follow [Conventional Commits](https://www.conventionalcommits.org/):

- `feat:` — new feature
- `fix:` — bug fix
- `docs:` — documentation only
- `style:` — formatting, no code change
- `refactor:` — code change that neither fixes a bug nor adds a feature
- `test:` — adding or updating tests
- `chore:` — maintenance tasks

Example: `feat: add support for UNTDID 1153 reference qualifiers`

## Pull Request Process

1. Fork the repository and create a feature branch
2. Make your changes with tests
3. Ensure the PR checklist passes (tests, clippy, fmt, docs)
4. Submit a PR against `main`
5. A maintainer will review within a few days

### What Makes a Good PR

- Small, focused changes (one feature or fix per PR)
- Tests for new functionality
- No unrelated formatting or refactoring changes
- Clear description of what and why

## Code Style

- Follow existing patterns in the codebase
- Use `rust_decimal::Decimal` for all monetary values (never `f64`)
- Use builder pattern for complex struct construction
- Use `thiserror` for error types
- Keep `#![forbid(unsafe_code)]` — no unsafe allowed

## Feature Flags

When adding new functionality:

- Core types go in `src/core/` (always available)
- Format-specific code goes behind a feature flag
- Optional dependencies use `dep:crate_name` syntax in Cargo.toml
- Gate modules with `#[cfg(feature = "...")]`

## Questions?

Open an issue on GitHub or reach out at security@cavort.de.
