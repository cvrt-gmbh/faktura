use thiserror::Error;

/// Errors that can occur during invoice construction or processing.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum RechnungError {
    /// One or more validation rules failed.
    #[error("validation failed: {0}")]
    Validation(String),

    /// Builder encountered invalid or missing configuration.
    #[error("builder error: {0}")]
    Builder(String),

    /// Invoice number sequencing error.
    #[error("numbering error: {0}")]
    Numbering(String),

    /// Invoice totals or arithmetic inconsistency.
    #[error("arithmetic error: {0}")]
    Arithmetic(String),

    /// XML generation or parsing error.
    #[error("XML error: {0}")]
    Xml(String),
}

/// A single validation error with field path and message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    /// Dot-separated path to the invalid field (e.g. "seller.address.country_code").
    pub field: String,
    /// Human-readable error description.
    pub message: String,
    /// EN 16931 business rule ID if applicable (e.g. "BR-01").
    pub rule: Option<String>,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(rule) = &self.rule {
            write!(f, "[{}] {}: {}", rule, self.field, self.message)
        } else {
            write!(f, "{}: {}", self.field, self.message)
        }
    }
}

impl ValidationError {
    /// Create a validation error without a rule ID.
    pub fn new(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            rule: None,
        }
    }

    /// Create a validation error with an EN 16931 rule ID.
    pub fn with_rule(
        field: impl Into<String>,
        message: impl Into<String>,
        rule: impl Into<String>,
    ) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            rule: Some(rule.into()),
        }
    }
}
