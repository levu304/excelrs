//! Data validation: constraints on cell values per OOXML §18.3.1.
//!
//! Each `<dataValidation>` element in `xl/worksheets/sheet{N}.xml` specifies
//! a cell range (sqref), a type (whole, decimal, list, date, time, textLength, custom),
//! an optional operator (between, notBetween, equal, notEqual, greaterThan, lessThan,
//! greaterThanOrEqual, lessThanOrEqual), and 1–2 formula values. Optional
//! messages (input prompt, error) are shown to the user.
//!
//! Validation is stored per-worksheet; multiple validations for the same sqref
//! are upserted (the last one wins).

use crate::error::ExcelrsError;
use napi_derive::napi;

/// Data validation constraint on a cell range.
#[napi(object)]
#[derive(Clone, Debug)]
pub struct DataValidation {
    /// Cell range reference (e.g. "A1", "A1:B10", "A1 C1:D5").
    pub sqref: String,
    /// Validation type: "whole", "decimal", "list", "date", "time", "textLength", "custom".
    pub r#type: String,
    /// Operator for range-based types: "between", "notBetween", "equal", "notEqual",
    /// "greaterThan", "lessThan", "greaterThanOrEqual", "lessThanOrEqual".
    /// Only used with "whole", "decimal", "date", "time", "textLength".
    pub operator: Option<String>,
    /// First formula value (constraint lower bound, or single value, or list).
    pub formula1: String,
    /// Second formula value (constraint upper bound); only used with "between"/"notBetween".
    pub formula2: Option<String>,
    /// Allow blank cells (default: true). When true, cells in sqref can be empty.
    pub allow_blank: Option<bool>,
    /// Show input prompt message (default: false).
    pub show_input_message: Option<bool>,
    /// Show error message (default: false).
    pub show_error_message: Option<bool>,
    /// Input prompt text.
    pub prompt: Option<String>,
    /// Input prompt title.
    pub prompt_title: Option<String>,
    /// Error message text.
    pub error: Option<String>,
    /// Error message title.
    pub error_title: Option<String>,
    /// Error message type: "information", "warning", "stop".
    pub error_style: Option<String>,
}

impl DataValidation {
    /// Validate the DataValidation instance.
    /// Returns an error if:
    /// - `sqref` is empty or whitespace
    /// - `r#type` is not one of the 7 valid types
    pub fn validate(&self) -> Result<(), ExcelrsError> {
        if self.sqref.trim().is_empty() {
            return Err(ExcelrsError::Parse(
                "DataValidation.sqref must not be empty".to_string(),
            ));
        }

        let valid_types = ["whole", "decimal", "list", "date", "time", "textLength", "custom"];
        if !valid_types.contains(&self.r#type.as_str()) {
            return Err(ExcelrsError::Parse(format!(
                "DataValidation.type '{}' is invalid; must be one of: {}",
                self.r#type,
                valid_types.join(", ")
            )));
        }

        // Validate operator if present
        if let Some(ref op) = self.operator {
            let valid_operators = [
                "between",
                "notBetween",
                "equal",
                "notEqual",
                "greaterThan",
                "lessThan",
                "greaterThanOrEqual",
                "lessThanOrEqual",
            ];
            if !valid_operators.contains(&op.as_str()) {
                return Err(ExcelrsError::Parse(format!(
                    "DataValidation.operator '{}' is invalid; must be one of: {}",
                    op,
                    valid_operators.join(", ")
                )));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_empty_sqref_errors() {
        let dv = DataValidation {
            sqref: "".to_string(),
            r#type: "whole".to_string(),
            operator: None,
            formula1: "1".to_string(),
            formula2: None,
            allow_blank: None,
            show_input_message: None,
            show_error_message: None,
            prompt: None,
            prompt_title: None,
            error: None,
            error_title: None,
            error_style: None,
        };
        assert!(dv.validate().is_err());
    }

    #[test]
    fn test_validate_whitespace_sqref_errors() {
        let dv = DataValidation {
            sqref: "   ".to_string(),
            r#type: "whole".to_string(),
            operator: None,
            formula1: "1".to_string(),
            formula2: None,
            allow_blank: None,
            show_input_message: None,
            show_error_message: None,
            prompt: None,
            prompt_title: None,
            error: None,
            error_title: None,
            error_style: None,
        };
        assert!(dv.validate().is_err());
    }

    #[test]
    fn test_validate_invalid_type_errors() {
        let dv = DataValidation {
            sqref: "A1".to_string(),
            r#type: "invalid_type".to_string(),
            operator: None,
            formula1: "1".to_string(),
            formula2: None,
            allow_blank: None,
            show_input_message: None,
            show_error_message: None,
            prompt: None,
            prompt_title: None,
            error: None,
            error_title: None,
            error_style: None,
        };
        assert!(dv.validate().is_err());
    }

    #[test]
    fn test_validate_invalid_operator_errors() {
        let dv = DataValidation {
            sqref: "A1".to_string(),
            r#type: "whole".to_string(),
            operator: Some("invalid_op".to_string()),
            formula1: "1".to_string(),
            formula2: None,
            allow_blank: None,
            show_input_message: None,
            show_error_message: None,
            prompt: None,
            prompt_title: None,
            error: None,
            error_title: None,
            error_style: None,
        };
        assert!(dv.validate().is_err());
    }

    #[test]
    fn test_validate_valid_type_whole() {
        let dv = DataValidation {
            sqref: "A1".to_string(),
            r#type: "whole".to_string(),
            operator: Some("between".to_string()),
            formula1: "1".to_string(),
            formula2: Some("10".to_string()),
            allow_blank: Some(true),
            show_input_message: Some(false),
            show_error_message: Some(true),
            prompt: Some("Enter a number".to_string()),
            prompt_title: Some("Number".to_string()),
            error: Some("Must be 1-10".to_string()),
            error_title: Some("Invalid".to_string()),
            error_style: Some("stop".to_string()),
        };
        assert!(dv.validate().is_ok());
    }

    #[test]
    fn test_validate_valid_type_list() {
        let dv = DataValidation {
            sqref: "B1:B100".to_string(),
            r#type: "list".to_string(),
            operator: None,
            formula1: "Red,Green,Blue".to_string(),
            formula2: None,
            allow_blank: None,
            show_input_message: None,
            show_error_message: None,
            prompt: None,
            prompt_title: None,
            error: None,
            error_title: None,
            error_style: None,
        };
        assert!(dv.validate().is_ok());
    }

    #[test]
    fn test_validate_all_valid_types() {
        let types = vec!["whole", "decimal", "list", "date", "time", "textLength", "custom"];
        for type_str in types {
            let dv = DataValidation {
                sqref: "A1".to_string(),
                r#type: type_str.to_string(),
                operator: None,
                formula1: "value".to_string(),
                formula2: None,
                allow_blank: None,
                show_input_message: None,
                show_error_message: None,
                prompt: None,
                prompt_title: None,
                error: None,
                error_title: None,
                error_style: None,
            };
            assert!(dv.validate().is_ok(), "type '{}' should be valid", type_str);
        }
    }

    #[test]
    fn test_validate_all_valid_operators() {
        let operators = vec![
            "between",
            "notBetween",
            "equal",
            "notEqual",
            "greaterThan",
            "lessThan",
            "greaterThanOrEqual",
            "lessThanOrEqual",
        ];
        for op in operators {
            let dv = DataValidation {
                sqref: "A1".to_string(),
                r#type: "whole".to_string(),
                operator: Some(op.to_string()),
                formula1: "1".to_string(),
                formula2: None,
                allow_blank: None,
                show_input_message: None,
                show_error_message: None,
                prompt: None,
                prompt_title: None,
                error: None,
                error_title: None,
                error_style: None,
            };
            assert!(dv.validate().is_ok(), "operator '{}' should be valid", op);
        }
    }
}
