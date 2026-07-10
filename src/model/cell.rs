//! Core cell types: `CellValue` (flat tagged union) and `Cell`.
//!
//! `CellValue` uses a flat `#[napi(object)]` struct with a `value_type` discriminant
//! string and optional typed fields for each variant. This is the proven pattern from
//! the napi-rs v3 spike — Rust enums with variant data cannot cross the FFI boundary.
//!
//! # Mutation semantics (interior mutability)
//!
//! `Cell` holds `Arc<Mutex<CellInner>>`, so every clone of a `Cell` shares the same
//! underlying state. Calling `ws.getCell('A1').value = x` or
//! `ws.getCell('A1').style = {...}` persists through the `Arc`, even though napi-rs
//! passes the `Cell` by value/clone across the FFI boundary. This matches the pattern
//! used by `Row` and `Column` in `worksheet.rs`.

use std::sync::{Arc, Mutex};

use napi_derive::napi;

use crate::error::ExcelrsError;
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
/// # Write-only variants (v0.5.0)
/// `Hyperlink`, `RichText`, `Merge` are write-only: they can be set via JS and
/// will be written to the XLSX, but calamine does not expose them on the read
/// path so they appear as `Null` when read back (see spec §9.2.1 item 2).
/// A rich text run: a text fragment with optional font formatting.
#[napi(object)]
#[derive(Clone, Debug)]
pub struct RichTextRun {
    /// Text content for this run.
    pub text: String,
    /// Font formatting for this run (optional).
    pub font: Option<crate::model::style::Font>,
}

#[napi(object)]
#[derive(Clone, Debug)]
pub struct CellValue {
    /// Discriminant: "Null" | "Number" | "String" | "Boolean" | "Formula" | "Error"
    /// | "Hyperlink" | "RichText" | "Merge"
    pub value_type: String,
    pub number: Option<f64>,
    pub string: Option<String>,
    pub boolean: Option<bool>,
    pub formula: Option<String>,
    pub error_value: Option<String>,
    // -- write-only variants (v0.5.0) --
    /// URL for hyperlink (write-only, Null on read).
    pub hyperlink: Option<String>,
    /// Display text for hyperlink (write-only, Null on read).
    pub hyperlink_text: Option<String>,
    /// Rich text runs (write-only, Null on read).
    pub rich_text: Option<Vec<RichTextRun>>,
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
            hyperlink: None,
            hyperlink_text: None,
            rich_text: None,
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

    pub fn hyperlink(url: impl Into<String>, text: Option<String>) -> Self {
        CellValue {
            value_type: "Hyperlink".into(),
            hyperlink: Some(url.into()),
            hyperlink_text: text,
            ..Default::default()
        }
    }

    pub fn rich_text(runs: Vec<RichTextRun>) -> Self {
        CellValue {
            value_type: "RichText".into(),
            rich_text: Some(runs),
            ..Default::default()
        }
    }

    /// Validate this cell value. Validates rich-text fonts.
    /// Returns `Ok(self)` if valid, `Err` with `ExcelrsError` otherwise.
    /// This is called by the writer before emitting XML.
    pub fn validate(mut self) -> Result<Self, ExcelrsError> {
        if let Some(ref mut runs) = self.rich_text {
            for run in runs.iter_mut() {
                if let Some(ref mut font) = run.font {
                    font.validate()?;
                }
            }
        }
        Ok(self)
    }
}

// ---------------------------------------------------------------------------
// CellInner (private)
// ---------------------------------------------------------------------------

/// The mutable inner state shared by all clones of a `Cell`.
#[derive(Clone, Debug)]
pub(crate) struct CellInner {
    pub address: String,
    pub row: u32,
    pub col: u32,
    pub value: CellValue,
    pub formula: Option<String>,
    /// Style reference. `None` = Normal (index 0).
    pub style: Option<Style>,
}

// ---------------------------------------------------------------------------
// Cell
// ---------------------------------------------------------------------------

/// A single cell in a worksheet.
///
/// Holds `Arc<Mutex<CellInner>>` so that every clone shares the same underlying
/// state — value and style mutations made through any handle persist to the
/// worksheet's internal model.
#[napi]
#[derive(Clone, Debug)]
pub struct Cell {
    inner: Arc<Mutex<CellInner>>,
}

#[napi]
impl Cell {
    #[napi(constructor)]
    pub fn new(address: String, row: u32, col: u32) -> Self {
        Cell {
            inner: Arc::new(Mutex::new(CellInner {
                address,
                row,
                col,
                value: CellValue::default(),
                formula: None,
                style: None,
            })),
        }
    }

    // -- value (getter + setter) --

    #[napi(getter)]
    pub fn value(&self) -> CellValue {
        self.inner.lock().expect("Cell lock poisoned").value.clone()
    }

    /// Accepts JS primitives via serde_json::Value auto-conversion (napi v3 serde-json feature).
    /// Dispatches to the correct CellValue variant based on the JSON value type.
    #[napi(setter)]
    pub fn set_value(&mut self, val: serde_json::Value) {
        let mut inner = self.inner.lock().expect("Cell lock poisoned");
        inner.value = match val {
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
        self.inner.lock().expect("Cell lock poisoned").address.clone()
    }

    // -- row (read-only) --

    #[napi(getter)]
    pub fn row(&self) -> u32 {
        self.inner.lock().expect("Cell lock poisoned").row
    }

    // -- col (read-only) --

    #[napi(getter)]
    pub fn col(&self) -> u32 {
        self.inner.lock().expect("Cell lock poisoned").col
    }

    // -- formula (read-only) --

    #[napi(getter)]
    pub fn formula(&self) -> Option<String> {
        self.inner.lock().expect("Cell lock poisoned").formula.clone()
    }

    // -- style (getter + setter) --

    /// Returns the cell's style, or `None` if Normal (index 0).
    #[napi(getter)]
    pub fn style(&self) -> Option<Style> {
        self.inner.lock().expect("Cell lock poisoned").style.clone()
    }

    /// Set the cell's style from a JS object. Full-replace semantics
    /// (spec §6.9): assigning a new style replaces the existing one.
    ///
    /// - `null | undefined | {}` → resets to Normal (None).
    /// - Throws `ExcelrsError::InvalidStyle` on validation failure.
    #[napi(setter)]
    pub fn set_style(&mut self, val: serde_json::Value) -> napi::Result<()> {
        let mut inner = self.inner.lock().expect("Cell lock poisoned");
        if val.is_null() {
            inner.style = None;
            return Ok(());
        }
        let style: Style = serde_json::from_value(val).map_err(|e| napi::Error::from_reason(format!("style: {e}")))?;
        if style.is_empty() {
            inner.style = None;
            return Ok(());
        }
        inner.style = Some(style.validate().map_err(|e| napi::Error::from_reason(e.to_string()))?);
        Ok(())
    }
}

impl Cell {
    /// Internal: set the CellValue directly (used by reader, add_row).
    /// Skips the serde_json::Value dispatch for efficiency.
    pub fn set_value_raw(&mut self, value: CellValue) {
        self.inner.lock().expect("Cell lock poisoned").value = value;
    }

    /// Internal: set the style directly (used by reader, set_columns).
    /// Skips the serde_json::Value dispatch.
    pub fn set_style_raw(&mut self, style: Option<Style>) {
        self.inner.lock().expect("Cell lock poisoned").style = style;
    }

    /// Internal: set the formula string (used by reader).
    pub fn set_formula(&mut self, formula: Option<String>) {
        self.inner.lock().expect("Cell lock poisoned").formula = formula;
    }

    /// A cell is "effectively empty" when it has no value, no formula, and
    /// no style — i.e., it was only created by a read-side `getCell` and
    /// never populated. The writer skips these cells to avoid phantom output.
    pub fn is_effectively_empty(&self) -> bool {
        let inner = self.inner.lock().expect("Cell lock poisoned");
        inner.value.value_type == "Null" && inner.formula.is_none() && inner.style.is_none()
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
