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
//! - `xl/styles.xml` (Normal-only minimal)
//!
//! # v0.1 limitations (per spec)
//! - No column width/properties preserved
//! - No merged cells
//! - No custom styles beyond Normal
//! - Formula cells write the formula string but no cached value

use std::collections::HashMap;
use std::io::{Seek, Write};
use std::path::Path;

use quick_xml::escape::escape;

use crate::error::ExcelrsError;
use crate::model::workbook_inner::WorkbookInner;
use crate::model::worksheet::Worksheet;

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

        // xl/styles.xml (Normal-only)
        start_file(&mut zip, "xl/styles.xml")?;
        write_styles_xml(&mut zip)?;

        // xl/worksheets/sheet{N}.xml
        for (i, ws) in worksheets.iter().enumerate() {
            let sheet_path = format!("xl/worksheets/sheet{}.xml", i + 1);
            start_file(&mut zip, &sheet_path)?;
            write_sheet_xml(&mut zip, ws, &string_indices)?;
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
// xl/styles.xml (Normal-only minimal)
// ---------------------------------------------------------------------------

fn write_styles_xml<W: Write>(w: &mut W) -> Result<(), ExcelrsError> {
    write_str(w, r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#)?;
    write_str(
        w,
        r#"<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">"#,
    )?;
    // Fonts: one default font
    write_str(
        w,
        r#"<fonts count="1"><font><sz val="11"/><name val="Calibri"/></font></fonts>"#,
    )?;
    // Fills: none (minimum 2 required, but we'll provide default)
    write_str(
        w,
        r#"<fills count="2"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill></fills>"#,
    )?;
    // Borders: one default border
    write_str(
        w,
        r#"<borders count="1"><border><left/><right/><top/><bottom/><diagonal/></border></borders>"#,
    )?;
    // Cell style formats: one default
    write_str(
        w,
        r#"<cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs>"#,
    )?;
    // Cell formats: one default
    write_str(
        w,
        r#"<cellXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/></cellXfs>"#,
    )?;
    write_str(w, "</styleSheet>")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// xl/worksheets/sheet{N}.xml
// ---------------------------------------------------------------------------

fn write_sheet_xml<W: Write>(
    w: &mut W,
    ws: &Worksheet,
    string_indices: &HashMap<String, u32>,
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

    for row in ws.rows() {
        write!(w, r#"<row r="{}">"#, row.number())?;
        for cell in row.sorted_cells() {
            write_cell_xml(w, cell, string_indices)?;
        }
        write_str(w, "</row>")?;
    }

    write_str(w, "</sheetData>")?;
    write_str(w, "</worksheet>")?;
    Ok(())
}

/// Write a single `<c>` element.
fn write_cell_xml<W: Write>(
    w: &mut W,
    cell: &crate::model::cell::Cell,
    string_indices: &HashMap<String, u32>,
) -> Result<(), ExcelrsError> {
    let cv = cell.value();
    let address = cell.address();
    let formula = cell.formula();

    // Open the cell element
    write!(w, r#"<c r="{}""#, address)?;

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

    // ---- helpers ----

    fn build_test_workbook() -> WorkbookInner {
        let mut ws = Worksheet::new("Test".into());
        ws.set_id(1);
        ws.add_row(vec![
            serde_json::json!(42),
            serde_json::json!("Hello"),
            serde_json::json!(true),
        ]);
        ws.add_row(vec![serde_json::json!(std::f64::consts::PI)]);

        let mut inner = WorkbookInner::new();
        inner.worksheets.push(ws);
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
