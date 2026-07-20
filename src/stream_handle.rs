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

use std::io::{Cursor, Read};
use std::sync::{Arc, Mutex};

use napi::bindgen_prelude::*;
use napi_derive::napi;

use crate::model::workbook_inner::WorkbookInner;
use crate::reader::styles::{self as reader_styles, StyleTableRead};
use crate::stream::{
    parse_shared_strings, parse_sheet_rows, parse_workbook_sheet_targets, stream_read, stream_write, StreamCell,
    StreamRow, StreamSheet, StreamValue, MAX_ARCHIVE_BYTES, MAX_ARCHIVE_ENTRIES, MAX_ENTRY_BYTES,
};

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
        // A JS cell with no populated field defaults to an empty-string cell,
        // matching v2.0.0. Use `empty: true` to emit an empty cell.
        StreamValue::Text(String::new())
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

// ---------------------------------------------------------------------------
// Constant-memory streaming reader — async iterator (pull-based)
// ---------------------------------------------------------------------------

/// Pre-parsed workbook metadata held across `next()` calls.
struct StreamParseState {
    /// Zip archive opened once in the constructor and reused per sheet
    /// (no central-directory re-parse on each `next()`).
    archive: zip::ZipArchive<Cursor<Arc<[u8]>>>,
    /// Sheet targets in document order.
    sheets: Vec<(String, String)>,
    /// Shared strings table (shared cheaply across sheets).
    shared: Arc<Vec<String>>,
    /// Style table (value formatting), if available (shared cheaply).
    style_table: Option<Arc<StyleTableRead>>,
    /// Current sheet index.
    index: usize,
}

/// Constant-memory streaming reader for .xlsx files.
///
/// Yields one `JsStreamSheet` at a time via `for await...of`.
/// Only the current sheet is materialized in memory.
///
/// @example
/// ```js
/// const reader = new StreamReader(buffer);
/// for await (const sheet of reader) {
///   console.log(sheet.name, sheet.rowCount);
/// }
/// ```
#[napi(async_iterator)]
pub struct StreamReader {
    state: Mutex<Option<StreamParseState>>,
}

#[napi]
impl StreamReader {
    /// Create a new streaming reader from an .xlsx buffer.
    ///
    /// Parses the workbook metadata (sheet names, shared strings, styles)
    /// eagerly so that each `next()` call only needs to parse one sheet.
    #[napi(constructor)]
    pub fn constructor(buffer: Buffer) -> Result<Self> {
        let data: Arc<[u8]> = buffer.to_vec().into();
        let cursor = Cursor::new(Arc::clone(&data));
        let mut archive = zip::ZipArchive::new(cursor).map_err(|e| napi::Error::from_reason(e.to_string()))?;

        if archive.len() > MAX_ARCHIVE_ENTRIES {
            return Err(napi::Error::from_reason(format!(
                "streaming reader rejected input: too many entries ({} > limit {})",
                archive.len(),
                MAX_ARCHIVE_ENTRIES
            )));
        }
        if buffer.len() as u64 > MAX_ARCHIVE_BYTES {
            return Err(napi::Error::from_reason(format!(
                "streaming reader rejected input: file too large ({} bytes > limit {} bytes)",
                buffer.len(),
                MAX_ARCHIVE_BYTES
            )));
        }

        let sheets = parse_workbook_sheet_targets(&mut archive).map_err(|e| napi::Error::from_reason(e.to_string()))?;
        let shared = parse_shared_strings(&mut archive).map_err(|e| napi::Error::from_reason(e.to_string()))?;
        let style_table = reader_styles::parse_styles_and_sheet_maps(&data, sheets.len())
            .map(|(t, _)| t)
            .ok();

        Ok(Self {
            state: Mutex::new(Some(StreamParseState {
                archive,
                sheets,
                shared: Arc::new(shared),
                style_table: style_table.map(Arc::new),
                index: 0,
            })),
        })
    }
}

impl napi::bindgen_prelude::AsyncGenerator for StreamReader {
    type Yield = JsStreamSheet;
    type Next = ();
    type Return = ();

    #[allow(refining_impl_trait)]
    fn next(
        &mut self,
        _value: Option<Self::Next>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<Self::Yield>>> + Send + 'static>> {
        // Extract owned data under the lock, drop the lock, then parse.
        let extracted = {
            let mut guard = match self.state.lock() {
                Ok(g) => g,
                Err(e) => {
                    let msg = e.to_string();
                    return Box::pin(async move { Err(napi::Error::from_reason(msg)) });
                }
            };
            let state = match guard.as_mut() {
                Some(s) => s,
                None => return Box::pin(async { Err(napi::Error::from_reason("StreamReader already consumed")) }),
            };

            if state.index >= state.sheets.len() {
                *guard = None;
                return Box::pin(async { Ok(None) });
            }

            let (name, path) = state.sheets[state.index].clone();
            let entry = match state.archive.by_name(&path) {
                Ok(e) => e,
                Err(err) => {
                    let msg = err.to_string();
                    return Box::pin(async move { Err(napi::Error::from_reason(msg)) });
                }
            };
            let mut raw = Vec::new();
            if let Err(err) = entry.take(MAX_ENTRY_BYTES).read_to_end(&mut raw) {
                let msg = err.to_string();
                return Box::pin(async move { Err(napi::Error::from_reason(msg)) });
            }
            let xml = String::from_utf8_lossy(&raw).to_string();
            let shared = Arc::clone(&state.shared);
            let style_table = state.style_table.clone();
            state.index += 1;
            (name, xml, shared, style_table)
        };

        // All data is owned — future is 'static, no borrow of self.
        Box::pin(async move {
            let (name, xml, shared, style_table) = extracted;

            let rows = match style_table {
                Some(ref st) => parse_sheet_rows(&xml, st.as_ref(), shared.as_slice()),
                None => parse_sheet_rows(&xml, &reader_styles::StyleTableRead::empty(), shared.as_slice()),
            }
            .map_err(|e| napi::Error::from_reason(e.to_string()))?;

            Ok(Some(to_js_sheet(&StreamSheet { name, rows })))
        })
    }
}

// ---------------------------------------------------------------------------
// Streaming writer — accepts sheets incrementally, outputs .xlsx Buffer
// ---------------------------------------------------------------------------

/// Streaming writer that accepts sheets one at a time and produces an .xlsx buffer.
///
/// The caller pushes sheets via `write_sheet()` and then calls `finalize()`
/// to get the complete .xlsx bytes. The caller does not need to hold all
/// sheets in memory at once.
///
/// @example
/// ```js
/// const writer = new StreamWriter();
/// writer.write_sheet(sheet1);
/// writer.write_sheet(sheet2);
/// const buf = writer.finalize();
/// ```
#[napi]
pub struct StreamWriter {
    sheets: Vec<StreamSheet>,
}

#[napi]
impl StreamWriter {
    /// Create a new empty streaming writer.
    #[napi(constructor)]
    pub fn constructor() -> Self {
        Self { sheets: Vec::new() }
    }

    /// Append a sheet to the output buffer.
    ///
    /// The sheet is stored internally and written to the .xlsx on `finalize()`.
    #[napi]
    pub fn write_sheet(&mut self, sheet: JsStreamSheet) -> Result<()> {
        self.sheets.push(from_js_sheet(&sheet));
        Ok(())
    }

    /// Finalize the streaming writer and produce the .xlsx buffer.
    ///
    /// Consumes the internal sheet list and returns the complete .xlsx bytes.
    #[napi]
    pub fn finalize(&mut self) -> Result<Buffer> {
        let sheets = std::mem::take(&mut self.sheets);
        let bytes = stream_write(&sheets).map_err(|e| napi::Error::from_reason(e.to_string()))?;
        Ok(Buffer::from(bytes))
    }
}
