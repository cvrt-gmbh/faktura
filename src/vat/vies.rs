//! EU VIES REST API client for VAT number validation.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Result of a VIES VAT number check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViesResult {
    /// Whether the VAT number is currently valid.
    pub valid: bool,
    /// Date of the request (YYYY-MM-DD).
    pub request_date: Option<String>,
    /// Registered company name (if available).
    pub name: Option<String>,
    /// Registered address (if available).
    pub address: Option<String>,
}

/// Error from the VIES API.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum ViesError {
    /// Network or HTTP error.
    Network(String),
    /// The VIES API returned an error (e.g. member state unavailable).
    ApiError(String),
    /// Failed to parse the response.
    ParseError(String),
}

impl fmt::Display for ViesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Network(e) => write!(f, "VIES network error: {e}"),
            Self::ApiError(e) => write!(f, "VIES API error: {e}"),
            Self::ParseError(e) => write!(f, "VIES parse error: {e}"),
        }
    }
}

impl std::error::Error for ViesError {}

const VIES_URL: &str = "https://ec.europa.eu/taxation_customs/vies/rest-api/check-vat-number";

/// VIES API response structure.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ViesApiResponse {
    valid: Option<bool>,
    request_date: Option<String>,
    name: Option<String>,
    address: Option<String>,
    // Error fields
    error_wrappers: Option<Vec<ViesErrorWrapper>>,
}

#[derive(Debug, Deserialize)]
struct ViesErrorWrapper {
    error: Option<String>,
    message: Option<String>,
}

/// VIES API request body.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ViesRequest {
    country_code: String,
    vat_number: String,
}

/// Check a VAT number against the EU VIES API.
///
/// `country_code` is the 2-letter ISO code (e.g. "DE").
/// `vat_number` is the number part without the country prefix.
///
/// This function is async and requires network access.
/// The VIES API has no authentication — it is a free public service.
///
/// # Errors
///
/// Returns `ViesError::Network` on connection issues,
/// `ViesError::ApiError` if a member state is unavailable,
/// `ViesError::ParseError` on unexpected response formats.
pub async fn check_vies(country_code: &str, vat_number: &str) -> Result<ViesResult, ViesError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| ViesError::Network(e.to_string()))?;

    let req = ViesRequest {
        country_code: country_code.to_uppercase(),
        vat_number: vat_number.to_string(),
    };

    let resp = client
        .post(VIES_URL)
        .json(&req)
        .send()
        .await
        .map_err(|e| ViesError::Network(e.to_string()))?;

    let status = resp.status();
    let body = resp
        .text()
        .await
        .map_err(|e| ViesError::Network(e.to_string()))?;

    if !status.is_success() {
        return Err(ViesError::ApiError(format!("HTTP {status}: {body}")));
    }

    let api_resp: ViesApiResponse = serde_json::from_str(&body)
        .map_err(|e: serde_json::Error| ViesError::ParseError(e.to_string()))?;

    // Check for API-level errors
    if let Some(ref errors) = api_resp.error_wrappers {
        if let Some(err) = errors.first() {
            let msg = err
                .message
                .clone()
                .or_else(|| err.error.clone())
                .unwrap_or_else(|| "unknown error".into());
            return Err(ViesError::ApiError(msg));
        }
    }

    Ok(ViesResult {
        valid: api_resp.valid.unwrap_or(false),
        request_date: api_resp.request_date,
        name: api_resp
            .name
            .filter(|n: &String| n != "---" && !n.is_empty()),
        address: api_resp
            .address
            .filter(|a: &String| a != "---" && !a.is_empty()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vies_url_is_https() {
        assert!(VIES_URL.starts_with("https://"));
    }

    #[test]
    fn vies_request_serialization() {
        let req = ViesRequest {
            country_code: "DE".into(),
            vat_number: "123456789".into(),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"countryCode\":\"DE\""));
        assert!(json.contains("\"vatNumber\":\"123456789\""));
    }

    #[test]
    fn vies_result_deserialization() {
        let json = r#"{"valid":true,"requestDate":"2024-01-15","name":"ACME GMBH","address":"MUSTERSTR 1\n10115 BERLIN"}"#;
        let resp: ViesApiResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.valid, Some(true));
        assert_eq!(resp.name.as_deref(), Some("ACME GMBH"));
    }

    #[test]
    fn vies_result_filters_dashes() {
        let result = ViesResult {
            valid: true,
            request_date: Some("2024-01-15".into()),
            name: Some("---".into()),
            address: Some("---".into()),
        };
        // The --- filtering happens in check_vies(), but we test the struct
        let filtered_name = result.name.filter(|n| n != "---" && !n.is_empty());
        assert!(filtered_name.is_none());
    }
}
