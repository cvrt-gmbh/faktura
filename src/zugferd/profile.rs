use crate::core::*;
use crate::xrechnung;

/// ZUGFeRD / Factur-X conformance profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZugferdProfile {
    /// Minimal machine-readable data (no line items).
    Minimum,
    /// Basic data without line items.
    BasicWl,
    /// Basic with line items.
    Basic,
    /// Full EN 16931 European norm (recommended for most use cases).
    EN16931,
    /// Extended profile (beyond EN 16931).
    Extended,
    /// XRechnung profile (German public sector).
    XRechnung,
}

impl ZugferdProfile {
    /// The URN identifier used in the CII XML `GuidelineSpecifiedDocumentContextParameter`.
    pub fn urn(&self) -> &'static str {
        match self {
            Self::Minimum => "urn:factur-x.eu:1p0:minimum",
            Self::BasicWl => "urn:factur-x.eu:1p0:basicwl",
            Self::Basic => "urn:cen.eu:en16931:2017#compliant#urn:factur-x.eu:1p0:basic",
            Self::EN16931 => "urn:cen.eu:en16931:2017",
            Self::Extended => "urn:cen.eu:en16931:2017#conformant#urn:factur-x.eu:1p0:extended",
            Self::XRechnung => xrechnung::XRECHNUNG_CUSTOMIZATION_ID,
        }
    }

    /// The XMP ConformanceLevel value.
    pub fn conformance_level(&self) -> &'static str {
        match self {
            Self::Minimum => "MINIMUM",
            Self::BasicWl => "BASIC WL",
            Self::Basic => "BASIC",
            Self::EN16931 => "EN 16931",
            Self::Extended => "EXTENDED",
            Self::XRechnung => "XRECHNUNG",
        }
    }

    /// The AFRelationship value for the PDF FileSpec.
    pub fn af_relationship(&self) -> &'static str {
        match self {
            Self::Minimum | Self::BasicWl => "Data",
            _ => "Alternative",
        }
    }
}

/// Generate ZUGFeRD/Factur-X CII XML for the given profile.
///
/// This wraps the xrechnung CII generator but substitutes the
/// profile-specific `GuidelineSpecifiedDocumentContextParameter`.
pub fn to_xml(invoice: &Invoice, profile: ZugferdProfile) -> Result<String, RechnungError> {
    let xml = xrechnung::to_cii_xml(invoice)?;

    // Replace the customization ID with the profile-specific URN
    let xml = xml.replace(xrechnung::XRECHNUNG_CUSTOMIZATION_ID, profile.urn());

    Ok(xml)
}
