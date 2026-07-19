//! napi surface for streaming XLSX I/O.
//!
//! Mirrors ExcelJS `workbook.stream.xlsx.read/write`. The streaming reader
//! yields sheet/row/cell structures without materializing the full in-memory
//! model; the streaming writer accepts them and emits a `.xlsx` buffer.
//!
//! v2.0.0 scope: cell *values* cross the FFI boundary (numbers, strings,
//! booleans, formulas). Per-cell styles are intentionally not surfaced here
//! yet — they remain available through the in-memory `xlsx` read/write path.
//! See `design.md` D4 (Open Questions) for the follow-up Node stream bridge
//! and style surfacing.

use std::sync::{Arc, Mutex};

use napi::bindgen_prelude::*;
use napi_derive::napi;

use crate::model::workbook_inner::WorkbookInner;
use crate::stream::{stream_read, stream_write, StreamCell, StreamRow, StreamSheet, StreamValue};

/// Cross-FFI cell value. Exactly one variant is populated per cell.
#[napi(object)]
pub struct JsStreamValue {
    pub number: Option<f64>,
    pub text: Option<String>,
    pub boolean: Option<bool>,
    pub formula: Option<String>,
    /// Set `true` for an empty cell (no value). Distinct from `text: ""`.
    pub empty: Option<bool>,
}

#[napi(object)]
pub struct JsStreamCell {
    pub col: u32,
    pub value: JsStreamValue,
}

#[napi(object)]
pub struct JsStreamRow {
    pub r: u32,
    pub cells: Vec<JsStreamCell>,
}

#[napi(object)]
pub struct JsStreamSheet {
    pub name: String,
    pub rows: Vec<JsStreamRow>,
}

fn to_js_value(v: &StreamValue) -> JsStreamValue {
    match v {
        StreamValue::Number(n) => JsStreamValue {
            number: Some(*n),
            text: None,
            boolean: None,
            formula: None,
            empty: None,
        },
        StreamValue::Text(s) => JsStreamValue {
            number: None,
            text: Some(s.clone()),
            boolean: None,
            formula: None,
            empty: None,
        },
        StreamValue::Bool(b) => JsStreamValue {
            number: None,
            text: None,
            boolean: Some(*b),
            formula: None,
            empty: None,
        },
        StreamValue::Formula(f) => JsStreamValue {
            number: None,
            text: None,
            boolean: None,
            formula: Some(f.clone()),
            empty: None,
        },
        StreamValue::Empty => JsStreamValue {
            number: None,
            text: None,
            boolean: None,
            formula: None,
            empty: Some(true),
        },
    }
}

fn from_js_value(v: &JsStreamValue) -> StreamValue {
    if let Some(n) = v.number {
        StreamValue::Number(n)
    } else if let Some(s) = &v.text {
        StreamValue::Text(s.clone())
    } else if let Some(b) = v.boolean {
        StreamValue::Bool(b)
    } else if let Some(f) = &v.formula {
        StreamValue::Formula(f.clone())
    } else if v.empty == Some(true) {
        StreamValue::Empty
    } else {
        // ponytail: a JS cell with no populated field is an empty cell, not Text("")
        StreamValue::Empty
    }
}

fn to_js_sheet(s: &StreamSheet) -> JsStreamSheet {
    JsStreamSheet {
        name: s.name.clone(),
        rows: s
            .rows
            .iter()
            .map(|r| JsStreamRow {
                r: r.r,
                cells: r
                    .cells
                    .iter()
                    .map(|c| JsStreamCell {
                        col: c.col,
                        value: to_js_value(&c.value),
                    })
                    .collect(),
            })
            .collect(),
    }
}

fn from_js_sheet(s: &JsStreamSheet) -> StreamSheet {
    StreamSheet {
        name: s.name.clone(),
        rows: s
            .rows
            .iter()
            .map(|r| StreamRow {
                r: r.r,
                cells: r
                    .cells
                    .iter()
                    .map(|c| StreamCell {
                        col: c.col,
                        value: from_js_value(&c.value),
                        style: None,
                    })
                    .collect(),
                style: None,
            })
            .collect(),
    }
}

/// Streaming I/O namespace handle (ExcelJS `workbook.stream`).
#[napi]
#[derive(Clone, Debug)]
pub struct WorkbookStream {
    #[allow(dead_code)]
    inner: Arc<Mutex<WorkbookInner>>,
}

impl WorkbookStream {
    pub fn new(inner: Arc<Mutex<WorkbookInner>>) -> Self {
        WorkbookStream { inner }
    }
}

#[napi]
impl WorkbookStream {
    /// Returns the streaming `xlsx` handle.
    #[napi(getter)]
    pub fn xlsx(&self) -> WorkbookStreamXlsx {
        WorkbookStreamXlsx::new(Arc::clone(&self.inner))
    }
}

/// Streaming `xlsx` read/write handle (ExcelJS `workbook.stream.xlsx`).
#[napi]
#[derive(Clone, Debug)]
pub struct WorkbookStreamXlsx {
    #[allow(dead_code)]
    inner: Arc<Mutex<WorkbookInner>>,
}

impl WorkbookStreamXlsx {
    pub fn new(inner: Arc<Mutex<WorkbookInner>>) -> Self {
        WorkbookStreamXlsx { inner }
    }
}

#[napi]
impl WorkbookStreamXlsx {
    /// Stream-read an .xlsx `Buffer` into sheet/row/cell structures without
    /// materializing the full in-memory model. Async.
    ///
    /// @remarks Must be awaited. Returns one entry per worksheet, each holding
    /// its rows and cells (numbers / strings / booleans / formulas).
    #[napi]
    pub async fn read(&self, buffer: Buffer) -> Result<Vec<JsStreamSheet>> {
        let data = buffer.to_vec();
        let sheets = stream_read(&data).map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok(sheets.iter().map(to_js_sheet).collect())
    }

    /// Stream-write sheet/row/cell structures to an .xlsx `Buffer` without
    /// materializing the full in-memory model. Async.
    ///
    /// @remarks Must be awaited. Returns the `.xlsx` bytes.
    #[napi]
    pub async fn write(&self, sheets: Vec<JsStreamSheet>) -> Result<Buffer> {
        let native: Vec<StreamSheet> = sheets.iter().map(from_js_sheet).collect();
        let bytes = stream_write(&native).map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok(Buffer::from(bytes))
    }
}
