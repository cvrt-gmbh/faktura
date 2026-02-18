use thiserror::Error;

/// Errors that can occur during invoice construction or processing.
#[derive(Debug, Error)]
pub enum RechnungError {
    #[error("validation failed: {0}")]
    Validation(String),

    #[error("builder error: {0}")]
    Builder(String),

    #[error("numbering error: {0}")]
    Numbering(String),

    #[error("arithmetic error: {0}")]
    Arithmetic(String),
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
    pub fn new(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            rule: None,
        }
    }

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
