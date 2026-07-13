//! Column definition: header label, data-binding key, width, visibility.

use napi_derive::napi;

use crate::model::style::Style;

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

    // -- col_num (read-only) --

    #[napi(getter)]
    pub fn col_num(&self) -> u32 {
        self.col_num
    }
}
