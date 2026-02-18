#![cfg(feature = "xrechnung")]

//! Integration tests against the official KoSIT XRechnung test suite.
//!
//! Tests parsing of all 86 reference UBL and CII invoices from
//! <https://github.com/itplr-kosit/xrechnung-testsuite> (v2026-01-31).

use faktura::xrechnung;
use std::fs;
use std::path::Path;

/// Collect all XML files from a fixture directory.
fn collect_xml_files(dir: &str) -> Vec<(String, String)> {
    let base = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/xrechnung-testsuite")
        .join(dir);
    if !base.exists() {
        panic!("fixture directory not found: {}", base.display());
    }

    let mut files = Vec::new();
    for entry in fs::read_dir(&base).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "xml") {
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            let content = fs::read_to_string(&path).unwrap();
            files.push((name, content));
        }
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    files
}

// ---------------------------------------------------------------------------
// Parse Tests — every reference file must parse without panic
// ---------------------------------------------------------------------------

#[test]
fn parse_standard_ubl_files() {
    let files = collect_xml_files("standard");
    let ubl_files: Vec<_> = files.iter().filter(|(n, _)| n.contains("_ubl")).collect();
    assert!(!ubl_files.is_empty(), "no UBL files found in standard/");

    let mut failures = Vec::new();
    for (name, xml) in &ubl_files {
        match xrechnung::from_ubl_xml(xml) {
            Ok(inv) => {
                // Basic sanity: invoice number should not be empty
                if inv.number.trim().is_empty() {
                    failures.push(format!("{name}: parsed but invoice number is empty"));
                }
            }
            Err(e) => {
                failures.push(format!("{name}: {e}"));
            }
        }
    }

    if !failures.is_empty() {
        panic!(
            "Failed to parse {}/{} UBL files:\n  {}",
            failures.len(),
            ubl_files.len(),
            failures.join("\n  ")
        );
    }
}

#[test]
fn parse_standard_cii_files() {
    let files = collect_xml_files("standard");
    let cii_files: Vec<_> = files
        .iter()
        .filter(|(n, _)| n.contains("_uncefact"))
        .collect();
    assert!(!cii_files.is_empty(), "no CII files found in standard/");

    let mut failures = Vec::new();
    for (name, xml) in &cii_files {
        match xrechnung::from_cii_xml(xml) {
            Ok(inv) => {
                if inv.number.trim().is_empty() {
                    failures.push(format!("{name}: parsed but invoice number is empty"));
                }
            }
            Err(e) => {
                failures.push(format!("{name}: {e}"));
            }
        }
    }

    if !failures.is_empty() {
        panic!(
            "Failed to parse {}/{} CII files:\n  {}",
            failures.len(),
            cii_files.len(),
            failures.join("\n  ")
        );
    }
}

#[test]
fn parse_technical_cius_ubl_files() {
    let files = collect_xml_files("technical-cases/cius");
    let ubl_files: Vec<_> = files.iter().filter(|(n, _)| n.contains("_ubl")).collect();
    assert!(
        !ubl_files.is_empty(),
        "no UBL files in technical-cases/cius/"
    );

    let mut failures = Vec::new();
    for (name, xml) in &ubl_files {
        match xrechnung::from_ubl_xml(xml) {
            Ok(inv) => {
                if inv.number.trim().is_empty() {
                    failures.push(format!("{name}: parsed but invoice number is empty"));
                }
            }
            Err(e) => {
                failures.push(format!("{name}: {e}"));
            }
        }
    }

    if !failures.is_empty() {
        panic!(
            "Failed to parse {}/{} CIUS UBL files:\n  {}",
            failures.len(),
            ubl_files.len(),
            failures.join("\n  ")
        );
    }
}

#[test]
fn parse_technical_cius_cii_files() {
    let files = collect_xml_files("technical-cases/cius");
    let cii_files: Vec<_> = files
        .iter()
        .filter(|(n, _)| n.contains("_uncefact"))
        .collect();
    assert!(
        !cii_files.is_empty(),
        "no CII files in technical-cases/cius/"
    );

    let mut failures = Vec::new();
    for (name, xml) in &cii_files {
        match xrechnung::from_cii_xml(xml) {
            Ok(inv) => {
                if inv.number.trim().is_empty() {
                    failures.push(format!("{name}: parsed but invoice number is empty"));
                }
            }
            Err(e) => {
                failures.push(format!("{name}: {e}"));
            }
        }
    }

    if !failures.is_empty() {
        panic!(
            "Failed to parse {}/{} CIUS CII files:\n  {}",
            failures.len(),
            cii_files.len(),
            failures.join("\n  ")
        );
    }
}

#[test]
fn parse_extension_files() {
    let files = collect_xml_files("extension");
    assert!(!files.is_empty(), "no files in extension/");

    let mut failures = Vec::new();
    for (name, xml) in &files {
        let result = if name.contains("_ubl") {
            xrechnung::from_ubl_xml(xml)
        } else if name.contains("_uncefact") {
            xrechnung::from_cii_xml(xml)
        } else {
            continue;
        };

        match result {
            Ok(inv) => {
                if inv.number.trim().is_empty() {
                    failures.push(format!("{name}: parsed but invoice number is empty"));
                }
            }
            Err(e) => {
                failures.push(format!("{name}: {e}"));
            }
        }
    }

    if !failures.is_empty() {
        panic!(
            "Failed to parse {}/{} extension files:\n  {}",
            failures.len(),
            files.len(),
            failures.join("\n  ")
        );
    }
}

#[test]
fn parse_technical_cvd_files() {
    let files = collect_xml_files("technical-cases/cvd");
    assert!(!files.is_empty(), "no files in technical-cases/cvd/");

    let mut failures = Vec::new();
    for (name, xml) in &files {
        let result = if name.contains("_ubl") {
            xrechnung::from_ubl_xml(xml)
        } else if name.contains("_uncefact") {
            xrechnung::from_cii_xml(xml)
        } else {
            continue;
        };

        match result {
            Ok(inv) => {
                if inv.number.trim().is_empty() {
                    failures.push(format!("{name}: parsed but invoice number is empty"));
                }
            }
            Err(e) => {
                failures.push(format!("{name}: {e}"));
            }
        }
    }

    if !failures.is_empty() {
        panic!(
            "Failed to parse {}/{} CVD files:\n  {}",
            failures.len(),
            files.len(),
            failures.join("\n  ")
        );
    }
}

// ---------------------------------------------------------------------------
// Roundtrip Tests — parse → generate → parse again → compare key fields
// ---------------------------------------------------------------------------

#[test]
fn roundtrip_standard_ubl_files() {
    let files = collect_xml_files("standard");
    let ubl_files: Vec<_> = files.iter().filter(|(n, _)| n.contains("_ubl")).collect();

    let mut failures = Vec::new();
    for (name, xml) in &ubl_files {
        let inv = match xrechnung::from_ubl_xml(xml) {
            Ok(inv) => inv,
            Err(_) => continue, // skip files that don't parse (tested separately)
        };

        // Generate UBL from parsed invoice
        let generated = match xrechnung::to_ubl_xml(&inv) {
            Ok(xml) => xml,
            Err(e) => {
                failures.push(format!("{name}: generation failed: {e}"));
                continue;
            }
        };

        // Parse the generated XML
        let inv2 = match xrechnung::from_ubl_xml(&generated) {
            Ok(inv) => inv,
            Err(e) => {
                failures.push(format!("{name}: re-parse failed: {e}"));
                continue;
            }
        };

        // Compare key fields
        if inv.number != inv2.number {
            failures.push(format!(
                "{name}: number mismatch: {:?} vs {:?}",
                inv.number, inv2.number
            ));
        }
        if inv.issue_date != inv2.issue_date {
            failures.push(format!(
                "{name}: issue_date mismatch: {:?} vs {:?}",
                inv.issue_date, inv2.issue_date
            ));
        }
        if inv.currency_code != inv2.currency_code {
            failures.push(format!(
                "{name}: currency mismatch: {:?} vs {:?}",
                inv.currency_code, inv2.currency_code
            ));
        }
        if inv.lines.len() != inv2.lines.len() {
            failures.push(format!(
                "{name}: line count mismatch: {} vs {}",
                inv.lines.len(),
                inv2.lines.len()
            ));
        }
    }

    if !failures.is_empty() {
        panic!(
            "UBL roundtrip failures ({}):\n  {}",
            failures.len(),
            failures.join("\n  ")
        );
    }
}

#[test]
fn roundtrip_standard_cii_files() {
    let files = collect_xml_files("standard");
    let cii_files: Vec<_> = files
        .iter()
        .filter(|(n, _)| n.contains("_uncefact"))
        .collect();

    let mut failures = Vec::new();
    for (name, xml) in &cii_files {
        let inv = match xrechnung::from_cii_xml(xml) {
            Ok(inv) => inv,
            Err(_) => continue,
        };

        let generated = match xrechnung::to_cii_xml(&inv) {
            Ok(xml) => xml,
            Err(e) => {
                failures.push(format!("{name}: generation failed: {e}"));
                continue;
            }
        };

        let inv2 = match xrechnung::from_cii_xml(&generated) {
            Ok(inv) => inv,
            Err(e) => {
                failures.push(format!("{name}: re-parse failed: {e}"));
                continue;
            }
        };

        if inv.number != inv2.number {
            failures.push(format!(
                "{name}: number mismatch: {:?} vs {:?}",
                inv.number, inv2.number
            ));
        }
        if inv.issue_date != inv2.issue_date {
            failures.push(format!(
                "{name}: issue_date mismatch: {:?} vs {:?}",
                inv.issue_date, inv2.issue_date
            ));
        }
        if inv.currency_code != inv2.currency_code {
            failures.push(format!(
                "{name}: currency mismatch: {:?} vs {:?}",
                inv.currency_code, inv2.currency_code
            ));
        }
        if inv.lines.len() != inv2.lines.len() {
            failures.push(format!(
                "{name}: line count mismatch: {} vs {}",
                inv.lines.len(),
                inv2.lines.len()
            ));
        }
    }

    if !failures.is_empty() {
        panic!(
            "CII roundtrip failures ({}):\n  {}",
            failures.len(),
            failures.join("\n  ")
        );
    }
}

// ---------------------------------------------------------------------------
// Specific file sanity checks
// ---------------------------------------------------------------------------

/// 01.01a is the "standard" business case with one line at 7% and one at 19%.
#[test]
fn parse_01_01a_ubl_sanity() {
    let base = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/xrechnung-testsuite/standard/01.01a-INVOICE_ubl.xml");
    let xml = fs::read_to_string(&base).unwrap();
    let inv = xrechnung::from_ubl_xml(&xml).unwrap();

    assert_eq!(inv.number, "123456XX");
    assert_eq!(inv.currency_code, "EUR");
    assert!(!inv.lines.is_empty());
    assert!(inv.totals.is_some());
}

#[test]
fn parse_01_01a_cii_sanity() {
    let base = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/xrechnung-testsuite/standard/01.01a-INVOICE_uncefact.xml");
    let xml = fs::read_to_string(&base).unwrap();
    let inv = xrechnung::from_cii_xml(&xml).unwrap();

    assert_eq!(inv.number, "123456XX");
    assert_eq!(inv.currency_code, "EUR");
    assert!(!inv.lines.is_empty());
    assert!(inv.totals.is_some());
}
