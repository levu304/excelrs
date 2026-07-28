//! Column definition: header label, data-binding key, width, visibility.

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
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Column {
    /// 1-indexed column number. 0 = auto-assign in set_columns.
    #[serde(default)]
    pub(crate) col_num: u32,
    header: String,
    key: String,
    width: f64,
    #[serde(default)]
    hidden: bool,
    /// Column-level style (default for cells in this column with no
    /// explicit cell-level style). Write-only in v0.2.0.
    #[serde(default)]
    pub(crate) style: Option<Style>,
    /// Outline/grouping level for this column, `0`–`7` (Excel's cap). `0` means no grouping.
    #[serde(default)]
    pub(crate) outline_level: u8,
}

#[napi]
impl Column {
    #[napi(constructor)]
    pub fn new(header: String, key: String, width: f64) -> Self {
        Column {
            col_num: 0,
            header,
            key,
            width,
            hidden: false,
            style: None,
            outline_level: 0,
        }
    }

    #[napi(getter)]
    pub fn header(&self) -> String {
        self.header.clone()
    }

    #[napi(setter)]
    pub fn set_header(&mut self, val: String) {
        self.header = val;
    }

    #[napi(getter)]
    pub fn key(&self) -> String {
        self.key.clone()
    }

    #[napi(setter)]
    pub fn set_key(&mut self, val: String) {
        self.key = val;
    }

    #[napi(getter)]
    pub fn width(&self) -> f64 {
        self.width
    }

    #[napi(setter)]
    pub fn set_width(&mut self, val: f64) {
        self.width = val;
    }

    #[napi(getter)]
    pub fn hidden(&self) -> bool {
        self.hidden
    }

    #[napi(setter)]
    pub fn set_hidden(&mut self, val: bool) {
        self.hidden = val;
    }

    // -- style (getter + setter) --

    #[napi(getter)]
    pub fn style(&self) -> Option<Style> {
        self.style.clone()
    }

    #[napi(setter)]
    pub fn set_style(&mut self, val: Option<Style>) -> napi::Result<()> {
        apply_style(&mut self.style, val)
    }

    // -- outline level (grouping) --

    /// Outline/grouping level for this column, `0`–`7` (Excel's cap). `0` means no grouping.
    #[napi(getter)]
    pub fn outline_level(&self) -> u8 {
        self.outline_level
    }

    /// Set the outline/grouping level. Values are clamped to `0`–`7`.
    #[napi(setter)]
    pub fn set_outline_level(&mut self, val: u32) {
        self.outline_level = val.min(7) as u8;
    }

    // -- col_num (read-only) --

    #[napi(getter)]
    pub fn col_num(&self) -> u32 {
        self.col_num
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::style::Font;

    #[test]
    fn test_column_set_style_some() {
        let mut col = Column {
            col_num: 1,
            header: String::new(),
            key: String::new(),
            width: 10.0,
            hidden: false,
            style: None,
            outline_level: 0,
        };
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
        let mut col = Column {
            col_num: 1,
            header: String::new(),
            key: String::new(),
            width: 10.0,
            hidden: false,
            style: Some(Style {
                font: Some(Font {
                    bold: Some(true),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            outline_level: 0,
        };
        assert!(col.style().is_some());
        // Reset with None
        col.set_style(None).unwrap();
        assert!(col.style().is_none());
    }

    #[test]
    fn test_column_set_style_empty_object() {
        let mut col = Column {
            col_num: 1,
            header: String::new(),
            key: String::new(),
            width: 10.0,
            hidden: false,
            style: None,
            outline_level: 0,
        };
        // {} in JS all-None Style is_empty() normalizes to None
        col.set_style(Some(Style::default())).unwrap();
        assert!(col.style().is_none());
    }

    #[test]
    fn test_column_set_style_rejects_invalid() {
        let mut col = Column {
            col_num: 1,
            header: String::new(),
            key: String::new(),
            width: 10.0,
            hidden: false,
            style: None,
            outline_level: 0,
        };
        // Empty num_fmt string is invalid
        let invalid = Style {
            num_fmt: Some("".into()),
            ..Default::default()
        };
        assert!(col.set_style(Some(invalid)).is_err());
    }
}
