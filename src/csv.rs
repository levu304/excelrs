//! CSV read/write: RFC 4180 parser + serializer + `WorkbookCsv` async handle.
//!
//! # Design decisions
//! - Manual RFC 4180 state machine, no new dependency (`# ponytail: manual;
//!   swap to the `csv` crate only if quoting edge cases multiply.`)
//! - Numeric inference on read (f64-parsable fields → Number cells)
//! - Single-sheet only on write (`worksheets[0]`)

use std::sync::{Arc, Mutex};

use napi::bindgen_prelude::*;
use napi_derive::napi;

use crate::model::cell::CellValue;
use crate::model::workbook_inner::WorkbookInner;

// ---------------------------------------------------------------------------
// Public API: WorkbookCsv – async handle mirroring WorkbookXlsx
// ---------------------------------------------------------------------------

/// Async CSV read/write handle.
///
/// Obtained via `Workbook.csv` getter.  Shares the same underlying
/// `Arc<Mutex<WorkbookInner>>` as the parent Workbook.
#[napi]
#[derive(Clone, Debug)]
pub struct WorkbookCsv {
    inner: Arc<Mutex<WorkbookInner>>,
}

impl WorkbookCsv {
    pub(crate) fn new(inner: Arc<Mutex<WorkbookInner>>) -> Self {
        WorkbookCsv { inner }
    }
}

#[napi]
impl WorkbookCsv {
    /// Parse a CSV `Buffer` into a single worksheet ("Sheet1"), replacing
    /// the workbook's existing worksheets in place.
    ///
    /// An optional `delimiter` overrides the field separator (default `,`).
    #[napi]
    pub async fn read(&self, buffer: Buffer, delimiter: Option<String>) -> Result<()> {
        let data = buffer.to_vec();
        let sep = resolve_delimiter(delimiter);
        let new_inner = parse_csv(&data, sep)?;
        *self.inner.lock().expect("WorkbookCsv lock poisoned") = new_inner;
        Ok(())
    }

    /// Read a CSV file from disk into a single worksheet ("Sheet1").
    ///
    /// An optional `delimiter` overrides the field separator (default `,`).
    #[napi]
    pub async fn read_file(&self, path: String, delimiter: Option<String>) -> Result<()> {
        let data = std::fs::read(&path)
            .map_err(|e| napi::Error::from_reason(format!("cannot read CSV file '{path}': {e}")))?;
        let sep = resolve_delimiter(delimiter);
        let new_inner = parse_csv(&data, sep)?;
        *self.inner.lock().expect("WorkbookCsv lock poisoned") = new_inner;
        Ok(())
    }

    /// Serialize the first worksheet to a CSV `Buffer`.
    ///
    /// Optional `delimiter` (default `,`) and `withBom` (default `false`).
    /// Only `worksheets[0]` is written (CSV is single-sheet).
    #[napi]
    pub async fn write(&self, delimiter: Option<String>, with_bom: Option<bool>) -> Result<Buffer> {
        let inner = self.inner.lock().expect("WorkbookCsv lock poisoned").clone();
        let sep = resolve_delimiter(delimiter);
        let bom = with_bom.unwrap_or(false);
        let bytes = serialize_csv(&inner, sep, bom)?;
        Ok(Buffer::from(bytes))
    }

    /// Serialize the first worksheet to a CSV file on disk.
    ///
    /// Optional `delimiter` (default `,`) and `withBom` (default `false`).
    #[napi]
    pub async fn write_file(&self, path: String, delimiter: Option<String>, with_bom: Option<bool>) -> Result<()> {
        let inner = self.inner.lock().expect("WorkbookCsv lock poisoned").clone();
        let sep = resolve_delimiter(delimiter);
        let bom = with_bom.unwrap_or(false);
        let bytes = serialize_csv(&inner, sep, bom)?;
        std::fs::write(&path, &bytes)
            .map_err(|e| napi::Error::from_reason(format!("cannot write CSV file '{path}': {e}")))?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Parser
// ---------------------------------------------------------------------------

/// Parse RFC 4180 CSV bytes into a `WorkbookInner` with a single worksheet
/// named "Sheet1".  Strips an optional UTF-8 BOM.
///
/// Fields that parse as a finite `f64` become `Number` cells; all other
/// non-empty fields become `String` cells.
pub fn parse_csv(data: &[u8], delimiter: u8) -> Result<WorkbookInner> {
    let s = std::str::from_utf8(data).map_err(|e| napi::Error::from_reason(format!("CSV is not valid UTF-8: {e}")))?;
    let s = s.trim_start_matches('\u{FEFF}'); // strip BOM

    let d = delimiter as char;
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut current_row: Vec<String> = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = s.chars().peekable();

    macro_rules! emit_field {
        () => {{
            current_row.push(std::mem::take(&mut field));
        }};
    }

    macro_rules! emit_row {
        () => {{
            rows.push(std::mem::take(&mut current_row));
        }};
    }

    while let Some(ch) = chars.next() {
        match ch {
            // Opening quote at start of an empty field → enter quoted mode
            '"' if !quoted && field.is_empty() => {
                quoted = true;
            }
            // Inside quotes: "" → escaped quote; " → end quoted mode
            '"' if quoted => match chars.peek() {
                Some('"') => {
                    field.push('"');
                    chars.next();
                }
                _ => {
                    quoted = false;
                }
            },
            // Delimiter outside quotes → end of field
            _ if ch == d && !quoted => {
                emit_field!();
            }
            // Newline outside quotes → end of field + end of row
            '\n' if !quoted => {
                emit_field!();
                emit_row!();
            }
            // Carriage return outside quotes → ignore (paired \n handles boundary)
            '\r' if !quoted => {}
            // Anything else → accumulate in current field
            _ => {
                field.push(ch);
            }
        }
    }

    // Flush last field + row if there's pending content (no trailing newline)
    if !current_row.is_empty() || !field.is_empty() {
        emit_field!();
        emit_row!();
    }

    // Build workbook
    let mut inner = WorkbookInner::new();
    if rows.is_empty() {
        return Ok(inner); // empty workbook (0 sheets) for empty input
    }

    let ws = inner.add_worksheet("Sheet1".into());
    for (ri, row_data) in rows.iter().enumerate() {
        for (ci, raw) in row_data.iter().enumerate() {
            let value = match raw.parse::<f64>() {
                Ok(n) if n.is_finite() => CellValue::number(n),
                _ => CellValue::string(raw.as_str()),
            };
            ws.insert_cell_value((ri + 1) as u32, (ci + 1) as u32, value);
        }
    }

    Ok(inner)
}

// ---------------------------------------------------------------------------
// Serializer
// ---------------------------------------------------------------------------

/// Serialize the first worksheet to RFC 4180 CSV bytes.
///
/// Returns an empty buffer when the workbook has no worksheets.  Only
/// `worksheets[0]` is written.
pub fn serialize_csv(inner: &WorkbookInner, delimiter: u8, with_bom: bool) -> Result<Vec<u8>> {
    let ws = match inner.worksheets.first() {
        Some(ws) => ws,
        None => return Ok(Vec::new()),
    };

    let all_rows = ws.rows(); // sorted by row number
    if all_rows.is_empty() {
        return Ok(Vec::new());
    }

    let d = delimiter as char;
    let mut output = String::new();
    let mut prev_row_num = 0u32;

    for row in &all_rows {
        let rn = row.number();
        // Emit blank rows for gaps between previous row and this one
        for _ in 0..(rn - prev_row_num - 1) {
            output.push('\n');
        }
        prev_row_num = rn;

        let max_col = row.max_col();
        if max_col == 0 {
            output.push('\n');
            continue;
        }

        // Build column-indexed map of cell text (1-based column index)
        let col_count = max_col as usize;
        let mut col_texts: Vec<Option<String>> = vec![None; col_count];

        for cell in row.sorted_cells() {
            let idx = (cell.col() - 1) as usize;
            if idx < col_count {
                col_texts[idx] = Some(cell_value_to_text(&cell.value_raw()));
            }
        }

        // Emit row
        for (i, maybe_text) in col_texts.iter().enumerate() {
            if i > 0 {
                output.push(d);
            }
            if let Some(text) = maybe_text {
                if !text.is_empty() {
                    output.push_str(&quote_field(text, delimiter));
                }
            }
        }
        output.push('\n');
    }

    let mut bytes = output.into_bytes();
    if with_bom {
        // UTF-8 BOM: 0xEF 0xBB 0xBF
        bytes.splice(0..0, [0xEFu8, 0xBB, 0xBF]);
    }
    Ok(bytes)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Convert a `CellValue` to its CSV text representation.
fn cell_value_to_text(value: &CellValue) -> String {
    match value.value_type.as_str() {
        "Null" => String::new(),
        "Number" => value.number.map(|n| n.to_string()).unwrap_or_default(),
        "String" => value.string.clone().unwrap_or_default(),
        "Boolean" => value
            .boolean
            .map(|b| if b { "TRUE" } else { "FALSE" }.to_string())
            .unwrap_or_default(),
        "Formula" => {
            // Prefer cached numeric/boolean value; fallback to formula string
            if let Some(n) = value.number {
                n.to_string()
            } else if let Some(b) = value.boolean {
                (if b { "TRUE" } else { "FALSE" }).to_string()
            } else {
                value.formula.clone().unwrap_or_default()
            }
        }
        "Error" => value.error_value.clone().unwrap_or_default(),
        "Hyperlink" => value
            .hyperlink_text
            .clone()
            .or_else(|| value.hyperlink.clone())
            .unwrap_or_default(),
        "RichText" => value
            .rich_text
            .as_ref()
            .map(|runs| runs.iter().map(|r| r.text.clone()).collect::<String>())
            .unwrap_or_default(),
        "Merge" => String::new(),
        _ => String::new(),
    }
}

/// RFC 4180 field quoting: wrap in `"…"` when the field contains the
/// delimiter, a newline, a carriage return, or a double-quote.
/// Embedded double-quotes are escaped as `""`.
fn quote_field(field: &str, delimiter: u8) -> String {
    let d = delimiter as char;
    if field.contains(d) || field.contains('\n') || field.contains('\r') || field.contains('"') || field.is_empty() {
        let escaped = field.replace('"', "\"\"");
        format!("\"{escaped}\"")
    } else {
        field.to_string()
    }
}

/// Resolve the delimiter from an optional JS argument.
/// Accepts any single-char string; defaults to `,`.
fn resolve_delimiter(delimiter: Option<String>) -> u8 {
    delimiter
        .as_deref()
        .and_then(|s| s.as_bytes().first().copied())
        .unwrap_or(b',')
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::workbook_inner::WorkbookInner;

    // ---- Parser tests ----

    #[test]
    fn test_parse_simple_csv() {
        let data = b"1,hello\n2,world";
        let inner = parse_csv(data, b',').unwrap();
        assert_eq!(inner.worksheet_count(), 1);
        assert_eq!(inner.worksheets()[0].name(), "Sheet1");

        let ws = &inner.worksheets[0];
        assert_eq!(ws.row_count(), 2);

        // Row 1: Number(1) in A, String("hello") in B
        let cell_a1 = ws.get_cell_by_address("A1".into());
        assert_eq!(cell_a1.value_raw().number, Some(1.0));
        let cell_b1 = ws.get_cell_by_address("B1".into());
        assert_eq!(cell_b1.value_raw().string, Some("hello".into()));

        // Row 2: Number(2) in A, String("world") in B
        let cell_a2 = ws.get_cell_by_address("A2".into());
        assert_eq!(cell_a2.value_raw().number, Some(2.0));
        let cell_b2 = ws.get_cell_by_address("B2".into());
        assert_eq!(cell_b2.value_raw().string, Some("world".into()));
    }

    #[test]
    fn test_parse_quoted_fields() {
        let data = b"\"a,b\",\"line1\nline2\"";
        let inner = parse_csv(data, b',').unwrap();
        let ws = &inner.worksheets[0];
        let cell_a1 = ws.get_cell_by_address("A1".into());
        assert_eq!(cell_a1.value_raw().string, Some("a,b".into()));
        assert_eq!(cell_a1.value_raw().value_type, "String");
        let cell_b1 = ws.get_cell_by_address("B1".into());
        assert_eq!(cell_b1.value_raw().string, Some("line1\nline2".into()));
    }

    #[test]
    fn test_parse_escaped_quotes() {
        let data = b"\"say \"\"hello\"\"\",next";
        let inner = parse_csv(data, b',').unwrap();
        let ws = &inner.worksheets[0];
        let cell_a1 = ws.get_cell_by_address("A1".into());
        assert_eq!(cell_a1.value_raw().string, Some("say \"hello\"".into()));
        let cell_b1 = ws.get_cell_by_address("B1".into());
        assert_eq!(cell_b1.value_raw().string, Some("next".into()));
    }

    #[test]
    fn test_parse_custom_delimiter() {
        let data = b"x;y\n1;2";
        let inner = parse_csv(data, b';').unwrap();
        let ws = &inner.worksheets[0];
        let cell_a1 = ws.get_cell_by_address("A1".into());
        assert_eq!(cell_a1.value_raw().string, Some("x".into()));
        let cell_b2 = ws.get_cell_by_address("B2".into());
        assert_eq!(cell_b2.value_raw().number, Some(2.0));
    }

    #[test]
    fn test_parse_bom_stripped() {
        // BOM followed by 2 data rows: "val",42 — both are parsed as data
        let data = b"\xef\xbb\xbfval\n42";
        let inner = parse_csv(data, b',').unwrap();
        let ws = &inner.worksheets[0];
        assert_eq!(ws.row_count(), 2);
        let cell_a1 = ws.get_cell_by_address("A1".into());
        assert_eq!(cell_a1.value_raw().string, Some("val".into()));
        let cell_a2 = ws.get_cell_by_address("A2".into());
        assert_eq!(cell_a2.value_raw().number, Some(42.0));
    }

    #[test]
    fn test_parse_empty_input() {
        let inner = parse_csv(b"", b',').unwrap();
        assert_eq!(inner.worksheet_count(), 0);
    }

    #[test]
    fn test_parse_trailing_newline() {
        let data = b"1,2\n3,4\n";
        let inner = parse_csv(data, b',').unwrap();
        let ws = &inner.worksheets[0];
        assert_eq!(ws.row_count(), 2);
    }

    #[test]
    fn test_parse_blank_fields() {
        let data = b",b\n1,";
        let inner = parse_csv(data, b',').unwrap();
        let ws = &inner.worksheets[0];
        assert_eq!(ws.row_count(), 2);
        // Row1: A empty, B = "b"
        let cell_b1 = ws.get_cell_by_address("B1".into());
        assert_eq!(cell_b1.value_raw().string, Some("b".into()));
        // Row2: A = Number(1)
        let cell_a2 = ws.get_cell_by_address("A2".into());
        assert_eq!(cell_a2.value_raw().number, Some(1.0));
    }

    // ---- Serializer tests ----

    #[test]
    fn test_serialize_numbers_and_strings() {
        let mut inner = WorkbookInner::new();
        let ws = inner.add_worksheet("Sheet1".into());
        ws.add_row(vec![serde_json::json!(1), serde_json::json!("hi")]);
        ws.add_row(vec![serde_json::json!(3.14), serde_json::json!("world")]);

        let bytes = serialize_csv(&inner, b',', false).unwrap();
        let result = String::from_utf8(bytes).unwrap();
        assert_eq!(result, "1,hi\n3.14,world\n");
    }

    #[test]
    fn test_serialize_quoting() {
        let mut inner = WorkbookInner::new();
        let ws = inner.add_worksheet("Sheet1".into());
        ws.add_row(vec![serde_json::json!("a,b"), serde_json::json!("line1\nline2")]);

        let bytes = serialize_csv(&inner, b',', false).unwrap();
        let result = String::from_utf8(bytes).unwrap();
        assert_eq!(result, "\"a,b\",\"line1\nline2\"\n");
    }

    #[test]
    fn test_serialize_empty_workbook() {
        let inner = WorkbookInner::new();
        let bytes = serialize_csv(&inner, b',', false).unwrap();
        assert!(bytes.is_empty());
    }

    #[test]
    fn test_serialize_empty_worksheet() {
        let mut inner = WorkbookInner::new();
        inner.add_worksheet("Sheet1".into());
        let bytes = serialize_csv(&inner, b',', false).unwrap();
        assert!(bytes.is_empty());
    }

    #[test]
    fn test_serialize_with_bom() {
        let mut inner = WorkbookInner::new();
        let ws = inner.add_worksheet("Sheet1".into());
        ws.add_row(vec![serde_json::json!(42)]);

        let bytes = serialize_csv(&inner, b',', true).unwrap();
        assert_eq!(bytes[0], 0xEF);
        assert_eq!(bytes[1], 0xBB);
        assert_eq!(bytes[2], 0xBF);
        let body = String::from_utf8(bytes[3..].to_vec()).unwrap();
        assert_eq!(body, "42\n");
    }

    #[test]
    fn test_serialize_custom_delimiter() {
        let mut inner = WorkbookInner::new();
        let ws = inner.add_worksheet("Sheet1".into());
        ws.add_row(vec![serde_json::json!(1), serde_json::json!(2)]);

        let bytes = serialize_csv(&inner, b';', false).unwrap();
        let result = String::from_utf8(bytes).unwrap();
        assert_eq!(result, "1;2\n");
    }

    #[test]
    fn test_serialize_round_trip() {
        let mut inner = WorkbookInner::new();
        let ws = inner.add_worksheet("Sheet1".into());
        ws.add_row(vec![serde_json::json!(42), serde_json::json!("hello")]);
        ws.add_row(vec![serde_json::json!(3.14), serde_json::json!("world")]);

        let bytes = serialize_csv(&inner, b',', false).unwrap();
        let parsed = parse_csv(&bytes, b',').unwrap();

        assert_eq!(parsed.worksheet_count(), 1);
        let pws = &parsed.worksheets[0];
        assert_eq!(pws.row_count(), 2);
        let cell_a1 = pws.get_cell_by_address("A1".into());
        assert_eq!(cell_a1.value_raw().number, Some(42.0));
        let cell_b1 = pws.get_cell_by_address("B1".into());
        assert_eq!(cell_b1.value_raw().string, Some("hello".into()));
        let cell_a2 = pws.get_cell_by_address("A2".into());
        assert![(cell_a2.value_raw().number.unwrap() - 3.14).abs() < 1e-10];
        let cell_b2 = pws.get_cell_by_address("B2".into());
        assert_eq!(cell_b2.value_raw().string, Some("world".into()));
    }

    // ---- Helper tests ----

    #[test]
    fn test_resolve_delimiter_default() {
        assert_eq!(resolve_delimiter(None), b',');
    }

    #[test]
    fn test_resolve_delimiter_custom() {
        assert_eq!(resolve_delimiter(Some(";".into())), b';');
        assert_eq!(resolve_delimiter(Some("\t".into())), b'\t');
    }

    #[test]
    fn test_cell_value_to_text() {
        use crate::model::cell::RichTextRun;

        assert_eq!(cell_value_to_text(&CellValue::number(42.0)), "42");
        assert_eq!(cell_value_to_text(&CellValue::string("hello")), "hello");
        assert_eq!(cell_value_to_text(&CellValue::boolean(true)), "TRUE");
        assert_eq!(cell_value_to_text(&CellValue::boolean(false)), "FALSE");
        let mut formula_val = CellValue::formula("=1+1");
        formula_val.number = Some(2.0);
        assert_eq!(cell_value_to_text(&formula_val), "2");
        assert_eq!(cell_value_to_text(&CellValue::formula("=A1+B1")), "=A1+B1");

        // Hyperlink: prefers hyperlink_text
        let hl = CellValue::hyperlink("https://x.com", Some("Click".into()));
        assert_eq!(cell_value_to_text(&hl), "Click");

        // RichText
        let rt = CellValue::rich_text(vec![
            RichTextRun {
                text: "Hello ".into(),
                font: None,
            },
            RichTextRun {
                text: "World".into(),
                font: None,
            },
        ]);
        assert_eq!(cell_value_to_text(&rt), "Hello World");
    }

    #[test]
    fn test_quote_field() {
        assert_eq!(quote_field("hello", b','), "hello");
        assert_eq!(quote_field("he,llo", b','), "\"he,llo\"");
        assert_eq!(quote_field("he\nllo", b','), "\"he\nllo\"");
        assert_eq!(quote_field("he\"llo", b','), "\"he\"\"llo\"");
        assert_eq!(quote_field("", b','), "\"\"");
    }

    // ---- Handle tests ----

    #[test]
    fn test_workbook_csv_new_shares_arc() {
        let inner = Arc::new(Mutex::new(WorkbookInner::new()));
        let csv = WorkbookCsv::new(Arc::clone(&inner));
        // Read a CSV through the handle
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(csv.read(Buffer::from(&b"a,b\n1,hello"[..]), None)).unwrap();

        let g = inner.lock().unwrap();
        assert_eq!(g.worksheet_count(), 1);
        assert_eq!(g.worksheets()[0].name(), "Sheet1");
    }

    #[test]
    fn test_workbook_csv_write() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let inner = Arc::new(Mutex::new(WorkbookInner::new()));
        {
            let ws = inner.lock().unwrap().add_worksheet("Sheet1".into());
            ws.add_row(vec![serde_json::json!(42), serde_json::json!("hi")]);
        }
        let csv = WorkbookCsv::new(Arc::clone(&inner));

        let buf = rt.block_on(csv.write(None, None)).unwrap();
        assert!(!buf.is_empty());
        let result = String::from_utf8(buf.to_vec()).unwrap();
        assert_eq!(result, "42,hi\n");
    }

    #[test]
    fn test_workbook_csv_write_file() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let inner = Arc::new(Mutex::new(WorkbookInner::new()));
        {
            let ws = inner.lock().unwrap().add_worksheet("Sheet1".into());
            ws.add_row(vec![serde_json::json!(99)]);
        }
        let csv = WorkbookCsv::new(inner);

        let tmp = std::env::temp_dir().join(format!(
            "excelrs_csv_write_{}.csv",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        let tmp_str = tmp.to_string_lossy().to_string();

        rt.block_on(csv.write_file(tmp_str, None, None)).unwrap();
        assert!(tmp.exists());

        // Verify content
        let content = std::fs::read_to_string(&tmp).unwrap();
        assert_eq!(content, "99\n");
        let _ = std::fs::remove_file(&tmp);
    }

    // ---- Round-trip regression tests (F1 & F2) ----

    #[test]
    fn test_round_trip_trailing_empty_column() {
        let mut inner = WorkbookInner::new();
        let ws = inner.add_worksheet("Sheet1".into());
        ws.add_row(vec![
            serde_json::json!(1),
            serde_json::json!(2),
            serde_json::Value::Null,
        ]);

        let bytes = serialize_csv(&inner, b',', false).unwrap();
        let result = String::from_utf8(bytes.clone()).unwrap();
        assert_eq!(result, "1,2,\n");

        // Parse back
        let parsed = parse_csv(&bytes, b',').unwrap();
        let pws = &parsed.worksheets[0];
        assert_eq!(pws.row_count(), 1);
        assert_eq!(pws.column_count(), 3);

        // Re-serialize
        let bytes2 = serialize_csv(&parsed, b',', false).unwrap();
        let result2 = String::from_utf8(bytes2).unwrap();
        assert_eq!(result2, "1,2,\n");
    }

    #[test]
    fn test_round_trip_sparse_rows() {
        let mut inner = WorkbookInner::new();
        let ws = inner.add_worksheet("Sheet1".into());
        ws.insert_cell_value(1, 1, CellValue::string("r1"));
        ws.insert_cell_value(5, 1, CellValue::string("r5"));

        let bytes = serialize_csv(&inner, b',', false).unwrap();
        let result = String::from_utf8(bytes.clone()).unwrap();
        assert_eq!(result, "r1\n\n\n\nr5\n");

        // Parse back
        let parsed = parse_csv(&bytes, b',').unwrap();
        let pws = &parsed.worksheets[0];
        assert_eq!(pws.row_count(), 5);

        let cell_a5 = pws.get_cell_by_address("A5".into());
        assert_eq!(cell_a5.value_raw().string, Some("r5".into()));
        assert_eq!(cell_a5.value_raw().value_type, "String");
    }

    #[test]
    fn test_quoted_trailing_empty() {
        let p = parse_csv(br#""a",b,"#, b',').unwrap();
        let ws = &p.worksheets[0];
        assert_eq!(ws.row_count(), 1);
        assert_eq!(ws.column_count(), 3);
        assert_eq!(
            ws.get_cell_by_address("A1".into()).value_raw().string.as_deref(),
            Some("a")
        );
        assert_eq!(
            ws.get_cell_by_address("B1".into()).value_raw().string.as_deref(),
            Some("b")
        );
    }

    #[test]
    fn test_multi_trailing_empty() {
        assert_eq!(parse_csv(b"a,b,,\n", b',').unwrap().worksheets[0].column_count(), 4);
        assert_eq!(parse_csv(b"a,,,\n", b',').unwrap().worksheets[0].column_count(), 4);
    }

    #[test]
    fn test_empty_quote_no_loss() {
        let e = parse_csv(b"1,,\"\"", b',').unwrap();
        assert_eq!(e.worksheets[0].column_count(), 3);
        let e2 = parse_csv(&serialize_csv(&e, b',', false).unwrap(), b',').unwrap();
        assert_eq!(e2.worksheets[0].column_count(), 3);
    }

    #[test]
    fn test_sparse_middle_end() {
        let mut inner = WorkbookInner::new();
        let ws = inner.add_worksheet("Sheet1".into());
        ws.insert_cell_value(2, 1, CellValue::string("r2"));
        ws.insert_cell_value(7, 1, CellValue::string("r7"));
        let out = String::from_utf8(serialize_csv(&inner, b',', false).unwrap()).unwrap();
        assert_eq!(out, "\nr2\n\n\n\n\nr7\n");
        let p = parse_csv(out.as_bytes(), b',').unwrap();
        assert_eq!(p.worksheets[0].row_count(), 7);
        assert_eq!(
            p.worksheets[0]
                .get_cell_by_address("A7".into())
                .value_raw()
                .string
                .as_deref(),
            Some("r7")
        );
    }

    #[test]
    fn test_crlf_sparse_rows() {
        // CRLF endings, blank lines between data must preserve positions
        let p = parse_csv(b"r1\r\n\r\n\r\n\r\nr5\r\n", b',').unwrap();
        let ws = &p.worksheets[0];
        assert_eq!(ws.row_count(), 5);
        assert_eq!(
            ws.get_cell_by_address("A5".into()).value_raw().string.as_deref(),
            Some("r5")
        );
    }

    #[test]
    fn test_custom_delimiter_trailing() {
        let p = parse_csv(b"a;b;", b';').unwrap();
        assert_eq!(p.worksheets[0].column_count(), 3);
    }

    #[test]
    fn test_round_trip_idempotent() {
        let mut inner = WorkbookInner::new();
        let ws = inner.add_worksheet("Sheet1".into());
        ws.insert_cell_value(1, 1, CellValue::string("r1"));
        ws.insert_cell_value(5, 1, CellValue::string("r5"));
        let s1 = serialize_csv(&inner, b',', false).unwrap();
        let s2 = serialize_csv(&parse_csv(&s1, b',').unwrap(), b',', false).unwrap();
        let s3 = serialize_csv(&parse_csv(&s2, b',').unwrap(), b',', false).unwrap();
        assert_eq!(s1, s2);
        assert_eq!(s2, s3);
    }
}
