//! Worksheet tables (v1.1.0).
//!
//! Mirrors ExcelJS `ws.addTable` / `ws.getTable(s)` / `ws.removeTable`.
//! Tables are serialized to `xl/tables/tableN.xml` (one part per table) and
//! referenced from the worksheet via a `table` relationship in the sheet `.rels`.

use napi_derive::napi;
use std::sync::{Arc, Mutex};

use crate::model::cell::CellValue;
use serde_json::Value;

/// A single table column definition.
#[napi(object)]
#[derive(Clone, Debug, Default)]
pub struct TableColumn {
    /// Column header text (written into the header-row cell).
    pub name: String,
    /// Totals-row label (e.g. "Total"). Emitted as `totalsRowLabel`.
    pub totals_row_label: Option<String>,
    /// Totals-row function (e.g. "sum", "average"). Emitted as `totalsRowFunction`.
    pub totals_row_function: Option<String>,
}

/// One row of table data values (data rows only; the header is derived from columns).
#[napi(object)]
#[derive(Clone, Debug, Default)]
pub struct TableRow {
    pub values: Vec<CellValue>,
}

/// Table style descriptor (metadata only — never used to compute cell styles).
#[napi(object)]
#[derive(Clone, Debug, Default)]
pub struct TableStyle {
    /// Named table style, e.g. "TableStyleMedium2" (ExcelJS `style.theme`).
    pub theme: Option<String>,
    pub show_first_column: Option<bool>,
    pub show_last_column: Option<bool>,
    pub show_row_stripes: Option<bool>,
    pub show_column_stripes: Option<bool>,
}

/// Options for `Worksheet.addTable`.
#[napi(object)]
#[derive(Clone, Debug, Default)]
pub struct AddTableOptions {
    pub name: String,
    pub display_name: Option<String>,
    /// A1 range covering header + data (+ optional totals), e.g. "A1:C4".
    #[napi(js_name = "ref")]
    pub ref_range: String,
    pub header_row: Option<bool>,
    pub totals_row: Option<bool>,
    pub columns: Vec<TableColumn>,
    /// Data rows as raw value arrays (ExcelJS-compatible: `[[v1, v2], ...]`).
    pub rows: Vec<Vec<Value>>,
    pub style: Option<TableStyle>,
    /// Optional autoFilter range for the table part. Defaults to the table ref.
    pub auto_filter: Option<String>,
}

/// A worksheet table — returned by `getTable` / `getTables`.
#[napi(object)]
#[derive(Clone, Debug, Default)]
pub struct Table {
    pub name: String,
    pub display_name: String,
    /// A1 range covering header + data (+ optional totals).
    #[napi(js_name = "ref")]
    pub ref_range: String,
    pub header_row: bool,
    pub totals_row: bool,
    pub columns: Vec<TableColumn>,
    pub rows: Vec<TableRow>,
    pub style: Option<TableStyle>,
    pub autofilter_ref: Option<String>,
}

/// Shared table list stored per worksheet (interior mutability mirrors `images`).
pub type TableList = Arc<Mutex<Vec<Table>>>;

/// Convert a `CellValue` to a plain text label (used to derive column names).
pub fn cell_text(cv: &CellValue) -> String {
    match cv.value_type.as_str() {
        "String" => cv.string.clone().unwrap_or_default(),
        "Number" => cv
            .number
            .map(|n| {
                if n.fract() == 0.0 {
                    format!("{}", n as i64)
                } else {
                    format!("{}", n)
                }
            })
            .unwrap_or_default(),
        "Boolean" => cv.boolean.map(|b| b.to_string()).unwrap_or_default(),
        _ => String::new(),
    }
}
