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

    /// Extract all invoices from PDFs, parse, and return them.
    #[cfg(feature = "xrechnung")]
    fn extract_and_parse_all() -> Vec<faktura::core::Invoice> {
        let mut invoices = Vec::new();
        for pdf_path in &list_pdfs() {
            let bytes = std::fs::read(pdf_path).unwrap();
            let xml = match faktura::zugferd::extract_from_pdf(&bytes) {
                Ok(xml) => xml,
                Err(_) => continue,
            };
            if let Ok((invoice, _)) = faktura::xrechnung::from_xml(&xml) {
                invoices.push(invoice);
            }
        }
        invoices
    }

    #[test]
    #[ignore = "requires DATEV PDF export in ~/Downloads"]
    #[cfg(all(feature = "xrechnung", feature = "datev", feature = "gdpdu"))]
    fn export_datev_extf_and_gdpdu_to_folder() {
        use chrono::NaiveDate;

        let invoices = extract_and_parse_all();
        if invoices.is_empty() {
            eprintln!("No invoices parsed, skipping");
            return;
        }

        let out_dir = PathBuf::from("/Users/jh/Downloads/faktura-export");
        std::fs::create_dir_all(&out_dir).unwrap();

        eprintln!(
            "Exporting {} invoices to {}",
            invoices.len(),
            out_dir.display()
        );

        // --- DATEV EXTF ---
        let datev_config = faktura::datev::DatevConfigBuilder::new(1326467, 20000)
            .fiscal_year_start(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap())
            .chart(faktura::datev::ChartOfAccounts::SKR03)
            .exported_by("faktura")
            .description("CAVORT 2025+2026 re-export via faktura")
            .build();

        let extf =
            faktura::datev::to_extf(&invoices, &datev_config).expect("DATEV EXTF export failed");
        let extf_path = out_dir.join("EXTF_Buchungsstapel.csv");
        std::fs::write(&extf_path, &extf).unwrap();
        let extf_lines = extf.lines().count();
        eprintln!(
            "[DATEV] {} lines, {} bytes → {}",
            extf_lines,
            extf.len(),
            extf_path.display()
        );

        // --- GDPdU ---
        let gdpdu_config = faktura::gdpdu::GdpduConfig {
            company_name: "CAVORT Konzepte GmbH".into(),
            location: "Deutschland".into(),
            comment: "GDPdU-Export Ausgangsrechnungen 2025-2026 via faktura".into(),
        };

        let gdpdu =
            faktura::gdpdu::to_gdpdu(&invoices, &gdpdu_config).expect("GDPdU export failed");

        let gdpdu_dir = out_dir.join("gdpdu");
        std::fs::create_dir_all(&gdpdu_dir).unwrap();
        std::fs::write(gdpdu_dir.join("index.xml"), &gdpdu.index_xml).unwrap();
        std::fs::write(gdpdu_dir.join("gdpdu-01-08-2002.dtd"), gdpdu.dtd).unwrap();
        for (name, content) in &gdpdu.files {
            std::fs::write(gdpdu_dir.join(name), content).unwrap();
            let lines = content.lines().count();
            eprintln!("[GDPdU] {name}: {lines} lines, {} bytes", content.len());
        }
        eprintln!("[GDPdU] index.xml: {} bytes", gdpdu.index_xml.len());

        // --- Summary / expected outcomes ---
        let mut summary = String::new();
        summary.push_str("# faktura Export — Expected Outcomes\n\n");
        summary.push_str(&format!(
            "Generated: {}\n",
            chrono::Local::now().format("%Y-%m-%d %H:%M")
        ));
        summary.push_str("Source: 452 DATEV invoice PDFs (Jan 2025 – Jan 2026)\n");
        summary.push_str(&format!(
            "Parsed: {} invoices with embedded ZUGFeRD XML\n\n",
            invoices.len()
        ));

        summary.push_str("## Files in this folder\n\n");
        summary.push_str("| File | Description |\n");
        summary.push_str("|------|-------------|\n");
        summary.push_str(&format!(
            "| `EXTF_Buchungsstapel.csv` | DATEV EXTF Buchungsstapel ({} lines, {} bytes) |\n",
            extf_lines,
            extf.len()
        ));
        summary.push_str("| `gdpdu/index.xml` | GDPdU index file (schema + table definitions) |\n");
        summary.push_str("| `gdpdu/gdpdu-01-08-2002.dtd` | GDPdU DTD (required by IDEA) |\n");
        for (name, content) in &gdpdu.files {
            summary.push_str(&format!(
                "| `gdpdu/{name}` | {} lines, {} bytes |\n",
                content.lines().count(),
                content.len()
            ));
        }
        summary.push_str("\n## DATEV EXTF — What to check\n\n");
        summary
            .push_str("1. **Import into DATEV**: Rechnungswesen → Stapelverarbeitung → Import\n");
        summary.push_str("2. **Header row**: Verify Berater-Nr (1326467), Mandanten-Nr (20000), WJ-Beginn (20250101)\n");
        summary.push_str(&format!(
            "3. **Row count**: {} data rows (1 row per VAT breakdown group, not per invoice)\n",
            extf_lines.saturating_sub(2)
        ));
        summary.push_str("4. **Encoding**: ISO 8859-1 (Latin-1), CRLF line endings\n");
        summary.push_str(
            "5. **Account mapping**: SKR03 Automatikkonten (8400 for 19%, 8300 for 7%, etc.)\n",
        );
        summary.push_str(
            "6. **Amounts**: Gross amounts in column 1, net in Belegfeld, VAT via BU-Schlüssel\n",
        );
        summary.push_str("7. **Dates**: Belegdatum matches invoice issue date (DDMM format)\n");
        summary.push_str(
            "8. **Credit notes**: TypeCode 381 invoices should have negative amounts (Haben)\n\n",
        );

        summary.push_str("## GDPdU — What to check\n\n");
        summary.push_str("1. **Import into IDEA**: File → Import → GDPdU, select `index.xml`\n");
        summary.push_str(
            "2. **DTD validation**: `index.xml` references `gdpdu-01-08-2002.dtd` (included)\n",
        );
        summary.push_str(
            "3. **Tables**: `kunden.csv` (unique buyers) + `rechnungsausgang.csv` (all invoices)\n",
        );
        summary.push_str(
            "4. **Cross-reference**: Customer IDs in `rechnungsausgang.csv` match `kunden.csv`\n",
        );
        summary.push_str("5. **Encoding**: UTF-8, semicolon-separated\n");
        summary.push_str("6. **Decimal format**: German notation (comma as decimal separator)\n");
        summary.push_str("7. **Completeness**: Every invoice with embedded XML should appear\n\n");

        summary.push_str("## Known issues\n\n");
        summary.push_str(
            "- 2 invoices (R-2025-09-1227, R-2025-09-1228) have an invalid buyer VAT ID\n",
        );
        summary
            .push_str("  (`DE 040 80861508` — this is a Steuernummer, not a USt-IdNr). They are\n");
        summary.push_str("  included in the export but will show validation warnings.\n");
        summary
            .push_str("- 83 PDFs had no embedded ZUGFeRD XML and are excluded from the export.\n");

        std::fs::write(out_dir.join("EXPECTED.md"), &summary).unwrap();
        eprintln!("\n[OK] All exports written to {}", out_dir.display());
        eprintln!("[OK] See EXPECTED.md for what to check in DATEV and IDEA");
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
