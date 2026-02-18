//! ZUGFeRD / Factur-X PDF/A-3 embedding and extraction.
//!
//! Generates CII XML for various ZUGFeRD profiles and embeds it
//! into PDF/A-3 files as `factur-x.xml`.
//!
//! # Profiles
//!
//! | Profile | Use case |
//! |---------|----------|
//! | Minimum | Minimal machine-readable data |
//! | BasicWl | Basic without line items |
//! | Basic | Line items without full EN 16931 |
//! | EN16931 | Full European norm (recommended) |
//! | Extended | Beyond EN 16931 |
//! | XRechnung | German public sector |

mod embed;
mod extract;
mod profile;
mod xmp;

pub use embed::embed_in_pdf;
pub use extract::extract_from_pdf;
pub use profile::{ZugferdProfile, to_xml};

/// The embedded XML filename per Factur-X 1.0+ specification.
pub const FACTURX_FILENAME: &str = "factur-x.xml";
