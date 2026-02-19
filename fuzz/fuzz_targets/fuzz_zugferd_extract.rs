#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Arbitrary bytes as PDF input — must not panic.
    let _ = faktura::zugferd::extract_from_pdf(data);
});
