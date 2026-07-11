//! Defined name (named range) model type.
//!
//! A defined name associates a symbolic name with a formula, range, or constant
//! value.  Names can be workbook-scoped (usable from any sheet) or sheet-scoped
//! (usable only within the owning sheet).  The value is stored as the raw OOXML
//! text — no formula evaluation is performed.

use napi_derive::napi;

/// A named range or defined name in the workbook.
///
/// - `name`: The defined name identifier (e.g. `"TaxRate"`, `"MyRange"`).
/// - `value`: The raw OOXML text content (e.g. `"Sheet1!$A$1"`, `"0.08"`).
/// - `sheet`: For sheet-scoped names (`localSheetId` resolved on read), the
///   resolved sheet name.  `None` for workbook-global names.
#[napi(object)]
#[derive(Debug, Clone, Default)]
pub struct DefinedName {
    pub name: String,
    pub value: String,
    pub sheet: Option<String>,
}

impl DefinedName {
    /// Create a workbook-scoped defined name.
    pub fn global(name: impl Into<String>, value: impl Into<String>) -> Self {
        DefinedName {
            name: name.into(),
            value: value.into(),
            sheet: None,
        }
    }

    /// Create a sheet-scoped defined name.
    pub fn sheet_scoped(name: impl Into<String>, value: impl Into<String>, sheet: impl Into<String>) -> Self {
        DefinedName {
            name: name.into(),
            value: value.into(),
            sheet: Some(sheet.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defined_name_global() {
        let dn = DefinedName::global("TaxRate", "0.08");
        assert_eq!(dn.name, "TaxRate");
        assert_eq!(dn.value, "0.08");
        assert!(dn.sheet.is_none());
    }

    #[test]
    fn test_defined_name_sheet_scoped() {
        let dn = DefinedName::sheet_scoped("LocalRef", "$A$1:$B$10", "Sheet1");
        assert_eq!(dn.name, "LocalRef");
        assert_eq!(dn.value, "$A$1:$B$10");
        assert_eq!(dn.sheet, Some("Sheet1".into()));
    }
}
