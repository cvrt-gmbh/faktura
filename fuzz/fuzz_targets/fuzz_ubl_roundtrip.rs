#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // Parse → serialize → parse must not panic at any step.
        if let Ok(invoice) = faktura::xrechnung::from_ubl_xml(s) {
            if let Ok(xml2) = faktura::xrechnung::to_ubl_xml(&invoice) {
                let _ = faktura::xrechnung::from_ubl_xml(&xml2);
            }
        }
    }
});
