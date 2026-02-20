//! Real-world PDF extraction tests — processes DATEV invoice PDFs.
//!
//! These PDFs are CAVORT production invoices exported from the billing system.
//! Each should contain an embedded Factur-X/ZUGFeRD XML that can be extracted,
//! parsed, and validated.

#[cfg(feature = "zugferd")]
mod zugferd_extraction {
    use std::path::PathBuf;

    /// All known DATEV export directories to scan.
    fn pdf_dirs() -> Vec<PathBuf> {
        vec![
            PathBuf::from(concat!(
                "/Users/jh/Downloads/",
                "BITTE-ENTPACKEN_Buchungsstapel_und_Belegbilder_1326467_20000_20260101_bis_20260131/",
                "DATEV_Rechnungsausgang_20260101_bis_20260131"
            )),
            PathBuf::from(concat!(
                "/Users/jh/Downloads/",
                "BITTE-ENTPACKEN_Buchungsstapel_und_Belegbilder_1326467_20000_20250101_bis_20251231/",
                "DATEV_Rechnungsausgang_20250101_bis_20251231"
            )),
        ]
    }

    fn list_pdfs() -> Vec<PathBuf> {
        let mut pdfs = Vec::new();
        for dir in pdf_dirs() {
            if !dir.exists() {
                continue;
            }
            let mut dir_pdfs: Vec<PathBuf> = std::fs::read_dir(&dir)
                .unwrap()
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|ext| ext == "pdf"))
                .collect();
            dir_pdfs.sort();
            pdfs.extend(dir_pdfs);
        }
        pdfs
    }

    #[test]
    #[ignore = "requires DATEV PDF export in ~/Downloads"]
    fn extract_zugferd_from_all_pdfs() {
        let pdfs = list_pdfs();
        if pdfs.is_empty() {
            eprintln!("No PDFs found in DATEV export directories, skipping");
            return;
        }

        eprintln!("Found {} PDFs to process", pdfs.len());

        let mut extracted = 0;
        let mut no_xml = 0;
        let mut errors = Vec::new();

        for pdf_path in &pdfs {
            let filename = pdf_path.file_name().unwrap().to_string_lossy();
            let bytes = std::fs::read(pdf_path).unwrap();

            match faktura::zugferd::extract_from_pdf(&bytes) {
                Ok(xml) => {
                    extracted += 1;
                    eprintln!("[OK] {filename} — extracted {} bytes of XML", xml.len());
                }
                Err(e) => {
                    let msg = format!("{e}");
                    if msg.contains("no embedded") || msg.contains("not found") {
                        no_xml += 1;
                        eprintln!("[SKIP] {filename} — no embedded XML");
                    } else {
                        errors.push(format!("{filename}: {e}"));
                        eprintln!("[ERR] {filename} — {e}");
                    }
                }
            }
        }

        eprintln!(
            "\nSummary: {} extracted, {} no XML, {} errors out of {} PDFs",
            extracted,
            no_xml,
            errors.len(),
            pdfs.len()
        );

        assert!(
            errors.is_empty(),
            "Extraction errors:\n{}",
            errors.join("\n")
        );
    }

    #[test]
    #[ignore = "requires DATEV PDF export in ~/Downloads"]
    #[cfg(feature = "xrechnung")]
    fn parse_and_validate_extracted_xml() {
        let pdfs = list_pdfs();
        if pdfs.is_empty() {
            eprintln!("No PDFs found, skipping");
            return;
        }

        let mut results = Vec::new();

        for pdf_path in &pdfs {
            let filename = pdf_path.file_name().unwrap().to_string_lossy();
            let bytes = std::fs::read(pdf_path).unwrap();

            let xml = match faktura::zugferd::extract_from_pdf(&bytes) {
                Ok(xml) => xml,
                Err(_) => continue,
            };

            // Parse the XML
            let (invoice, syntax) = match faktura::xrechnung::from_xml(&xml) {
                Ok(result) => result,
                Err(e) => {
                    eprintln!("[PARSE ERR] {filename}: {e}");
                    results.push((
                        filename.to_string(),
                        None,
                        vec![format!("parse error: {e}")],
                    ));
                    continue;
                }
            };

            let syntax_str = match syntax {
                faktura::xrechnung::XmlSyntax::Ubl => "UBL",
                faktura::xrechnung::XmlSyntax::Cii => "CII",
                _ => "Unknown",
            };

            // Validate
            let ustg_errors = faktura::core::validate_14_ustg(&invoice);
            let en_errors = faktura::core::validate_en16931(&invoice);
            let arith_errors = faktura::core::validate_arithmetic(&invoice);

            let totals = invoice.totals.as_ref();
            let total_str = totals
                .map(|t| {
                    format!(
                        "net={} vat={} gross={}",
                        t.net_total, t.vat_total, t.gross_total
                    )
                })
                .unwrap_or_else(|| "no totals".into());

            let all_errors: Vec<String> = ustg_errors
                .iter()
                .chain(en_errors.iter())
                .chain(arith_errors.iter())
                .map(|e| format!("{e}"))
                .collect();

            eprintln!(
                "[{syntax_str}] {} | {} | {} | lines={} | errors={}",
                invoice.number,
                invoice.currency_code,
                total_str,
                invoice.lines.len(),
                all_errors.len()
            );

            results.push((
                filename.to_string(),
                Some(invoice.number.clone()),
                all_errors,
            ));
        }

        // Summary table
        eprintln!("\n=== VALIDATION SUMMARY ===");
        let mut pass = 0;
        let mut fail = 0;
        for (filename, number, errors) in &results {
            let num = number.as_deref().unwrap_or("?");
            if errors.is_empty() {
                eprintln!("  PASS  {num} ({filename})");
                pass += 1;
            } else {
                eprintln!("  FAIL  {num} ({filename}):");
                for e in errors {
                    eprintln!("        - {e}");
                }
                fail += 1;
            }
        }
        eprintln!(
            "\n{pass} passed, {fail} failed out of {} parsed",
            results.len()
        );
    }

    #[test]
    #[ignore = "requires DATEV PDF export in ~/Downloads and KoSIT Docker"]
    #[cfg(feature = "xrechnung")]
    fn validate_extracted_xml_against_kosit() {
        let pdfs = list_pdfs();
        if pdfs.is_empty() {
            eprintln!("No PDFs found, skipping");
            return;
        }

        let client = reqwest::blocking::Client::new();
        let mut pass = 0;
        let mut fail = 0;
        let mut skip = 0;

        for pdf_path in &pdfs {
            let filename = pdf_path.file_name().unwrap().to_string_lossy();
            let bytes = std::fs::read(pdf_path).unwrap();

            let xml = match faktura::zugferd::extract_from_pdf(&bytes) {
                Ok(xml) => xml,
                Err(_) => {
                    skip += 1;
                    continue;
                }
            };

            let resp = client
                .post("http://localhost:8081/validation")
                .header("Content-Type", "application/xml")
                .body(xml)
                .send();

            match resp {
                Ok(r) if r.status().is_success() => {
                    eprintln!("[KOSIT OK] {filename}");
                    pass += 1;
                }
                Ok(r) => {
                    let body = r.text().unwrap_or_default();
                    eprintln!("[KOSIT FAIL] {filename}: {}", &body[..500.min(body.len())]);
                    fail += 1;
                }
                Err(e) => {
                    panic!("KoSIT not reachable: {e}");
                }
            }
        }

        eprintln!("\nKoSIT: {pass} valid, {fail} invalid, {skip} skipped (no XML)");
    }
}
