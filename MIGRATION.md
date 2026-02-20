# Migration Guide

## 0.1.x → 0.2.0

### `#[non_exhaustive]` on enums

All public enums are now `#[non_exhaustive]`. If you match on them exhaustively, add a wildcard arm:

```rust
// Before
match category {
    TaxCategory::StandardRate => ...,
    TaxCategory::ZeroRated => ...,
    TaxCategory::Exempt => ...,
    TaxCategory::ReverseCharge => ...,
    TaxCategory::IntraCommunitySupply => ...,
    TaxCategory::Export => ...,
    TaxCategory::NotSubjectToVat => ...,
}

// After
match category {
    TaxCategory::StandardRate => ...,
    TaxCategory::ZeroRated => ...,
    // ... other variants ...
    _ => { /* handle unknown variants */ }
}
```

Affected enums: `TaxCategory`, `VatScenario`, `InvoiceTypeCode`, `PaymentMeansCode`, `RechnungError`, `XmlSyntax`, `ZugferdProfile`, `ChartOfAccounts`, `ViesError`, `DebitCredit`.

### `#[non_exhaustive]` on core structs

The following structs are now `#[non_exhaustive]`: `Invoice`, `Party`, `Address`, `LineItem`, `Totals`.

This means you cannot construct them with struct literals from outside the crate. Use the provided builders instead:

```rust
// Before (no longer compiles)
let party = Party { name: "ACME".into(), address: addr, ..Default::default() };

// After (use the builder)
let party = PartyBuilder::new("ACME", addr).build();
```

### No other breaking changes

All existing builder APIs, validation functions, and serialization formats remain unchanged.
