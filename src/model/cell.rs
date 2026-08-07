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
use napi::Unknown;
use napi_derive::napi;

use crate::error::ExcelrsError;
use crate::model::comment::CellComment;
use crate::model::style::{apply_style, Font, Style};
use crate::types;

// ---------------------------------------------------------------------------
// CellType
// ---------------------------------------------------------------------------

/// Discriminant for cell value variants. Mirrors the `value_type` string values.
#[napi(string_enum)]
#[derive(Clone, Debug, PartialEq)]
pub enum CellType {
    Null,
    Number,
    String,
    Boolean,
    Date,
    Formula,
    Error,
    Hyperlink,
    RichText,
    Merge,
}

impl CellType {
    /// Central lookup table mapping discriminant tag strings to `CellType`.
    /// Returns `None` for unrecognized tags.
    pub(crate) fn from_tag(tag: &str) -> Option<CellType> {
        Some(match tag {
            "Null" => CellType::Null,
            "Number" => CellType::Number,
            "String" => CellType::String,
            "Boolean" => CellType::Boolean,
            "Date" => CellType::Date,
            "Formula" => CellType::Formula,
            "Error" => CellType::Error,
            "Hyperlink" => CellType::Hyperlink,
            "RichText" => CellType::RichText,
            "Merge" => CellType::Merge,
            _ => return None,
        })
    }
}

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
#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RichTextRun {
    /// Text content for this run.
    pub text: String,
    /// Font formatting for this run (optional).
    pub font: Option<crate::model::style::Font>,
}

#[napi(object)]
#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CellValue {
    /// Discriminant: "Null" | "Number" | "String" | "Boolean" | "Formula" | "Error"
    /// | "Hyperlink" | "RichText" | "Date" | "Merge"
    #[napi(
        ts_type = "\"Null\" | \"Number\" | \"String\" | \"Boolean\" | \"Formula\" | \"Error\" | \"Hyperlink\" | \"RichText\" | \"Date\" | \"Merge\""
    )]
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

    /// Mark an existing `CellValue` as a formula, preserving all other
    /// cached fields (number, string, etc.). Used by the reader when a cell
    /// already carries a cached scalar from Pass 1 but also has a formula.
    pub fn mark_formula(mut self, formula: impl Into<String>) -> Self {
        self.value_type = "Formula".to_string();
        self.formula = Some(formula.into());
        self
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
    /// `true` only when a cached scalar was set by `Worksheet::recalculate()`
    /// (via `set_cached_value_raw`). Guards `cached_value()` per R4.
    pub recalc_only: bool,
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
                recalc_only: false,
            })),
        }
    }

    // -- value (getter + setter) --

    #[napi(getter, ts_return_type = "CellValueResult")]
    pub fn value(&self, env: Env) -> napi::Result<Unknown<'_>> {
        let inner = self.inner.lock().expect("Cell lock poisoned");
        let cv = &inner.value;
        match CellType::from_tag(&cv.value_type) {
            Some(CellType::Number) => Ok(env.to_js_value(&cv.number.unwrap_or(f64::NAN))?),
            Some(CellType::String) => Ok(env.to_js_value(&cv.string.as_deref().unwrap_or(""))?),
            Some(CellType::Boolean) => Ok(env.to_js_value(&cv.boolean.unwrap_or(false))?),
            Some(CellType::Date) => {
                let serial = cv.date_serial.unwrap_or(0.0);
                let ms = serial_to_millis(serial) as f64;
                let d = env.create_date(ms)?;
                // SAFETY: JsDate lifetime marker is nominal; the underlying napi_value
                // is valid for the environment's lifetime (same pattern as date getter).
                let d: napi::JsDate<'static> = unsafe { std::mem::transmute(d) };
                Ok(d.to_unknown())
            }
            Some(CellType::Null) => Ok(env.to_js_value(&serde_json::Value::Null)?),
            Some(CellType::Formula) => {
                // Formula cells carry an optional cached scalar (Excel-authored
                // `<f>..</f><v>..</v>` or JS-authored `cell.value = { formula, number, .. }`).
                // Return the cached scalar so it round-trips as a bare value; null
                // when no cache is present (formula authored without a result).
                if let Some(n) = cv.number {
                    Ok(env.to_js_value(&n)?)
                } else if let Some(ref s) = cv.string {
                    Ok(env.to_js_value(s)?)
                } else if let Some(b) = cv.boolean {
                    Ok(env.to_js_value(&b)?)
                } else if let Some(ref e) = &cv.error_value {
                    Ok(env.to_js_value(e)?)
                } else if let Some(serial) = cv.date_serial {
                    let ms = serial_to_millis(serial) as f64;
                    let d = env.create_date(ms)?;
                    let d: napi::JsDate<'static> = unsafe { std::mem::transmute(d) };
                    Ok(d.to_unknown())
                } else {
                    Ok(env.to_js_value(&serde_json::Value::Null)?)
                }
            }
            // Unknown tag (None), RichText, Hyperlink, Error, Merge — round-trip as CellValue object.
            _ => Ok(env.to_js_value(cv)?),
        }
    }

    /// Returns the cell value type discriminant.
    #[napi(getter, js_name = "type")]
    pub fn value_type(&self) -> CellType {
        let inner = self.inner.lock().expect("Cell lock poisoned");
        CellType::from_tag(&inner.value.value_type).unwrap_or(CellType::Null)
    }

    // -- date (read-only) --

    /// Returns a JS `Date` for Date-type cells, or `null` otherwise.
    #[napi(getter)]
    pub fn date(&self, env: Env) -> napi::Result<Option<napi::JsDate<'static>>> {
        let inner = self.inner.lock().expect("Cell lock poisoned");
        let cv = &inner.value;
        if matches!(CellType::from_tag(&cv.value_type), Some(CellType::Date)) {
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
            inner.formula = None;
            inner.recalc_only = false;
            return Ok(());
        }

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
            serde_json::Value::Object(obj) => {
                // Object-shape inference: detect cell-value variant from keys
                if obj.contains_key("richText") {
                    let runs = obj
                        .get("richText")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .map(|item| {
                                    let text = item.get("text").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                    let font = item.get("font").and_then(|v| {
                                        v.as_object().map(|f| Font {
                                            name: f.get("name").and_then(|v| v.as_str()).map(|s| s.to_string()),
                                            size: f.get("size").and_then(|v| v.as_f64()),
                                            bold: f.get("bold").and_then(|v| v.as_bool()),
                                            italic: f.get("italic").and_then(|v| v.as_bool()),
                                            underline: f.get("underline").and_then(|v| v.as_bool()),
                                            color: f.get("color").and_then(|v| v.as_str()).map(|s| s.to_string()),
                                            color_theme: None,
                                            color_tint: None,
                                        })
                                    });
                                    RichTextRun { text, font }
                                })
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    CellValue::rich_text(runs)
                } else if let Some(url) = obj.get("hyperlink").and_then(|v| v.as_str()) {
                    let text = obj.get("hyperlinkText").and_then(|v| v.as_str()).map(|s| s.to_string());
                    CellValue::hyperlink(url.to_string(), text)
                } else if let Some(f) = obj.get("formula").and_then(|v| v.as_str()) {
                    let mut cv = CellValue::formula(f.to_string());
                    cv.number = obj.get("number").and_then(|v| v.as_f64());
                    cv.string = obj.get("string").and_then(|v| v.as_str()).map(str::to_string);
                    cv.boolean = obj.get("boolean").and_then(|v| v.as_bool());
                    cv.error_value = obj.get("errorValue").and_then(|v| v.as_str()).map(str::to_string);
                    cv.date_serial = obj.get("dateSerial").and_then(|v| v.as_f64());
                    cv
                } else if let Some(vt) = obj.get("valueType").and_then(|v| v.as_str()) {
                    if CellType::from_tag(vt).is_none() {
                        return Err(napi::Error::from_reason(format!("Unknown valueType discriminant: '{vt}'. Expected one of: Number, String, Boolean, Formula, Error, Hyperlink, RichText, Date, Null, Merge")));
                    }
                    let number = obj.get("number").and_then(|v| v.as_f64());
                    let string = obj.get("string").and_then(|v| v.as_str()).map(|s| s.to_string());
                    let boolean = obj.get("boolean").and_then(|v| v.as_bool());
                    let error_value = obj.get("errorValue").and_then(|v| v.as_str()).map(|s| s.to_string());
                    let date_serial = obj.get("dateSerial").and_then(|v| v.as_f64());
                    CellValue {
                        value_type: vt.to_string(),
                        number,
                        string,
                        boolean,
                        error_value,
                        date_serial,
                        ..Default::default()
                    }
                } else {
                    CellValue::default()
                }
            }
            _ => CellValue::default(),
        };
        inner.formula = if matches!(CellType::from_tag(&inner.value.value_type), Some(CellType::Formula)) {
            inner.value.formula.clone()
        } else {
            None
        };
        inner.recalc_only = false;
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

    /// Cached computed value for formula cells.
    ///
    /// Returns a typed `CellValue` (number, string, boolean, or error)
    /// when the formula has been evaluated via `Worksheet::recalculate()`,
    /// or `null` when the cell is not a formula or hasn't been evaluated.
    #[napi(getter, js_name = "cachedValue")]
    pub fn cached_value(&self) -> Option<CellValue> {
        let inner = self.inner.lock().expect("Cell lock poisoned");
        let cv = &inner.value;
        if !matches!(CellType::from_tag(&cv.value_type), Some(CellType::Formula)) || !inner.recalc_only {
            return None;
        }
        if let Some(n) = cv.number {
            return Some(CellValue::number(n));
        }
        if let Some(ref s) = cv.string {
            return Some(CellValue::string(s));
        }
        if let Some(b) = cv.boolean {
            return Some(CellValue::boolean(b));
        }
        if let Some(ref e) = cv.error_value {
            return Some(CellValue {
                value_type: "Error".to_string(),
                error_value: Some(e.clone()),
                ..Default::default()
            });
        }
        if let Some(serial) = cv.date_serial {
            let result = CellValue::date(serial);
            return Some(result);
        }
        None
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
    pub fn set_style(&mut self, val: Option<Style>) -> napi::Result<()> {
        let mut inner = self.inner.lock().expect("Cell lock poisoned");
        apply_style(&mut inner.style, val)
    }

    // -- value_of / rich_text (typed accessors) --

    /// Returns the full `CellValue` discriminated union for this cell.
    /// The return type is `CellValue` (a TS discriminated union);
    /// narrow on `valueType` to access variant-specific fields
    /// without casting.
    #[napi(getter, js_name = "valueOf")]
    pub fn value_of(&self) -> CellValue {
        self.inner.lock().expect("Cell lock poisoned").value.clone()
    }

    /// Returns the parsed rich-text runs when the cell is a RichText cell,
    /// or `null` otherwise. No cast is required to read the runs.
    /// Mirrors `cell.formula` — a dedicated typed accessor instead of forcing
    /// the caller to narrow `CellValue`.
    #[napi(getter)]
    pub fn rich_text(&self) -> Option<Vec<RichTextRun>> {
        self.inner.lock().expect("Cell lock poisoned").value.rich_text.clone()
    }
}

impl Cell {
    /// Internal: set the CellValue directly (used by reader, add_row).
    /// Skips the serde_json::Value dispatch for efficiency.
    pub fn set_value_raw(&mut self, value: CellValue) {
        let mut inner = self.inner.lock().expect("Cell lock poisoned");
        inner.value = value;
        inner.recalc_only = false;
    }

    /// Internal: return the raw `CellValue` (a `Date` cell exposes the serial,
    /// not a JS `Date`). Used by the writer and tests.
    pub fn value_raw(&self) -> CellValue {
        self.inner.lock().expect("Cell lock poisoned").value.clone()
    }

    /// Internal: set the style directly (used by reader, set_columns).
    /// Skips the serde_json::Value dispatch.
    /// Deep clone: creates a new independent `Arc<Mutex<CellInner>>`
    /// (clone shares the existing Arc, mutate the same backing state).
    pub(crate) fn deep_clone(&self) -> Self {
        Cell {
            inner: Arc::new(Mutex::new(self.inner.lock().expect("Cell lock poisoned").clone())),
        }
    }

    pub fn set_style_raw(&mut self, style: Option<Style>) {
        self.inner.lock().expect("Cell lock poisoned").style = style;
    }

    /// Internal: set the formula string (used by reader).
    pub fn set_formula(&mut self, formula: Option<String>) {
        self.inner.lock().expect("Cell lock poisoned").formula = formula;
    }

    /// Internal: store a computed formula result on the cell's `CellValue`.
    /// Called by `Worksheet::recalculate()` after evaluating a formula.
    /// Preserves `value_type = "Formula"` and the existing `formula` string;
    /// only populates the cached scalar fields (`number`, `string`, etc.).
    #[cfg(feature = "formula-eval")]
    pub fn set_cached_value_raw(&mut self, value: CellValue) {
        let mut inner = self.inner.lock().expect("Cell lock poisoned");
        inner.value.number = value.number;
        inner.value.string = value.string;
        inner.value.boolean = value.boolean;
        inner.value.error_value = value.error_value;
        inner.value.date_serial = value.date_serial;
        inner.value.rich_text = value.rich_text;
        inner.recalc_only = true;
    }

    /// Internal: renumber this cell to a new row, updating its cached `row`
    /// and recomputing its A1 `address`. Used when rows are shifted
    /// (insert/splice/duplicate) so cell addresses stay consistent.
    pub fn renumber(&mut self, new_row: u32) {
        // Lock the inner state and update directly (row/col/address on Cell
        // are #[napi(getter)] which shadows direct field access).
        let mut inner = self.inner.lock().expect("Cell lock poisoned");
        inner.row = new_row;
        inner.address = Cell::compute_address(new_row, inner.col);
    }

    /// A cell is "effectively empty" when it has no value, no formula, and
    /// no style — i.e., it was only created by a read-side `getCell` and
    /// never populated. The writer skips these cells to avoid phantom output.
    pub fn is_effectively_empty(&self) -> bool {
        let inner = self.inner.lock().expect("Cell lock poisoned");
        matches!(CellType::from_tag(&inner.value.value_type), Some(CellType::Null)) && inner.formula.is_none() && inner.style.is_none()
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
    use crate::model::style::Font;

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
    fn test_set_style_some() {
        let mut cell = Cell::new("A1".into(), 1, 1);
        let style = Style {
            font: Some(Font {
                bold: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        };
        cell.set_style(Some(style)).unwrap();
        assert!(cell.style().is_some());
        assert_eq!(cell.style().unwrap().font.unwrap().bold, Some(true));
    }

    #[test]
    fn test_set_style_none() {
        let mut cell = Cell::new("A1".into(), 1, 1);
        // Pre-set a style via raw
        cell.set_style_raw(Some(Style {
            font: Some(Font {
                bold: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        }));
        assert!(cell.style().is_some());
        // Reset with None
        cell.set_style(None).unwrap();
        assert!(cell.style().is_none());
    }

    #[test]
    fn test_set_style_empty_object() {
        let mut cell = Cell::new("A1".into(), 1, 1);
        // {} in JS all-None Style is_empty() normalizes to None
        let empty = Style {
            font: None,
            fill: None,
            border: None,
            alignment: None,
            num_fmt: None,
        };
        cell.set_style(Some(empty)).unwrap();
        assert!(cell.style().is_none());
    }

    #[test]
    fn test_set_style_rejects_invalid() {
        let mut cell = Cell::new("A1".into(), 1, 1);
        // Empty num_fmt string is invalid
        let invalid = Style {
            num_fmt: Some("".into()),
            ..Default::default()
        };
        assert!(cell.set_style(Some(invalid)).is_err());
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

    // ---------------------------------------------------------------------------
    // from_tag round-trip tests
    // ---------------------------------------------------------------------------

    #[test]
    fn test_cell_type_from_tag_all_tags() {
        // Each CellType variant must have a corresponding arm in from_tag.
        // When adding a new variant, add it to this array.
        // No compile-time exhaustiveness: from_tag has a `_ => return None` arm.
        let cases = [
            ("Null", CellType::Null),
            ("Number", CellType::Number),
            ("String", CellType::String),
            ("Boolean", CellType::Boolean),
            ("Date", CellType::Date),
            ("Formula", CellType::Formula),
            ("Error", CellType::Error),
            ("Hyperlink", CellType::Hyperlink),
            ("RichText", CellType::RichText),
            ("Merge", CellType::Merge),
        ];
        for (tag, expected) in &cases {
            assert_eq!(
                CellType::from_tag(tag),
                Some(expected.clone()),
                "from_tag({tag:?}) should be Some({expected:?})"
            );
        }
        assert_eq!(CellType::from_tag("Unknown"), None);
        assert_eq!(CellType::from_tag(""), None);
    }

    #[test]
    fn test_cell_type_from_tag_round_trip_via_cell_value() {
        // CellValue constructors set value_type strings that from_tag must parse.
        // This tests the real coupling between construction and parsing.
        let cases = [
            (CellValue::default(), CellType::Null),
            (CellValue::number(0.0), CellType::Number),
            (CellValue::string(""), CellType::String),
            (CellValue::boolean(false), CellType::Boolean),
            (CellValue::formula(""), CellType::Formula),
            (CellValue::hyperlink("", None), CellType::Hyperlink),
            (CellValue::rich_text(vec![]), CellType::RichText),
            (CellValue::date(0.0), CellType::Date),
        ];
        for (cv, expected) in &cases {
            assert_eq!(
                CellType::from_tag(&cv.value_type),
                Some(expected.clone()),
                "round-trip via {expected:?} failed"
            );
        }
    }

    #[test]
    fn test_cached_value_getter_r4() {
        // R4: cachedValue is recalc-only — returns None unless the cached
        // scalar was set by recalculate() via set_cached_value_raw.
        // Reader/authoring paths (set_value_raw, set_value) set recalc_only=false.

        // Non-Formula cell: cachedValue is null.
        let mut cell = Cell::new("A1".into(), 1, 1);
        cell.set_value_raw(CellValue::number(42.0));
        assert_eq!(cell.value_raw().value_type, "Number");
        assert!(cell.cached_value().is_none());

        // Formula cell with a cached scalar set via set_value_raw (reader/authoring
        // path) returns None — cachedValue requires recalc_only=true.
        let mut cell = Cell::new("B1".into(), 1, 2);
        let mut cv = CellValue::formula("A1+B1");
        cv.number = Some(3.0);
        cell.set_value_raw(cv);
        assert_eq!(cell.value_raw().value_type, "Formula");
        assert!(
            cell.cached_value().is_none(),
            "cachedValue must be None without recalc (R4)"
        );

        // Formula cell without a cached scalar returns None.
        let mut cell = Cell::new("C1".into(), 1, 3);
        cell.set_value_raw(CellValue::formula("SUM(A1:B1)"));
        assert!(cell.cached_value().is_none());
    }
}
