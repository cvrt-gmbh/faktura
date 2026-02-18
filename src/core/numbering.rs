use chrono::{Datelike, NaiveDate};

use super::error::RechnungError;

/// Gapless invoice number sequence generator.
///
/// Generates invoice numbers in the format `{prefix}{year}-{sequential}`,
/// e.g. "RE-2024-001", "RE-2024-002", etc.
///
/// German tax law (§14 UStG, GoBD) requires gapless, sequential numbering.
/// This struct tracks the last issued number and ensures no gaps.
#[derive(Debug, Clone)]
pub struct InvoiceNumberSequence {
    prefix: String,
    year: i32,
    next_number: u64,
    zero_pad: usize,
}

impl InvoiceNumberSequence {
    /// Create a new sequence starting at 1.
    pub fn new(prefix: impl Into<String>, year: i32) -> Self {
        Self {
            prefix: prefix.into(),
            year,
            next_number: 1,
            zero_pad: 3,
        }
    }

    /// Create a sequence continuing from a given number.
    pub fn starting_at(prefix: impl Into<String>, year: i32, next_number: u64) -> Self {
        Self {
            prefix: prefix.into(),
            year,
            next_number,
            zero_pad: 3,
        }
    }

    /// Set zero-padding width (default: 3, so "001").
    pub fn with_padding(mut self, width: usize) -> Self {
        self.zero_pad = width;
        self
    }

    /// Generate the next invoice number.
    pub fn next_number(&mut self) -> String {
        let num = self.next_number;
        self.next_number += 1;
        format!(
            "{}{}-{:0>width$}",
            self.prefix,
            self.year,
            num,
            width = self.zero_pad
        )
    }

    /// Preview the next number without consuming it.
    pub fn peek(&self) -> String {
        format!(
            "{}{}-{:0>width$}",
            self.prefix,
            self.year,
            self.next_number,
            width = self.zero_pad
        )
    }

    /// Get the current year of the sequence.
    pub fn year(&self) -> i32 {
        self.year
    }

    /// Get the next number that will be issued (without prefix/formatting).
    pub fn next_raw(&self) -> u64 {
        self.next_number
    }

    /// Advance to a new year, resetting the counter to 1.
    pub fn advance_year(&mut self, new_year: i32) -> Result<(), RechnungError> {
        if new_year <= self.year {
            return Err(RechnungError::Numbering(format!(
                "new year {new_year} must be greater than current year {}",
                self.year
            )));
        }
        self.year = new_year;
        self.next_number = 1;
        Ok(())
    }

    /// Auto-advance year if the given date is in a new year.
    /// Returns true if the year was advanced.
    pub fn auto_advance(&mut self, date: NaiveDate) -> bool {
        let date_year = date.year();
        if date_year > self.year {
            self.year = date_year;
            self.next_number = 1;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sequential_numbering() {
        let mut seq = InvoiceNumberSequence::new("RE-", 2024);
        assert_eq!(seq.next_number(), "RE-2024-001");
        assert_eq!(seq.next_number(), "RE-2024-002");
        assert_eq!(seq.next_number(), "RE-2024-003");
    }

    #[test]
    fn peek_does_not_consume() {
        let mut seq = InvoiceNumberSequence::new("RE-", 2024);
        assert_eq!(seq.peek(), "RE-2024-001");
        assert_eq!(seq.peek(), "RE-2024-001");
        assert_eq!(seq.next_number(), "RE-2024-001");
        assert_eq!(seq.peek(), "RE-2024-002");
    }

    #[test]
    fn starting_at() {
        let mut seq = InvoiceNumberSequence::starting_at("INV-", 2024, 42);
        assert_eq!(seq.next_number(), "INV-2024-042");
        assert_eq!(seq.next_number(), "INV-2024-043");
    }

    #[test]
    fn custom_padding() {
        let mut seq = InvoiceNumberSequence::new("R", 2024).with_padding(5);
        assert_eq!(seq.next_number(), "R2024-00001");
    }

    #[test]
    fn year_advance() {
        let mut seq = InvoiceNumberSequence::new("RE-", 2024);
        seq.next_number(); // RE-2024-001
        seq.next_number(); // RE-2024-002
        seq.advance_year(2025).unwrap();
        assert_eq!(seq.next_number(), "RE-2025-001");
    }

    #[test]
    fn year_advance_rejects_past() {
        let mut seq = InvoiceNumberSequence::new("RE-", 2024);
        assert!(seq.advance_year(2023).is_err());
        assert!(seq.advance_year(2024).is_err());
    }

    #[test]
    fn auto_advance_year() {
        let mut seq = InvoiceNumberSequence::new("RE-", 2024);
        seq.next_number(); // RE-2024-001

        let jan_2025 = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        assert!(seq.auto_advance(jan_2025));
        assert_eq!(seq.next_number(), "RE-2025-001");

        // Same year doesn't advance
        let feb_2025 = NaiveDate::from_ymd_opt(2025, 2, 1).unwrap();
        assert!(!seq.auto_advance(feb_2025));
        assert_eq!(seq.next_number(), "RE-2025-002");
    }
}
