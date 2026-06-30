//! XLSX writer — serializes the excelrs model into an .xlsx byte buffer using
//! the `zip` crate for the OOXML archive and `quick-xml` for string escaping.
//!
//! # Entry points
//! - `workbook_to_bytes(&WorkbookInner) -> Result<Vec<u8>>` — in-memory .xlsx
//! - `workbook_to_path(&WorkbookInner, &Path) -> Result<()>` — write to disk
//!
//! # What gets written (v0.1)
//! - `[Content_Types].xml`
//! - `_rels/.rels`
//! - `xl/workbook.xml` + `xl/_rels/workbook.xml.rels`
//! - `xl/worksheets/sheet{N}.xml` (one per sheet, with `<dimension>` and `<sheetData>`)
//! - `xl/sharedStrings.xml` (deduplicated string table)
//! - `xl/styles.xml` (v0.2.0: full dedup'd style table; see `styles.rs`)
//!
//! # v0.1 limitations (per spec)
//! - No column width/properties preserved
//! - No merged cells
//! - No custom styles beyond Normal
//! - Formula cells write the formula string but no cached value

use std::collections::{BTreeMap, HashMap};
use std::io::{Seek, Write};
use std::path::Path;

use quick_xml::escape::escape;

use crate::error::ExcelrsError;
use crate::model::cell::Cell;
use crate::model::style::Style;
use crate::model::workbook_inner::WorkbookInner;
use crate::model::worksheet::Worksheet;

use super::styles;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Serialise `inner` to an in-memory .xlsx byte buffer.
pub fn workbook_to_bytes(inner: &WorkbookInner) -> Result<Vec<u8>, ExcelrsError> {
    let mut buf = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));

        let worksheets = if inner.worksheets.is_empty() {
            // Emit a single default "Sheet1" (Excel convention)
            vec![make_default_sheet()]
        } else {
            inner.worksheets.clone()
        };
        let sheet_count = worksheets.len();

        // --- Pass 1: build the shared strings table ---
        let (string_table, string_indices) = build_shared_strings(&worksheets);

        // --- Write all OOXML parts ---

        // [Content_Types].xml
        start_file(&mut zip, "[Content_Types].xml")?;
        write_content_types(&mut zip, sheet_count)?;

        // _rels/.rels
        start_file(&mut zip, "_rels/.rels")?;
        write_rels_rels(&mut zip)?;

        // xl/workbook.xml
        start_file(&mut zip, "xl/workbook.xml")?;
        write_workbook_xml(&mut zip, &worksheets)?;

        // xl/_rels/workbook.xml.rels
        start_file(&mut zip, "xl/_rels/workbook.xml.rels")?;
        write_workbook_rels(&mut zip, sheet_count)?;

        // xl/sharedStrings.xml
        start_file(&mut zip, "xl/sharedStrings.xml")?;
        write_shared_strings(&mut zip, &string_table)?;

        // xl/styles.xml (v0.2.0: full dedup'd style table)
        start_file(&mut zip, "xl/styles.xml")?;
        // Collect effective styles across every worksheet
        // Precedence: cell-level wins, then column-level, then Normal (None).
        let all_styles: Vec<Option<Style>> = worksheets
            .iter()
            .flat_map(|ws| {
                let col_style_map: BTreeMap<u32, Option<Style>> =
                    ws.columns().iter().map(|c| (c.col_num(), c.style())).collect();
                ws.rows().into_iter().flat_map(move |row| {
                    let col_style_map = &col_style_map;
                    row.sorted_cells()
                        .into_iter()
                        .map(move |c| effective_cell_style_with_fallback(c, col_style_map))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        let style_table = styles::build_style_table(&all_styles);
        styles::emit_styles_xml(&mut zip, &style_table)?;

        // xl/worksheets/sheet{N}.xml
        let mut style_offset = 0usize;
        for (i, ws) in worksheets.iter().enumerate() {
            let sheet_path = format!("xl/worksheets/sheet{}.xml", i + 1);
            start_file(&mut zip, &sheet_path)?;

            // Count cells in this worksheet for the style-indices slice
            let cell_count: usize = ws.rows().iter().map(|r| r.sorted_cells().len()).sum();
            let ws_indices = &style_table.cell_indices[style_offset..style_offset + cell_count];
            style_offset += cell_count;

            write_sheet_xml(&mut zip, ws, &string_indices, ws_indices)?;
        }

        zip.finish()
            .map_err(|e| ExcelrsError::Write(format!("Failed to finalise zip: {e}")))?;
    }
    Ok(buf)
}

/// Serialise `inner` to an .xlsx file on disk.
pub fn workbook_to_path(inner: &WorkbookInner, path: &Path) -> Result<(), ExcelrsError> {
    let bytes = workbook_to_bytes(inner)?;
    std::fs::write(path, &bytes)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Start a new file inside the zip archive with standard deflate options.
fn start_file<W: Write + Seek>(zip: &mut zip::ZipWriter<W>, name: &str) -> Result<(), ExcelrsError> {
    let options: zip::write::FileOptions<'_, ()> =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    zip.start_file(name, options)
        .map_err(|e| ExcelrsError::Write(format!("Failed to write '{name}': {e}")))
}

/// Create a default "Sheet1" worksheet (used when the workbook is empty).
fn make_default_sheet() -> Worksheet {
    let mut ws = Worksheet::new("Sheet1".into());
    ws.set_id(1);
    ws
}

// ---------------------------------------------------------------------------
// Column-style helpers (A7)
// ---------------------------------------------------------------------------

/// Resolve the effective style for a cell: cell-style wins; else column-style;
/// else None (Normal).
///
/// Takes a pre-computed column-style map keyed by `col_num` to avoid calling
/// `ws.columns()` per cell.  Cells with no matching column entry get Normal.
fn effective_cell_style_with_fallback(cell: &Cell, col_style_map: &BTreeMap<u32, Option<Style>>) -> Option<Style> {
    match cell.style() {
        Some(s) if !s.is_empty() => Some(s),
        _ => col_style_map.get(&cell.col()).and_then(|s| s.clone()),
    }
}

// ---------------------------------------------------------------------------
// Shared strings table
// ---------------------------------------------------------------------------

/// Walk all worksheets and deduplicate string values.
///
/// Returns `(string_table, string_indices)` where:
/// - `string_table` is an index-ordered `Vec<String>` suitable for
///   `xl/sharedStrings.xml`
/// - `string_indices` is a `HashMap<String, u32>` for fast look-up when
///   writing cell references as `<c t="s"><v>idx</v></c>`
fn build_shared_strings(worksheets: &[Worksheet]) -> (Vec<String>, HashMap<String, u32>) {
    let mut string_table: Vec<String> = Vec::new();
    let mut string_indices: HashMap<String, u32> = HashMap::new();

    for ws in worksheets {
        for row in ws.rows() {
            for cell in row.sorted_cells() {
                let cv = cell.value();
                if cv.value_type == "String" {
                    if let Some(s) = cv.string {
                        string_indices.entry(s.clone()).or_insert_with(|| {
                            let idx = string_table.len() as u32;
                            string_table.push(s);
                            idx
                        });
                    }
                }
            }
        }
    }

    (string_table, string_indices)
}

// ---------------------------------------------------------------------------
// [Content_Types].xml
// ---------------------------------------------------------------------------

fn write_content_types<W: Write>(w: &mut W, sheet_count: usize) -> Result<(), ExcelrsError> {
    write_str(w, r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#)?;
    write_str(
        w,
        r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">"#,
    )?;
    write_str(
        w,
        r#"<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>"#,
    )?;
    write_str(w, r#"<Default Extension="xml" ContentType="application/xml"/>"#)?;
    write_str(
        w,
        r#"<Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>"#,
    )?;
    for i in 1..=sheet_count {
        write_str(
            w,
            &format!(
                r#"<Override PartName="/xl/worksheets/sheet{i}.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>"#,
            ),
        )?;
    }
    write_str(
        w,
        r#"<Override PartName="/xl/sharedStrings.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sharedStrings+xml"/>"#,
    )?;
    write_str(
        w,
        r#"<Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/>"#,
    )?;
    write_str(w, "</Types>")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// _rels/.rels
// ---------------------------------------------------------------------------

fn write_rels_rels<W: Write>(w: &mut W) -> Result<(), ExcelrsError> {
    write_str(w, r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#)?;
    write_str(
        w,
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
    )?;
    write_str(
        w,
        r#"<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>"#,
    )?;
    write_str(w, "</Relationships>")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// xl/workbook.xml
// ---------------------------------------------------------------------------

fn write_workbook_xml<W: Write>(w: &mut W, worksheets: &[Worksheet]) -> Result<(), ExcelrsError> {
    write_str(w, r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#)?;
    write_str(
        w,
        r#"<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">"#,
    )?;
    write_str(w, "<sheets>")?;
    for (i, ws) in worksheets.iter().enumerate() {
        let name = ws.name();
        let name_esc = escape(&name);
        // rId must match workbook.xml.rels: rId1=styles, rId2=sharedStrings, rId3+=worksheets
        let rid = i + 3;
        write_str(
            w,
            &format!(
                r#"<sheet name="{}" sheetId="{}" r:id="rId{}"/>"#,
                name_esc,
                ws.id(),
                rid
            ),
        )?;
    }
    write_str(w, "</sheets>")?;
    write_str(w, "</workbook>")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// xl/_rels/workbook.xml.rels
// ---------------------------------------------------------------------------

fn write_workbook_rels<W: Write>(w: &mut W, sheet_count: usize) -> Result<(), ExcelrsError> {
    write_str(w, r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#)?;
    write_str(
        w,
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
    )?;
    write_str(
        w,
        r#"<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>"#,
    )?;
    write_str(
        w,
        r#"<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings" Target="sharedStrings.xml"/>"#,
    )?;
    for i in 1..=sheet_count {
        write_str(
            w,
            &format!(
                r#"<Relationship Id="rId{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet{i}.xml"/>"#,
                i + 2, // rId1=styles, rId2=sharedStrings, rId3+=worksheets
            ),
        )?;
    }
    write_str(w, "</Relationships>")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// xl/sharedStrings.xml
// ---------------------------------------------------------------------------

fn write_shared_strings<W: Write>(w: &mut W, string_table: &[String]) -> Result<(), ExcelrsError> {
    let count = string_table.len();
    write_str(w, r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#)?;
    write_str(
        w,
        &format!(
            r#"<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" count="{}" uniqueCount="{}">"#,
            count, count
        ),
    )?;
    for s in string_table {
        // xml:space="preserve" when the string has leading/trailing whitespace
        if s.starts_with(' ') || s.ends_with(' ') {
            write_str(w, &format!("<si><t xml:space=\"preserve\">{}</t></si>", escape(s)))?;
        } else {
            write_str(w, &format!("<si><t>{}</t></si>", escape(s)))?;
        }
    }
    write_str(w, "</sst>")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// xl/worksheets/sheet{N}.xml
// ---------------------------------------------------------------------------

fn write_sheet_xml<W: Write>(
    w: &mut W,
    ws: &Worksheet,
    string_indices: &HashMap<String, u32>,
    style_indices: &[u32],
) -> Result<(), ExcelrsError> {
    write_str(w, r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#)?;
    write_str(
        w,
        r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">"#,
    )?;

    // <dimension ref="A1:Z1000"/> — used range
    let dimension = compute_dimension(ws);
    if let Some(dim) = dimension {
        write_str(w, &format!("<dimension ref=\"{}\"/>", dim))?;
    }

    write_str(w, "<sheetData>")?;

    write_cells_with_styles(w, ws, string_indices, style_indices)?;

    write_str(w, "</sheetData>")?;
    write_str(w, "</worksheet>")?;
    Ok(())
}

/// Iterate a worksheet's cells in order, writing `<row>` and `<c>` elements
/// with the style index at each cell.  Returns `Err` if `style_indices` is
/// exhausted before the last cell (writer internal invariant).
fn write_cells_with_styles<W: Write>(
    w: &mut W,
    ws: &Worksheet,
    string_indices: &HashMap<String, u32>,
    style_indices: &[u32],
) -> Result<(), ExcelrsError> {
    let mut si = style_indices.iter();
    for row in ws.rows() {
        write!(w, r#"<row r="{}">"#, row.number())?;
        for cell in row.sorted_cells() {
            let style_idx = si
                .next()
                .copied()
                .ok_or_else(|| ExcelrsError::Write("style_indices exhausted mid-sheet (writer bug)".into()))?;
            write_cell_xml(w, cell, string_indices, style_idx)?;
        }
        write_str(w, "</row>")?;
    }
    Ok(())
}

/// Write a single `<c>` element.
fn write_cell_xml<W: Write>(
    w: &mut W,
    cell: &crate::model::cell::Cell,
    string_indices: &HashMap<String, u32>,
    style_index: u32,
) -> Result<(), ExcelrsError> {
    let cv = cell.value();
    let address = cell.address();
    let formula = cell.formula();

    // Open the cell element with style attribute
    write!(w, r#"<c r="{}" s="{}""#, address, style_index)?;

    // Determine cell type and write value attribute
    let cell_type_attr = match cv.value_type.as_str() {
        "String" => Some("t=\"s\""),
        "Boolean" => Some("t=\"b\""),
        "Error" => Some("t=\"e\""),
        _ => None, // Number, Null, Formula (no type attr)
    };

    if let Some(attr) = cell_type_attr {
        write!(w, " {}", attr)?;
    }

    write_str(w, ">")?;

    // Formula element (if present)
    if let Some(f) = &formula {
        if !f.is_empty() {
            write_str(w, &format!("<f>{}</f>", escape(f)))?;
        }
    }

    // Value element (skip Null cells — Excel interprets absence as empty)
    match cv.value_type.as_str() {
        "Number" => {
            if let Some(n) = cv.number {
                write_str(w, &format!("<v>{}</v>", n))?;
            }
        }
        "String" => {
            if let Some(s) = &cv.string {
                if let Some(idx) = string_indices.get(s) {
                    write_str(w, &format!("<v>{}</v>", idx))?;
                }
            }
        }
        "Boolean" => {
            let v = if cv.boolean.unwrap_or(false) { "1" } else { "0" };
            write_str(w, &format!("<v>{}</v>", v))?;
        }
        "Error" => {
            if let Some(e) = &cv.error_value {
                write_str(w, &format!("<v>{}</v>", escape(e)))?;
            }
        }
        "Formula" => {
            // The value was already written as the <f> element above
            // If there's also a cached value, write it
            if let Some(n) = cv.number {
                write_str(w, &format!("<v>{}</v>", n))?;
            }
        }
        _ => {}
    }

    write_str(w, "</c>")?;
    Ok(())
}

/// Compute the `<dimension ref="...">` string for a worksheet.
/// Returns `None` if the sheet has no cells.
fn compute_dimension(ws: &Worksheet) -> Option<String> {
    let mut min_row = u32::MAX;
    let mut max_row = 0u32;
    let mut min_col = u32::MAX;
    let mut max_col = 0u32;
    let mut has_cells = false;

    for row in ws.rows() {
        let r = row.number();
        if row.cell_count() > 0 {
            if r < min_row {
                min_row = r;
            }
            if r > max_row {
                max_row = r;
            }
            // Find min_col per row
            for cell in row.sorted_cells() {
                let c = cell.col();
                if c < min_col {
                    min_col = c;
                }
                if c > max_col {
                    max_col = c;
                }
            }
            has_cells = true;
        }
    }

    if !has_cells {
        return None;
    }

    let start = crate::types::address_to_string(min_col, min_row).unwrap_or_else(|_| format!("R{min_row}C{min_col}"));
    let end = crate::types::address_to_string(max_col, max_row).unwrap_or_else(|_| format!("R{max_row}C{max_col}"));
    Some(format!("{start}:{end}"))
}

// ---------------------------------------------------------------------------
// Utility
// ---------------------------------------------------------------------------

/// Write a string to the output, propagating errors.
fn write_str<W: Write>(w: &mut W, s: &str) -> Result<(), ExcelrsError> {
    w.write_all(s.as_bytes())
        .map_err(|e| ExcelrsError::Write(format!("Write error: {e}")))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::cell::CellValue;
    use crate::model::workbook_inner::WorkbookInner;
    use crate::reader::xlsx::workbook_inner_from_bytes;
    use std::collections::BTreeMap;

    // ---- writer unit tests ----

    #[test]
    fn test_write_empty_workbook() {
        let inner = WorkbookInner::new();
        let bytes = workbook_to_bytes(&inner).expect("workbook_to_bytes should succeed");
        assert!(!bytes.is_empty(), "should produce non-empty bytes");

        // Write to temp file for external inspection if test fails
        let tmp = std::env::temp_dir().join("excelrs_debug_empty.xlsx");
        let _ = std::fs::write(&tmp, &bytes);

        eprintln!("DEBUG: wrote {} bytes to {:?}", bytes.len(), tmp);
        eprintln!("DEBUG: first 8 bytes: {:02x?}", &bytes[..bytes.len().min(8)]);

        // Verify it can be read back
        match workbook_inner_from_bytes(&bytes) {
            Ok(re_read) => {
                assert_eq!(re_read.worksheet_count(), 1);
                assert_eq!(re_read.worksheets()[0].name(), "Sheet1");
            }
            Err(e) => {
                panic!("Read-back failed: {e}");
            }
        }

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_write_single_sheet() {
        let inner = build_test_workbook();
        let bytes = workbook_to_bytes(&inner).unwrap();
        assert!(!bytes.is_empty());

        // Verify re-read
        let re_read = workbook_inner_from_bytes(&bytes).unwrap();
        assert_eq!(re_read.worksheet_count(), 1);
        let ws = &re_read.worksheets()[0];
        assert_eq!(ws.name(), "Test");

        // Check cell values
        let a1 = ws.get_cell_by_address("A1".into());
        assert_eq!(a1.value().value_type, "Number");
        assert_eq!(a1.value().number, Some(42.0));

        let b1 = ws.get_cell_by_address("B1".into());
        assert_eq!(b1.value().value_type, "String");
        assert_eq!(b1.value().string.as_deref(), Some("Hello"));

        let c1 = ws.get_cell_by_address("C1".into());
        assert_eq!(c1.value().value_type, "Boolean");
        assert_eq!(c1.value().boolean, Some(true));

        let a2 = ws.get_cell_by_address("A2".into());
        assert_eq!(a2.value().value_type, "Number");
        assert_eq!(a2.value().number, Some(std::f64::consts::PI));
    }

    #[test]
    fn test_write_multi_sheet() {
        let mut inner = WorkbookInner::new();
        inner.add_worksheet("First".into());
        inner.add_worksheet("Second".into());

        // Write to second sheet
        if let Some(ws) = inner.worksheets.get_mut(1) {
            ws.add_row(vec![serde_json::json!("data")]);
        }

        let bytes = workbook_to_bytes(&inner).unwrap();
        let re_read = workbook_inner_from_bytes(&bytes).unwrap();
        assert_eq!(re_read.worksheet_count(), 2);
        assert_eq!(re_read.worksheets()[0].name(), "First");
        assert_eq!(re_read.worksheets()[1].name(), "Second");

        let ws2 = &re_read.worksheets()[1];
        let a1 = ws2.get_cell_by_address("A1".into());
        assert_eq!(a1.value().string.as_deref(), Some("data"));
    }

    #[test]
    fn test_write_formula_cell() {
        let mut inner = WorkbookInner::new();
        let mut ws = Worksheet::new("Formulas".into());
        ws.set_id(1);

        // Add rows with number values and formula
        ws.insert_cell_value(1, 1, CellValue::number(10.0));
        ws.insert_cell_value(2, 1, CellValue::number(20.0));
        ws.insert_cell_value(3, 1, CellValue::number(30.0));
        ws.insert_cell_formula(3, 1, "SUM(A1:A2)".into());

        inner.worksheets.push(ws);

        let bytes = workbook_to_bytes(&inner).unwrap();
        let re_read = workbook_inner_from_bytes(&bytes).unwrap();

        let ws = &re_read.worksheets()[0];
        let a3 = ws.get_cell_by_address("A3".into());
        assert!(a3.formula().is_some(), "formula should be preserved");
        let f = a3.formula().unwrap().to_uppercase();
        assert!(f.contains("SUM"), "formula content should match, got: {f}");
    }

    #[test]
    fn test_write_shared_string_dedup() {
        let mut ws = Worksheet::new("Dedup".into());
        ws.set_id(1);

        // Same string in multiple cells
        ws.add_row(vec![
            serde_json::json!("apple"),
            serde_json::json!("banana"),
            serde_json::json!("apple"), // dup
        ]);

        let mut inner = WorkbookInner::new();
        inner.worksheets.push(ws);

        let bytes = workbook_to_bytes(&inner).unwrap();
        let re_read = workbook_inner_from_bytes(&bytes).unwrap();

        let ws = &re_read.worksheets()[0];
        assert_eq!(
            ws.get_cell_by_address("A1".into()).value().string.as_deref(),
            Some("apple")
        );
        assert_eq!(
            ws.get_cell_by_address("B1".into()).value().string.as_deref(),
            Some("banana")
        );
        assert_eq!(
            ws.get_cell_by_address("C1".into()).value().string.as_deref(),
            Some("apple")
        );
    }

    // ---- round-trip tests ----

    #[test]
    fn test_round_trip_write_read() {
        let mut inner = WorkbookInner::new();
        let mut ws = Worksheet::new("RoundTrip".into());
        ws.set_id(1);
        ws.add_row(vec![
            serde_json::json!("Name"),
            serde_json::json!("Age"),
            serde_json::json!("Active"),
        ]);
        ws.add_row(vec![
            serde_json::json!("Alice"),
            serde_json::json!(30),
            serde_json::json!(true),
        ]);
        inner.worksheets.push(ws);

        let bytes = workbook_to_bytes(&inner).unwrap();
        let re_read = workbook_inner_from_bytes(&bytes).unwrap();

        assert_eq!(re_read.worksheet_count(), 1);
        let ws = &re_read.worksheets()[0];
        assert_eq!(ws.name(), "RoundTrip");
        assert_eq!(ws.row_count(), 2);

        // Row 1
        let r1c1 = ws.get_cell_by_address("A1".into());
        assert_eq!(r1c1.value().string.as_deref(), Some("Name"));
        let r1c2 = ws.get_cell_by_address("B1".into());
        assert_eq!(r1c2.value().string.as_deref(), Some("Age"));
        let r1c3 = ws.get_cell_by_address("C1".into());
        assert_eq!(r1c3.value().string.as_deref(), Some("Active"));

        // Row 2
        let r2c1 = ws.get_cell_by_address("A2".into());
        assert_eq!(r2c1.value().string.as_deref(), Some("Alice"));
        let r2c2 = ws.get_cell_by_address("B2".into());
        assert_eq!(r2c2.value().number, Some(30.0));
        let r2c3 = ws.get_cell_by_address("C2".into());
        assert_eq!(r2c3.value().boolean, Some(true));
    }

    #[test]
    fn test_write_to_file_and_read_back() {
        let mut inner = WorkbookInner::new();
        inner.add_worksheet("FileTest".into());

        let tmp = std::env::temp_dir().join(format!(
            "excelrs_write_test_{}.xlsx",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        workbook_to_path(&inner, &tmp).unwrap();
        assert!(tmp.exists(), "file should exist");

        // Read back
        let re_read = workbook_inner_from_path(&tmp).unwrap();
        assert_eq!(re_read.worksheet_count(), 1);
        assert_eq!(re_read.worksheets()[0].name(), "FileTest");

        // Clean up
        let _ = std::fs::remove_file(&tmp);
    }

    // ---- s="<idx>" attribute tests ----

    /// Normal cells (no style) get s="0" in the written sheet XML.
    #[test]
    fn test_normal_cell_has_s_attr() {
        let inner = build_test_workbook();
        let bytes = workbook_to_bytes(&inner).unwrap();

        // Extract sheet1.xml from the zip
        use std::io::Cursor;
        use std::io::Read;
        let mut archive = zip::read::ZipArchive::new(Cursor::new(&bytes)).unwrap();
        let mut sheet_xml = String::new();
        archive
            .by_name("xl/worksheets/sheet1.xml")
            .unwrap()
            .read_to_string(&mut sheet_xml)
            .unwrap();

        // All cells should have s="0" (Normal)
        assert!(sheet_xml.contains(r#"<c r="A1" s="0""#));
        assert!(sheet_xml.contains(r#"<c r="B1" s="0" t="s""#));
        assert!(sheet_xml.contains(r#"<c r="C1" s="0" t="b""#));
        assert!(sheet_xml.contains(r#"<c r="A2" s="0""#));
    }

    /// Direct test: write_cell_xml emits s="<idx>" with the given style index.
    #[test]
    fn test_write_cell_xml_emits_style_index() {
        use crate::model::cell::Cell;
        use std::collections::HashMap;

        let mut buf = Vec::new();
        let cell = Cell::new("A1".into(), 1, 1);
        let string_indices = HashMap::new();
        write_cell_xml(&mut buf, &cell, &string_indices, 42).unwrap();
        let xml = String::from_utf8(buf).unwrap();
        assert!(xml.contains(r#"s="42""#), "expected s=\"42\" in cell XML, got: {xml}");
    }

    // ---- A7: column-level style fallback tests ----

    /// Column style applies to cells in that column that have no explicit style.
    #[test]
    fn test_column_style_applies_to_cells() {
        use crate::model::column::Column;

        let mut ws = Worksheet::new("Col".into());
        ws.set_id(1);

        // Column A has its own style
        let mut col_a = Column::new("A".into(), "a".into(), 10.0);
        col_a.set_style(serde_json::json!({ "num_fmt": "0.00%" })).unwrap();
        ws.set_columns(serde_json::to_value(&[col_a]).unwrap()).unwrap();

        ws.add_row(vec![serde_json::json!(0.123)]); // A1, gets column style
        ws.add_row(vec![serde_json::json!(0.456)]); // A2, gets column style

        let mut inner = WorkbookInner::new();
        inner.worksheets.push(ws);
        let bytes = workbook_to_bytes(&inner).unwrap();

        use std::io::Cursor;
        use std::io::Read;
        let mut archive = zip::read::ZipArchive::new(Cursor::new(&bytes)).unwrap();
        let mut sheet_xml = String::new();
        archive
            .by_name("xl/worksheets/sheet1.xml")
            .unwrap()
            .read_to_string(&mut sheet_xml)
            .unwrap();

        // Both cells share the same column-style cellXfs (index 1)
        assert!(
            sheet_xml.contains(r#"<c r="A1" s="1""#),
            "A1 should get column-style s=1"
        );
        assert!(
            sheet_xml.contains(r#"<c r="A2" s="1""#),
            "A2 should get column-style s=1, same index as A1"
        );
    }

    /// Cell-level style overrides column-level style — verify via helper directly.
    #[test]
    fn test_effective_cell_style_precedence() {
        use crate::model::style::{Font, Style};

        let bold_col = Style {
            font: Some(Font {
                bold: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        };

        // Cell with explicit style → wins over column
        let mut cell = Cell::new("A1".into(), 1, 1);
        cell.set_style(serde_json::json!({ "num_fmt": "0.00%" })).unwrap();
        let map: BTreeMap<u32, Option<Style>> = [(1u32, Some(bold_col.clone()))].into();
        let result = effective_cell_style_with_fallback(&cell, &map);
        assert!(result.is_some());
        assert_eq!(
            result.unwrap().num_fmt,
            Some("0.00%".into()),
            "cell style should win over column style"
        );

        // Cell with no style → falls back to column (col=1 matched)
        let cell2 = Cell::new("A1".into(), 1, 1);
        let result2 = effective_cell_style_with_fallback(&cell2, &map);
        assert!(result2.is_some());
        assert_eq!(
            result2.unwrap().font.unwrap().bold,
            Some(true),
            "column style should apply when cell has no style"
        );

        // Cell with no style, column also no style → None (Normal)
        let cell3 = Cell::new("A1".into(), 1, 1);
        let empty_map: BTreeMap<u32, Option<Style>> = [(1u32, None), (2u32, None), (3u32, None)].into();
        let result3 = effective_cell_style_with_fallback(&cell3, &empty_map);
        assert!(result3.is_none(), "no cell or column style → Normal");

        // Cell in column 2, but map only has col_num=1 → no column fallback
        let cell4 = Cell::new("B1".into(), 1, 2);
        let result4 = effective_cell_style_with_fallback(&cell4, &map);
        assert!(result4.is_none(), "column 2 missing from map → no fallback");
    }

    /// Cell outside the defined columns array gets Normal (s="0").
    #[test]
    fn test_cell_outside_columns_uses_normal() {
        let mut ws = Worksheet::new("Outside".into());
        ws.set_id(1);
        // Empty columns array — no column styles
        ws.set_columns(serde_json::json!([])).unwrap();

        ws.add_row(vec![
            serde_json::json!(1),
            serde_json::json!(2),
            serde_json::json!(3),
            serde_json::json!(4),
            serde_json::json!(5), // E1 = col 5, beyond any column definitions
        ]);

        let mut inner = WorkbookInner::new();
        inner.worksheets.push(ws);
        let bytes = workbook_to_bytes(&inner).unwrap();

        use std::io::Cursor;
        use std::io::Read;
        let mut archive = zip::read::ZipArchive::new(Cursor::new(&bytes)).unwrap();
        let mut sheet_xml = String::new();
        archive
            .by_name("xl/worksheets/sheet1.xml")
            .unwrap()
            .read_to_string(&mut sheet_xml)
            .unwrap();

        // All cells should be Normal (empty column styles → no column-level fallback)
        // Note: all 5 values are numbers, so no t="s" or t="b" attributes
        assert!(sheet_xml.contains(r#"<c r="A1" s="0">"#));
        assert!(sheet_xml.contains(r#"<c r="B1" s="0">"#));
        assert!(sheet_xml.contains(r#"<c r="C1" s="0">"#));
        assert!(sheet_xml.contains(r#"<c r="D1" s="0">"#));
        assert!(sheet_xml.contains(r#"<c r="E1" s="0">"#));
    }

    /// Column with empty (default) style is treated as no column style.
    #[test]
    fn test_column_empty_style_is_normal() {
        use crate::model::column::Column;

        let mut ws = Worksheet::new("Empty".into());
        ws.set_id(1);

        // Column A with a Style::default() (all None)
        let col_a = Column::new("A".into(), "a".into(), 10.0);
        ws.set_columns(serde_json::to_value(&[col_a]).unwrap()).unwrap();

        ws.add_row(vec![serde_json::json!(42)]); // A1

        let mut inner = WorkbookInner::new();
        inner.worksheets.push(ws);
        let bytes = workbook_to_bytes(&inner).unwrap();

        use std::io::Cursor;
        use std::io::Read;
        let mut archive = zip::read::ZipArchive::new(Cursor::new(&bytes)).unwrap();
        let mut sheet_xml = String::new();
        archive
            .by_name("xl/worksheets/sheet1.xml")
            .unwrap()
            .read_to_string(&mut sheet_xml)
            .unwrap();

        // Normal
        assert!(sheet_xml.contains(r#"<c r="A1" s="0""#));
    }

    /// write_cells_with_styles returns Err when style_indices is exhausted early.
    #[test]
    fn test_write_cells_with_styles_exhaustion() {
        let ws = build_test_worksheet();

        let mut buf = Vec::new();
        let string_indices = HashMap::new();
        // worksheet has 4 cells but slice is length 1 → should error, not panic
        let style_indices = vec![0u32];

        let result = write_cells_with_styles(&mut buf, &ws, &string_indices, &style_indices);
        match result {
            Err(ExcelrsError::Write(msg)) => {
                assert!(
                    msg.contains("style_indices"),
                    "error should mention style_indices: {msg}"
                );
            }
            other => panic!("expected Err(Write), got {other:?}"),
        }
    }

    // -- End-to-end style round-trip (v0.3.1) --

    /// Write a styled cell with excelrs, read back with excelrs, verify the
    /// style is preserved end-to-end.  Catches the "napi setter unreachable"
    /// class of bug for non-alignment styles and any silent style loss
    /// through the write-then-parse cycle.
    #[test]
    fn test_round_trip_style_preserved() {
        use crate::reader::xlsx::workbook_inner_from_bytes;

        let mut inner = WorkbookInner::new();
        let ws = inner.add_worksheet("RoundTrip".into());
        ws.add_row(vec![serde_json::json!("hello")]);

        // Set a style with font + fill + alignment + num_fmt
        ws.set_cell_style(
            1,
            1,
            serde_json::json!({
                "font": { "bold": true, "color": "FFFF0000" },
                "fill": { "kind": "solid", "foreground": "FFFFFF00" },
                "alignment": { "horizontal": "center", "vertical": "middle" },
                "num_fmt": "0.00%",
            }),
        )
        .unwrap();

        // Write with excelrs
        let bytes = crate::writer::xlsx::workbook_to_bytes(&inner).unwrap();

        // Read back with excelrs
        let read_back = workbook_inner_from_bytes(&bytes).unwrap();
        let ws = &read_back.worksheets()[0];
        let cell = ws.get_cell_by_address("A1".into());

        let style = cell.style().expect("style should round-trip");
        assert_eq!(style.font.as_ref().unwrap().bold, Some(true));
        assert_eq!(style.font.as_ref().unwrap().color.as_deref(), Some("FFFF0000"));
        assert_eq!(style.fill.as_ref().unwrap().foreground.as_deref(), Some("FFFFFF00"));
        assert_eq!(style.alignment.as_ref().unwrap().horizontal.as_deref(), Some("center"));
        assert_eq!(style.alignment.as_ref().unwrap().vertical.as_deref(), Some("middle"));
        assert_eq!(style.num_fmt.as_deref(), Some("0.00%"));
    }

    fn build_test_worksheet() -> Worksheet {
        let mut ws = Worksheet::new("Test".into());
        ws.set_id(1);
        ws.add_row(vec![
            serde_json::json!(42),
            serde_json::json!("Hello"),
            serde_json::json!(true),
        ]);
        ws.add_row(vec![serde_json::json!(std::f64::consts::PI)]);
        ws
    }

    // ---- helpers ----

    fn build_test_workbook() -> WorkbookInner {
        let mut inner = WorkbookInner::new();
        inner.worksheets.push(build_test_worksheet());
        inner
    }

    fn workbook_inner_from_path(path: &Path) -> Result<WorkbookInner, ExcelrsError> {
        use std::io::Read;
        let mut file = std::fs::File::open(path).map_err(ExcelrsError::Io)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data).map_err(ExcelrsError::Io)?;
        workbook_inner_from_bytes(&data)
    }
}
