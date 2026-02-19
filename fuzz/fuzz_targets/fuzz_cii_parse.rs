#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // Must not panic — errors are fine, panics are bugs.
        let _ = faktura::xrechnung::from_cii_xml(s);
    }
});
