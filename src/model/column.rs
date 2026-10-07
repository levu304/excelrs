//! Column definition: header label, data-binding key, width, visibility.

use std::sync::{Arc, Mutex};

use napi_derive::napi;

use crate::model::style::{apply_style, Style};

/// Input type for `Worksheet.setColumns` — a plain JS object with optional fields.
///
/// Mirrors the `Column` class fields but uses `Option<T>` so every property
/// is optional in TypeScript.  napi-rs generates a TS interface (not a class)
/// from `#[napi(object)]`, accepting plain JS objects directly.
///
/// Fields not provided by the caller get sensible defaults on the Rust side
/// (empty string for header/key, 0 for width, etc.).
#[napi(object)]
#[derive(Clone, Debug, Default)]
pub struct ColumnInput {
    /// 1-indexed column position. `None` or `0` means auto-assigned.
    pub col_num: Option<u32>,
    pub header: Option<String>,
    pub key: Option<String>,
    pub width: Option<f64>,
    pub hidden: Option<bool>,
    /// Column-level default style.
    pub style: Option<Style>,
    /// Outline/grouping level, clamped to 0–7.
    pub outline_level: Option<u8>,
}

/// The mutable state shared by all clones of a `Column`.
///
/// Every clone of a `Column` shares the same underlying state, so mutations
/// made through a handle returned by `getColumn` are visible to the
/// worksheet model (and vice versa) even though napi-rs passes `Column` by
/// value/clone across the FFI boundary. This matches the pattern used by
/// `Row` and `Cell`.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ColumnInner {
    /// 1-indexed column number. 0 = auto-assign in set_columns.
    #[serde(default)]
    pub col_num: u32,
    pub header: String,
    pub key: String,
    pub width: f64,
    #[serde(default)]
    pub hidden: bool,
    /// Column-level style (default for cells in this column with no
    /// explicit cell-level style). Write-only in v0.2.0.
    #[serde(default)]
    pub style: Option<Style>,
    /// Outline/grouping level for this column, `0`–`7` (Excel's cap). `0` means no grouping.
    #[serde(default)]
    pub outline_level: u8,
}

/// A column definition in a worksheet.
///
/// Mirrors the exceljs `Column` interface: header label, data-binding key,
/// width in characters, hidden state, and 1-indexed column number.
///
/// `col_num` is optional in the JS object. If omitted (or 0), it is
/// auto-assigned sequentially in `Worksheet.setColumns` — the first column
/// gets col_num=1, the second gets col_num=2, etc.  For sparse definitions
/// (e.g. defining only column B), pass the `colNum` explicitly.
#[napi]
#[derive(Clone, Debug)]
pub struct Column {
    inner: Arc<Mutex<ColumnInner>>,
}

impl serde::Serialize for Column {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.inner.lock().expect("Column lock poisoned").serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for Column {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        ColumnInner::deserialize(deserializer).map(|inner| Column {
            inner: Arc::new(Mutex::new(inner)),
        })
    }
}

#[napi]
impl Column {
    #[napi(constructor)]
    pub fn new(header: String, key: String, width: f64) -> Self {
        Column {
            inner: Arc::new(Mutex::new(ColumnInner {
                col_num: 0,
                header,
                key,
                width,
                hidden: false,
                style: None,
                outline_level: 0,
            })),
        }
    }

    #[napi(getter)]
    pub fn header(&self) -> String {
        self.inner.lock().expect("Column lock poisoned").header.clone()
    }

    #[napi(setter)]
    pub fn set_header(&mut self, val: String) {
        self.inner.lock().expect("Column lock poisoned").header = val;
    }

    #[napi(getter)]
    pub fn key(&self) -> String {
        self.inner.lock().expect("Column lock poisoned").key.clone()
    }

    #[napi(setter)]
    pub fn set_key(&mut self, val: String) {
        self.inner.lock().expect("Column lock poisoned").key = val;
    }

    #[napi(getter)]
    pub fn width(&self) -> f64 {
        self.inner.lock().expect("Column lock poisoned").width
    }

    #[napi(setter)]
    pub fn set_width(&mut self, val: f64) {
        self.inner.lock().expect("Column lock poisoned").width = val;
    }

    #[napi(getter)]
    pub fn hidden(&self) -> bool {
        self.inner.lock().expect("Column lock poisoned").hidden
    }

    #[napi(setter)]
    pub fn set_hidden(&mut self, val: bool) {
        self.inner.lock().expect("Column lock poisoned").hidden = val;
    }

    // -- style (getter + setter) --

    #[napi(getter)]
    pub fn style(&self) -> Option<Style> {
        self.inner.lock().expect("Column lock poisoned").style.clone()
    }

    #[napi(setter)]
    pub fn set_style(&mut self, val: Option<Style>) -> napi::Result<()> {
        let mut guard = self.inner.lock().expect("Column lock poisoned");
        apply_style(&mut guard.style, val)
    }

    // -- outline level (grouping) --

    /// Outline/grouping level for this column, `0`–`7` (Excel's cap). `0` means no grouping.
    #[napi(getter)]
    pub fn outline_level(&self) -> u8 {
        self.inner.lock().expect("Column lock poisoned").outline_level
    }

    /// Set the outline/grouping level. Values are clamped to `0`–`7`.
    #[napi(setter)]
    pub fn set_outline_level(&mut self, val: u32) {
        self.inner.lock().expect("Column lock poisoned").outline_level = val.min(7) as u8;
    }

    // -- col_num (read-only) --

    #[napi(getter)]
    pub fn col_num(&self) -> u32 {
        self.inner.lock().expect("Column lock poisoned").col_num
    }
}

// Internal methods (not exposed via napi)
impl Column {
    /// Set the 1-indexed column number (used by `set_columns` auto-assignment
    /// and `get_column` creation). crate-internal: the public surface is read-only.
    pub(crate) fn set_col_num(&mut self, val: u32) {
        self.inner.lock().expect("Column lock poisoned").col_num = val;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::style::Font;

    fn test_column() -> Column {
        let mut col = Column::new(String::new(), String::new(), 10.0);
        col.set_col_num(1);
        col
    }

    #[test]
    fn test_column_set_style_some() {
        let mut col = test_column();
        let style = Style {
            font: Some(Font {
                bold: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        };
        col.set_style(Some(style)).unwrap();
        assert!(col.style().is_some());
        assert_eq!(col.style().unwrap().font.unwrap().bold, Some(true));
    }

    #[test]
    fn test_column_set_style_none() {
        let mut col = test_column();
        // Pre-set a style
        col.set_style(Some(Style {
            font: Some(Font {
                bold: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        }))
        .unwrap();
        assert!(col.style().is_some());
        // Reset with None
        col.set_style(None).unwrap();
        assert!(col.style().is_none());
    }

    #[test]
    fn test_column_set_style_empty_object() {
        let mut col = test_column();
        // {} in JS all-None Style is_empty() normalizes to None
        col.set_style(Some(Style::default())).unwrap();
        assert!(col.style().is_none());
    }

    #[test]
    fn test_column_set_style_rejects_invalid() {
        let mut col = test_column();
        // Empty num_fmt string is invalid
        let invalid = Style {
            num_fmt: Some("".into()),
            ..Default::default()
        };
        assert!(col.set_style(Some(invalid)).is_err());
    }

    #[test]
    fn test_column_clone_shares_state() {
        // Live-handle contract: a clone shares the same backing state, so a
        // mutation through one handle is visible through the other (the FFI
        // passes Column by clone, as Row and Cell already do).
        let mut col = test_column();
        let clone = col.clone();
        col.set_width(20.0);
        assert_eq!(clone.width(), 20.0);
        col.set_header("Name".into());
        assert_eq!(clone.header(), "Name");
    }
}
