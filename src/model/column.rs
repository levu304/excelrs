//! Column definition: header label, data-binding key, width, visibility.

use napi_derive::napi;

/// A column definition in a worksheet.
///
/// Mirrors the exceljs `Column` interface: header label, data-binding key,
/// width in characters, and hidden state.
#[napi]
#[derive(Clone, Debug)]
pub struct Column {
    header: String,
    key: String,
    width: f64,
    hidden: bool,
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
}
