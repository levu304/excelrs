//! Core cell types: `CellValue` (flat tagged union) and `Cell`.
//!
//! `CellValue` uses a flat `#[napi(object)]` struct with a `value_type` discriminant
//! string and optional typed fields for each variant. This is the proven pattern from
//! the napi-rs v3 spike — Rust enums with variant data cannot cross the FFI boundary.
//!
//! # Mutation semantics (clone-on-read)
//! napi-rs passes structs by value/clone across FFI. Calling `ws.getCell('A1')` returns
//! a clone of the internal cell. Mutating the returned object has **no effect** on the
//! worksheet's internal state. This is a fundamental constraint of napi-rs v3.
//! v0.2 will explore interior mutability (`Arc<Mutex<>>`) for chainable mutation.

use napi_derive::napi;

use crate::model::style::Style;
use crate::types;

// ---------------------------------------------------------------------------
// CellValue
// ---------------------------------------------------------------------------

/// Flat tagged union for cell values across the FFI boundary.
///
/// Discriminant is `value_type`:
/// - `"Null"` — no value (default)
/// - `"Number"` — numeric value (field: `number`)
/// - `"String"` — text value (field: `string`)
/// - `"Boolean"` — boolean value (field: `boolean`)
/// - `"Formula"` — formula string (field: `formula`; preserved, not evaluated)
/// - `"Error"` — error value (field: `error_value`)
///
/// # Variants deferred from v0.1
/// `Hyperlink`, `RichText`, `SharedString`, and `Merge` are not included because
/// calamine does not expose them on the read path. They will be reintroduced in v0.2.
#[napi(object)]
#[derive(Clone, Debug)]
pub struct CellValue {
    /// Discriminant: "Null" | "Number" | "String" | "Boolean" | "Formula" | "Error"
    pub value_type: String,
    pub number: Option<f64>,
    pub string: Option<String>,
    pub boolean: Option<bool>,
    pub formula: Option<String>,
    pub error_value: Option<String>,
}

impl Default for CellValue {
    fn default() -> Self {
        CellValue {
            value_type: "Null".into(),
            number: None,
            string: None,
            boolean: None,
            formula: None,
            error_value: None,
        }
    }
}

/// Helper constructors for common cell value variants (used by tests and reader).
impl CellValue {
    pub fn number(n: f64) -> Self {
        CellValue {
            value_type: "Number".into(),
            number: Some(n),
            ..Default::default()
        }
    }

    pub fn string(s: impl Into<String>) -> Self {
        CellValue {
            value_type: "String".into(),
            string: Some(s.into()),
            ..Default::default()
        }
    }

    pub fn boolean(b: bool) -> Self {
        CellValue {
            value_type: "Boolean".into(),
            boolean: Some(b),
            ..Default::default()
        }
    }

    pub fn formula(f: impl Into<String>) -> Self {
        CellValue {
            value_type: "Formula".into(),
            formula: Some(f.into()),
            ..Default::default()
        }
    }
}

// ---------------------------------------------------------------------------
// Cell
// ---------------------------------------------------------------------------

/// A single cell in a worksheet.
///
/// Holds its address (e.g., "A1"), position (1-indexed row/col), value, formula,
/// and style reference. Cells are **value types** across the FFI boundary — see
/// the module-level doc for mutation semantics.
#[napi]
#[derive(Clone, Debug)]
pub struct Cell {
    address: String,
    row: u32,
    col: u32,
    value: CellValue,
    formula: Option<String>,
    /// Style reference. `None` = Normal (index 0). Write-only in v0.2.0.
    /// Reading a styled `.xlsx` yields `None` (style read deferred to v0.3.0).
    style: Option<Style>,
}

#[napi]
impl Cell {
    #[napi(constructor)]
    pub fn new(address: String, row: u32, col: u32) -> Self {
        Cell {
            address,
            row,
            col,
            value: CellValue::default(),
            formula: None,
            style: None,
        }
    }

    // -- value (getter + setter) --

    #[napi(getter)]
    pub fn value(&self) -> CellValue {
        self.value.clone()
    }

    /// Accepts JS primitives via serde_json::Value auto-conversion (napi v3 serde-json feature).
    /// Dispatches to the correct CellValue variant based on the JSON value type.
    #[napi(setter)]
    pub fn set_value(&mut self, val: serde_json::Value) {
        self.value = match val {
            serde_json::Value::Number(n) => CellValue {
                value_type: "Number".into(),
                number: n.as_f64(),
                ..Default::default()
            },
            serde_json::Value::String(s) => CellValue {
                value_type: "String".into(),
                string: Some(s),
                ..Default::default()
            },
            serde_json::Value::Bool(b) => CellValue {
                value_type: "Boolean".into(),
                boolean: Some(b),
                ..Default::default()
            },
            // Arrays, objects, and unrecognized values become Null
            _ => CellValue::default(),
        };
    }

    // -- address (read-only) --

    #[napi(getter)]
    pub fn address(&self) -> String {
        self.address.clone()
    }

    // -- row (read-only) --

    #[napi(getter)]
    pub fn row(&self) -> u32 {
        self.row
    }

    // -- col (read-only) --

    #[napi(getter)]
    pub fn col(&self) -> u32 {
        self.col
    }

    // -- formula (read-only) --

    #[napi(getter)]
    pub fn formula(&self) -> Option<String> {
        self.formula.clone()
    }

    // -- style (getter + setter) --

    /// Returns the cell's style, or `None` if Normal (index 0).
    #[napi(getter)]
    pub fn style(&self) -> Option<Style> {
        self.style.clone()
    }

    /// Set the cell's style from a JS object. Full-replace semantics
    /// (spec §6.9): assigning a new style replaces the existing one.
    ///
    /// - `null | undefined | {}` → resets to Normal (None).
    /// - Throws `ExcelrsError::InvalidStyle` on validation failure.
    #[napi(setter)]
    pub fn set_style(&mut self, val: serde_json::Value) -> napi::Result<()> {
        if val.is_null() {
            self.style = None;
            return Ok(());
        }
        let style: Style = serde_json::from_value(val).map_err(|e| napi::Error::from_reason(format!("style: {e}")))?;
        if style.is_empty() {
            self.style = None;
            return Ok(());
        }
        self.style = Some(style.validate().map_err(|e| napi::Error::from_reason(e.to_string()))?);
        Ok(())
    }
}

impl Cell {
    /// Internal: set the CellValue directly (used by reader, add_row).
    /// Skips the serde_json::Value dispatch for efficiency.
    pub fn set_value_raw(&mut self, value: CellValue) {
        self.value = value;
    }

    /// Internal: set the style directly (used by reader, set_columns).
    /// Skips the serde_json::Value dispatch.
    pub fn set_style_raw(&mut self, style: Option<Style>) {
        self.style = style;
    }

    /// Internal: set the formula string (used by reader).
    pub fn set_formula(&mut self, formula: Option<String>) {
        self.formula = formula;
    }

    /// Compute the A1 address from (col, row). Used during row/cell creation.
    pub fn compute_address(row: u32, col: u32) -> String {
        types::address_to_string(col, row).unwrap_or_else(|_| format!("R{row}C{col}"))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cell_new() {
        let cell = Cell::new("A1".into(), 1, 1);
        assert_eq!(cell.address(), "A1");
        assert_eq!(cell.row(), 1);
        assert_eq!(cell.col(), 1);
        assert_eq!(cell.value().value_type, "Null");
        assert!(cell.formula().is_none());
    }

    #[test]
    fn test_cell_set_value_number() {
        let mut cell = Cell::new("B2".into(), 2, 2);
        cell.set_value(serde_json::json!(42));
        let v = cell.value();
        assert_eq!(v.value_type, "Number");
        assert_eq!(v.number, Some(42.0));
    }

    #[test]
    fn test_cell_set_value_string() {
        let mut cell = Cell::new("C3".into(), 3, 3);
        cell.set_value(serde_json::json!("hello"));
        let v = cell.value();
        assert_eq!(v.value_type, "String");
        assert_eq!(v.string, Some("hello".into()));
    }

    #[test]
    fn test_cell_set_value_bool() {
        let mut cell = Cell::new("D4".into(), 4, 4);
        cell.set_value(serde_json::json!(true));
        let v = cell.value();
        assert_eq!(v.value_type, "Boolean");
        assert_eq!(v.boolean, Some(true));
    }

    #[test]
    fn test_cell_set_value_null() {
        let mut cell = Cell::new("E5".into(), 5, 5);
        cell.set_value(serde_json::json!("hello"));
        cell.set_value(serde_json::Value::Null);
        let v = cell.value();
        assert_eq!(v.value_type, "Null");
    }

    #[test]
    fn test_cell_compute_address() {
        assert_eq!(Cell::compute_address(1, 1), "A1");
        assert_eq!(Cell::compute_address(42, 27), "AA42");
        assert_eq!(Cell::compute_address(1048576, 16384), "XFD1048576");
    }

    #[test]
    fn test_set_style_raw_sets_style_field() {
        use crate::model::style::{Font, Style};

        let mut cell = Cell::new("A1".into(), 1, 1);
        let style = Style {
            font: Some(Font {
                bold: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        };
        cell.set_style_raw(Some(style));
        assert!(cell.style().is_some());
        assert_eq!(cell.style().unwrap().font.unwrap().bold, Some(true));

        // Clear with None
        cell.set_style_raw(None);
        assert!(cell.style().is_none());
    }
}
