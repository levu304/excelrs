//! Worksheet protection flags per OOXML CT_SheetProtection.
//!
//! Booleans follow OOXML convention: present="1"/"true" → true, absent or "0"/"false" → false.
//! excelrs exposes these as an optional object (matching ExcelJS `ws.protection`).

use napi_derive::napi;

/// Sheet protection flags. Each flag controls whether the user may perform
/// that action when the sheet is protected. A `None` value means the flag
/// was not in the file; absence of the element means the sheet is unprotected.
#[napi(object)]
#[derive(Clone, Debug, Default)]
pub struct SheetProtection {
    pub locked: Option<bool>,
    pub auto_filter: Option<bool>,
    pub delete_columns: Option<bool>,
    pub delete_rows: Option<bool>,
    pub format_cells: Option<bool>,
    pub format_columns: Option<bool>,
    pub format_rows: Option<bool>,
    pub insert_columns: Option<bool>,
    pub insert_hyperlinks: Option<bool>,
    pub insert_rows: Option<bool>,
    pub pivot_tables: Option<bool>,
    pub select_locked_cells: Option<bool>,
    pub select_unlocked_cells: Option<bool>,
    pub sort: Option<bool>,
    /// The password hash (salted SHA-1), if set. Not exposed to JS by excelrs.
    pub password_hash: Option<String>,
    /// The password salt, if set. Not exposed to JS by excelrs.
    pub salt_value: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sheet_protection_default() {
        let sp = SheetProtection::default();
        assert!(sp.locked.is_none());
        assert!(sp.select_locked_cells.is_none());
    }

    #[test]
    fn test_sheet_protection_some_flags() {
        let sp = SheetProtection {
            locked: Some(true),
            select_locked_cells: Some(true),
            format_cells: Some(false),
            ..Default::default()
        };
        assert_eq!(sp.locked, Some(true));
        assert_eq!(sp.format_cells, Some(false));
    }
}
