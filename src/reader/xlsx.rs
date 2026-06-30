//! XLSX reader — parses .xlsx files into the model layer using calamine.
//!
//! # Architecture
//! Two pairs of entry points:
//! - `workbook_inner_from_*` → return `WorkbookInner` (used by `WorkbookXlsx` I/O)
//! - `read_from_*`           → thin wrappers that wrap the inner in `Workbook` (legacy)
//!
//! # Critical caveats
//! - calamine stores formulas in a **separate API** from cell data. The reader must call
//!   `worksheet_formula()` explicitly and merge results by cell address. If you only iterate
//!   cell data, formulas are silently dropped.
//! - Shared formulas (`<f t="shared" si="0">`) are expanded to regular formulas on write in v0.1.
//! - Shared strings are resolved automatically by calamine — the reader never sees shared string
//!   indices.

use std::io::{Cursor, Read, Seek};
use std::path::Path;

use calamine::{open_workbook_auto_from_rs, Data, Reader, Sheets};

use crate::error::ExcelrsError;
use crate::model::cell::CellValue;
use crate::model::workbook::Workbook;
use crate::model::workbook_inner::WorkbookInner;

use super::styles::{self, SheetStyleMap, StyleTableRead};

// ---------------------------------------------------------------------------
// Public API — WorkbookInner variants (for WorkbookXlsx)
// ---------------------------------------------------------------------------

/// Read an .xlsx file from a byte buffer, returning a `WorkbookInner`.
///
/// Used internally by `WorkbookXlsx::read`.
pub fn workbook_inner_from_bytes(data: &[u8]) -> Result<WorkbookInner, ExcelrsError> {
    // Step 1: open calamine (for sheet count + cell data)
    let cursor = Cursor::new(data.to_vec());
    let mut workbook: Sheets<_> = open_workbook_auto_from_rs(cursor)
        .map_err(|e| ExcelrsError::Parse(format!("Failed to open workbook from buffer: {e}")))?;
    let sheet_names = workbook.sheet_names().to_owned();
    let sheet_count = sheet_names.len();

    // Step 2: parse styles + sheet cell-style maps from the same buffer via zip
    let (style_table, sheet_style_maps) = styles::parse_styles_and_sheet_maps(data, sheet_count)?;

    // Step 3: convert calamine model → excelrs model with styles
    workbook_to_inner_model(&mut workbook, &style_table, &sheet_style_maps)
}

/// Read an .xlsx file from disk, returning a `WorkbookInner`.
///
/// Used internally by `WorkbookXlsx::readFile`.
pub fn workbook_inner_from_path(path: &Path) -> Result<WorkbookInner, ExcelrsError> {
    let data = std::fs::read(path)?;
    workbook_inner_from_bytes(&data)
}

// ---------------------------------------------------------------------------
// Public API — legacy wrappers (for existing reader tests and standalone use)
// ---------------------------------------------------------------------------

/// Read an .xlsx file from a byte buffer. Returns a populated `Workbook`.
pub fn read_from_buffer(data: &[u8]) -> Result<Workbook, ExcelrsError> {
    Ok(Workbook::from_inner(workbook_inner_from_bytes(data)?))
}

/// Read an .xlsx file from disk. Returns a populated `Workbook`.
pub fn read_from_file(path: &Path) -> Result<Workbook, ExcelrsError> {
    Ok(Workbook::from_inner(workbook_inner_from_path(path)?))
}

// ---------------------------------------------------------------------------
// Internal: convert calamine model → excelrs WorkbookInner
// ---------------------------------------------------------------------------

/// Convert a calamine `Sheets<R>` workbook into a `WorkbookInner`.
///
/// Three passes per sheet:
/// 1. **Data pass:** iterate `worksheet_range().used_cells()` → set `Cell.value`
/// 2. **Style pass:** look up cellXfs index from pre-parsed sheet-style map →
///    resolve to `Style` → set on `Cell`
/// 3. **Formula pass:** iterate `worksheet_formula().used_cells()` → set `Cell.formula`
///
/// The formula pass is separate because calamine stores formulas in a different
/// data structure from cell values.  The style pass is separate because calamine
/// does not expose the `s` attribute on cells — styles are parsed from the zip
/// archive directly (see [`styles::parse_sheet_cell_styles`]).
///
/// `sheet_style_maps` is indexed by sheet index (0-based, matching the iteration
/// order of `calamine_wb.sheet_names()`).
///
/// ponytail: sheet-style-map indexing assumes sequential `sheet{N}.xml` numbering
/// matching the workbook's sheet order.  This holds for all files we write and
/// for most third-party files.  A correct fix would parse `xl/workbook.xml` to
/// map rId → file number; defer that until a real-world counterexample appears.
fn workbook_to_inner_model<R: Read + Seek>(
    calamine_wb: &mut Sheets<R>,
    style_table: &StyleTableRead,
    sheet_style_maps: &[SheetStyleMap],
) -> Result<WorkbookInner, ExcelrsError> {
    let sheet_names = calamine_wb.sheet_names().to_owned();
    let mut worksheets = Vec::with_capacity(sheet_names.len());

    for (id, name) in sheet_names.iter().enumerate() {
        let mut ws = crate::model::worksheet::Worksheet::new(name.clone());
        ws.set_id((id + 1) as u32);

        // --- Pass 1: cell data ---
        if let Ok(range) = calamine_wb.worksheet_range(name) {
            let (base_row, base_col) = range.start().unwrap_or((0, 0));
            for (row_off, col_off, cell_data) in range.used_cells() {
                // used_cells() returns offsets relative to range.start()
                let row = match u32::try_from(row_off).ok() {
                    Some(r) if r.checked_add(base_row).is_some() => base_row + r + 1,
                    _ => continue,
                };
                let col = match u32::try_from(col_off).ok() {
                    Some(c) if c.checked_add(base_col).is_some() => base_col + c + 1,
                    _ => continue,
                };
                let cell_value = map_data(cell_data);
                ws.insert_cell_value(row, col, cell_value);

                // --- Pass 2: cell style (attached during the same cell walk) ---
                if let Some(map) = sheet_style_maps.get(id) {
                    if let Some(&xf_idx) = map.get(&(row, col)) {
                        if let Some(style) = style_table.resolve_style(xf_idx) {
                            ws.insert_cell_style(row, col, style);
                        }
                    }
                }
            }
        }

        // --- Pass 2: formulas (separate API) ---
        // If this fails, cells still have their values — formulas are best-effort.
        if let Ok(formulas) = calamine_wb.worksheet_formula(name) {
            let (base_row, base_col) = formulas.start().unwrap_or((0, 0));
            for (row_off, col_off, formula) in formulas.used_cells() {
                if !formula.is_empty() {
                    let row = match u32::try_from(row_off).ok() {
                        Some(r) if r.checked_add(base_row).is_some() => base_row + r + 1,
                        _ => continue,
                    };
                    let col = match u32::try_from(col_off).ok() {
                        Some(c) if c.checked_add(base_col).is_some() => base_col + c + 1,
                        _ => continue,
                    };
                    ws.insert_cell_formula(row, col, formula.clone());
                }
            }
        }

        worksheets.push(ws);
    }

    let mut inner = WorkbookInner::new();
    inner.set_worksheets(worksheets);
    Ok(inner)
}

/// Map a calamine `Data` enum variant to an excelrs `CellValue`.
fn map_data(data: &Data) -> CellValue {
    match data {
        Data::Empty => CellValue::default(),
        Data::Int(n) => CellValue::number(*n as f64),
        Data::Float(f) => CellValue::number(*f),
        Data::String(s) => CellValue::string(s.clone()),
        Data::Bool(b) => CellValue::boolean(*b),
        Data::DateTime(dt) => {
            // v0.1: Date is stored as an ISO-8601 formatted string.
            let (y, m, d, hh, mm, ss, ms) = dt.to_ymd_hms_milli();
            CellValue::string(format!(
                "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}",
                y, m, d, hh, mm, ss, ms
            ))
        }
        Data::DateTimeIso(s) => CellValue::string(s.clone()),
        Data::DurationIso(s) => CellValue::string(s.clone()),
        Data::Error(e) => {
            let msg = format!("{:?}", e);
            CellValue {
                value_type: "Error".into(),
                error_value: Some(msg),
                ..Default::default()
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- map_data unit tests (no file I/O) --

    #[test]
    fn test_map_data_empty() {
        let result = map_data(&Data::Empty);
        assert_eq!(result.value_type, "Null");
        assert!(result.number.is_none());
        assert!(result.string.is_none());
        assert!(result.boolean.is_none());
        assert!(result.formula.is_none());
        assert!(result.error_value.is_none());
    }

    #[test]
    fn test_map_data_int() {
        let result = map_data(&Data::Int(42));
        assert_eq!(result.value_type, "Number");
        assert_eq!(result.number, Some(42.0));
    }

    #[test]
    fn test_map_data_float() {
        let result = map_data(&Data::Float(std::f64::consts::PI));
        assert_eq!(result.value_type, "Number");
        assert_eq!(result.number, Some(std::f64::consts::PI));
    }

    #[test]
    fn test_map_data_string() {
        let result = map_data(&Data::String("hello".into()));
        assert_eq!(result.value_type, "String");
        assert_eq!(result.string, Some("hello".into()));
    }

    #[test]
    fn test_map_data_bool() {
        let result = map_data(&Data::Bool(true));
        assert_eq!(result.value_type, "Boolean");
        assert_eq!(result.boolean, Some(true));

        let result = map_data(&Data::Bool(false));
        assert_eq!(result.value_type, "Boolean");
        assert_eq!(result.boolean, Some(false));
    }

    #[test]
    fn test_map_data_date_time() {
        use calamine::{ExcelDateTime, ExcelDateTimeType};
        let dt = ExcelDateTime::new(45943.541, ExcelDateTimeType::DateTime, false);
        let result = map_data(&Data::DateTime(dt));
        assert_eq!(result.value_type, "String");
        let s = result.string.unwrap();
        assert!(s.starts_with("2025-10-13"), "expected 2025-10-13 date, got {s}");
    }

    #[test]
    fn test_map_data_date_time_iso() {
        let result = map_data(&Data::DateTimeIso("2025-10-13T12:00:00Z".into()));
        assert_eq!(result.value_type, "String");
        assert_eq!(result.string, Some("2025-10-13T12:00:00Z".into()));
    }

    #[test]
    fn test_map_data_duration_iso() {
        let result = map_data(&Data::DurationIso("PT12H30M".into()));
        assert_eq!(result.value_type, "String");
        assert_eq!(result.string, Some("PT12H30M".into()));
    }

    #[test]
    fn test_map_data_error() {
        use calamine::CellErrorType;
        let result = map_data(&Data::Error(CellErrorType::Div0));
        assert_eq!(result.value_type, "Error");
        assert!(result.error_value.is_some());
        let msg = result.error_value.unwrap();
        assert!(!msg.is_empty(), "error message should not be empty");
    }

    #[test]
    fn test_map_data_error_na() {
        use calamine::CellErrorType;
        let result = map_data(&Data::Error(CellErrorType::NA));
        assert_eq!(result.value_type, "Error");
        assert!(result.error_value.is_some());
    }

    // -- read errors (no real xlsx available) --

    #[test]
    fn test_read_from_buffer_invalid_data() {
        let result = read_from_buffer(b"not an xlsx file");
        assert!(result.is_err());
        match result {
            Err(ExcelrsError::Parse(msg)) => {
                assert!(!msg.is_empty(), "Parse error should have a message");
            }
            other => panic!("Expected Parse error, got: {other:?}"),
        }
    }

    #[test]
    fn test_read_from_file_nonexistent() {
        let result = read_from_file(Path::new("/nonexistent/file.xlsx"));
        assert!(result.is_err());
    }

    // -- WorkbookInner entry points --

    #[test]
    fn test_workbook_inner_from_bytes_invalid() {
        let result = workbook_inner_from_bytes(b"not an xlsx file");
        assert!(result.is_err());
    }

    #[test]
    fn test_workbook_inner_from_bytes_valid_minimal() {
        // Build a minimal xlsx and verify it parses
        let bytes = make_minimal_xlsx();
        let inner = workbook_inner_from_bytes(&bytes).unwrap();
        assert_eq!(inner.worksheet_count(), 1);
        assert_eq!(inner.worksheets()[0].name(), "Sheet1");
    }

    #[test]
    fn test_workbook_inner_from_path_nonexistent() {
        let result = workbook_inner_from_path(Path::new("/nonexistent/file.xlsx"));
        assert!(result.is_err());
    }

    // -- helpers --

    fn make_minimal_xlsx() -> Vec<u8> {
        use std::io::Write;

        let mut buf = Vec::new();
        {
            let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            let options: zip::write::FileOptions<'_, ()> =
                zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

            zip.start_file("[Content_Types].xml", options).unwrap();
            write!(
                zip,
                r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>
  <Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>
  <Override PartName="/xl/sharedStrings.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sharedStrings+xml"/>
</Types>"#
            )
            .unwrap();

            zip.start_file("_rels/.rels", options).unwrap();
            write!(
                zip,
                r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>
</Relationships>"#
            )
            .unwrap();

            zip.start_file("xl/workbook.xml", options).unwrap();
            write!(
                zip,
                r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
          xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <sheets>
    <sheet name="Sheet1" sheetId="1" r:id="rId1"/>
  </sheets>
</workbook>"#
            )
            .unwrap();

            zip.start_file("xl/_rels/workbook.xml.rels", options).unwrap();
            write!(
                zip,
                r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>
</Relationships>"#
            )
            .unwrap();

            zip.start_file("xl/worksheets/sheet1.xml", options).unwrap();
            write!(
                zip,
                r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheetData>
    <row r="1">
      <c r="A1" t="inlineStr"><is><t>hello</t></is></c>
    </row>
  </sheetData>
</worksheet>"#
            )
            .unwrap();

            zip.start_file("xl/sharedStrings.xml", options).unwrap();
            write!(
                zip,
                r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" count="0" uniqueCount="0"/>
"#
            )
            .unwrap();

            zip.finish().unwrap();
        }
        buf
    }
}
