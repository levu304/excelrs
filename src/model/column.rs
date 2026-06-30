//! Column definition: header label, data-binding key, width, visibility.

use napi_derive::napi;

use crate::model::style::Style;

/// A column definition in a worksheet.
///
/// Mirrors the exceljs `Column` interface: header label, data-binding key,
/// width in characters, and hidden state.
#[napi]
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Column {
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
        let style: Style = serde_json::from_value(val).map_err(|e| {
            napi::Error::from_reason(format!("style: {e}"))
        })?;
        if style.is_empty() {
            self.style = None;
            return Ok(());
        }
        self.style = Some(style.validate().map_err(|e| {
            napi::Error::from_reason(e.to_string())
        })?);
        Ok(())
    }
}
