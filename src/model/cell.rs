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

use napi::bindgen_prelude::{FromNapiValue, JsValue};
use napi::Env;
use napi_derive::napi;

use crate::error::ExcelrsError;
use crate::model::comment::CellComment;
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
    /// Excel serial date value (days since 1899-12-30; fractional part = time of day).
    /// Exposed as `dateSerial` on the JS `CellValue` object for round-trip support.
    pub date_serial: Option<f64>,
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
            date_serial: None,
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

    /// Build a `Date` cell value from an Excel serial number (days since
    /// 1899-12-30; fractional part = time of day). The serial is preserved on
    /// round-trip; the public `Cell.value` getter surfaces it as a JS `Date`.
    pub fn date(serial: f64) -> Self {
        CellValue {
            value_type: "Date".into(),
            date_serial: Some(serial),
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
    /// Cell comment / note (v1.0.0). `None` = no comment.
    pub comment: Option<CellComment>,
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
                comment: None,
            })),
        }
    }

    // -- value (getter + setter) --

    #[napi(getter)]
    pub fn value(&self) -> napi::Result<CellValue> {
        let inner = self.inner.lock().expect("Cell lock poisoned");
        let cv = &inner.value;
        if cv.value_type == "Date" {
            Ok(CellValue {
                value_type: "Date".into(),
                date_serial: cv.date_serial,
                ..Default::default()
            })
        } else {
            Ok(cv.clone())
        }
    }

    // -- date (read-only) --

    /// Returns a JS `Date` for Date-type cells, or `null` otherwise.
    #[napi(getter)]
    pub fn date(&self, env: Env) -> napi::Result<Option<napi::JsDate<'static>>> {
        let inner = self.inner.lock().expect("Cell lock poisoned");
        let cv = &inner.value;
        if cv.value_type == "Date" {
            let serial = cv
                .date_serial
                .ok_or_else(|| napi::Error::from_reason("Date cell missing serial"))?;
            let ms = serial_to_millis(serial) as f64;
            let d = env.create_date(ms)?;
            // SAFETY: `JsDate` only wraps a raw `napi_value`; its lifetime marker is
            // nominal. The underlying JS value is valid for the environment's
            // lifetime and is converted to a `napi_value` immediately by the
            // generated wrapper, so extending the lifetime is sound here.
            let d: napi::JsDate<'static> = unsafe { std::mem::transmute(d) };
            Ok(Some(d))
        } else {
            Ok(None)
        }
    }

    /// Accepts JS primitives and CellValue objects.
    ///
    /// Three-path dispatch:
    /// 1. Raw JS `Date` → serial (for `cell.value = new Date(...)`)
    /// 2. `CellValue` object / other objects → `Null` (round-trip via object is not supported)
    /// 3. `serde_json::Value` fallback (Number, String, Bool, Null)
    #[napi(setter)]
    pub fn set_value(&mut self, val: napi::Unknown) -> napi::Result<()> {
        let raw = val.value();
        let raw_env = raw.env;
        let raw_val = raw.value;

        // Path 1 — Raw JS Date
        if let Ok(ms) = unsafe { napi::JsDate::from_napi_value(raw_env, raw_val) }.and_then(|d| d.value_of()) {
            let mut inner = self.inner.lock().expect("Cell lock poisoned");
            inner.value = CellValue::date(millis_to_serial(ms));
            return Ok(());
        }

        // ponytail: CellValue-object round-trip (cell.value = cell.value for Date
        // cells) not handled — use cell.value = new Date(...) pattern instead.
        // serde_json handles null/undefined → Null, numbers/strings/bools correctly.
        // Objects (including CellValue literals) become Null (pre-existing).
        let json = unsafe { serde_json::Value::from_napi_value(raw_env, raw_val)? };
        let mut inner = self.inner.lock().expect("Cell lock poisoned");
        inner.value = match json {
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
            _ => CellValue::default(),
        };
        Ok(())
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

    // -- comment / note (v1.0.0) --
    /// Convenience getter for the comment text (ExcelJS `cell.note`).
    /// Returns `None` when the cell has no comment.
    #[napi(getter)]
    pub fn note(&self) -> Option<String> {
        self.inner
            .lock()
            .expect("Cell lock poisoned")
            .comment
            .as_ref()
            .map(|c| c.text.clone())
    }

    /// Convenience setter for the comment text (ExcelJS `cell.note = "..."`).
    /// Preserves any existing author.
    #[napi(setter)]
    pub fn set_note(&mut self, text: String) {
        let mut inner = self.inner.lock().expect("Cell lock poisoned");
        let author = inner.comment.as_ref().and_then(|c| c.author.clone());
        inner.comment = Some(CellComment { text, author });
    }

    /// Full comment accessor (text + author).
    #[napi(getter)]
    pub fn comment(&self) -> Option<CellComment> {
        self.inner.lock().expect("Cell lock poisoned").comment.clone()
    }

    /// Full comment setter (text + author).
    #[napi(setter)]
    pub fn set_comment(&mut self, c: Option<CellComment>) {
        self.inner.lock().expect("Cell lock poisoned").comment = c;
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

    /// Internal: return the raw `CellValue` (a `Date` cell exposes the serial,
    /// not a JS `Date`). Used by the writer and tests.
    pub fn value_raw(&self) -> CellValue {
        self.inner.lock().expect("Cell lock poisoned").value.clone()
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
// Date helpers (v0.13.0)
// ---------------------------------------------------------------------------

/// Excel's date epoch (1899-12-30) expressed as an Excel serial number.
/// Unix epoch 1970-01-01 == serial 25569.0.
const EXCEL_EPOCH_SERIAL: f64 = 25569.0;

/// Convert an Excel serial number to Unix epoch milliseconds (UTC interpretation).
pub fn serial_to_millis(serial: f64) -> i64 {
    ((serial - EXCEL_EPOCH_SERIAL) * 86_400_000.0).round() as i64
}

/// Convert Unix epoch milliseconds to an Excel serial number.
pub fn millis_to_serial(ms: f64) -> f64 {
    ms / 86_400_000.0 + EXCEL_EPOCH_SERIAL
}

/// Choose a default date number format for a serial: a non-zero time component
/// gets the date-time format, otherwise the date-only format.
pub fn date_format_for_serial(serial: f64) -> String {
    let frac = serial.fract().abs();
    if !(1e-9..=1.0 - 1e-9).contains(&frac) {
        "yyyy-mm-dd".to_string()
    } else {
        "yyyy-mm-dd hh:mm:ss".to_string()
    }
}

/// Heuristic: does this number format look like a date/time format?
/// True when it contains any of the date/time tokens `y`, `m`, `d`, `h`, `s`.
pub fn is_date_format(fmt: &str) -> bool {
    let lowered = fmt.to_lowercase();
    ["y", "m", "d", "h", "s"].iter().any(|t| lowered.contains(t))
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
        assert_eq!(cell.value_raw().value_type, "Null");
        assert!(cell.formula().is_none());
    }

    #[test]
    fn test_cell_set_value_number() {
        let mut cell = Cell::new("B2".into(), 2, 2);
        cell.set_value_raw(CellValue::number(42.0));
        let v = cell.value_raw();
        assert_eq!(v.value_type, "Number");
        assert_eq!(v.number, Some(42.0));
    }

    #[test]
    fn test_cell_set_value_string() {
        let mut cell = Cell::new("C3".into(), 3, 3);
        cell.set_value_raw(CellValue::string("hello"));
        let v = cell.value_raw();
        assert_eq!(v.value_type, "String");
        assert_eq!(v.string, Some("hello".into()));
    }

    #[test]
    fn test_cell_set_value_bool() {
        let mut cell = Cell::new("D4".into(), 4, 4);
        cell.set_value_raw(CellValue::boolean(true));
        let v = cell.value_raw();
        assert_eq!(v.value_type, "Boolean");
        assert_eq!(v.boolean, Some(true));
    }

    #[test]
    fn test_cell_set_value_null() {
        let mut cell = Cell::new("E5".into(), 5, 5);
        cell.set_value_raw(CellValue::string("hello"));
        cell.set_value_raw(CellValue::default());
        let v = cell.value_raw();
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

    #[test]
    fn test_cell_value_date() {
        let cv = CellValue::date(45458.5);
        assert_eq!(cv.value_type, "Date");
        assert_eq!(cv.date_serial, Some(45458.5));
        // Note: `number` is separate; Date stores its serial in `date_serial`.
    }

    #[test]
    fn test_serial_epoch_round_trip() {
        // Unix epoch (1970-01-01) -> serial EXCEL_EPOCH_SERIAL
        assert!((millis_to_serial(0.0) - 25569.0).abs() < 1e-6);
        assert_eq!(serial_to_millis(25569.0), 0);

        // Round-trip a modern date: 2024-06-15T12:00:00Z
        let serial = 45458.5;
        let dt = serial_to_millis(serial) as f64;
        let roundtripped = millis_to_serial(dt);
        assert!(
            (roundtripped - serial).abs() < 1e-4,
            "serial {} -> dt {} ms -> serial {} (delta {})",
            serial,
            dt,
            roundtripped,
            (roundtripped - serial).abs()
        );
    }

    #[test]
    fn test_is_date_format_heuristic() {
        assert!(is_date_format("yyyy-mm-dd"));
        assert!(is_date_format("dd/mm/yyyy hh:mm:ss"));
        assert!(is_date_format("m/d/yy"));
        assert!(!is_date_format("General"));
        assert!(!is_date_format("0.00"));
        assert!(!is_date_format("0.0%"));
        assert!(!is_date_format(""));
    }

    #[test]
    fn test_date_format_for_serial() {
        // Whole-day serial (no fraction) -> date-only
        assert_eq!(date_format_for_serial(45458.0), "yyyy-mm-dd");
        // Fractional serial -> datetime
        assert_eq!(date_format_for_serial(45458.5), "yyyy-mm-dd hh:mm:ss");
        // Edge: exactly at noon
        assert_eq!(date_format_for_serial(25569.5), "yyyy-mm-dd hh:mm:ss");
    }
}
