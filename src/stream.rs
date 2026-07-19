//! Streaming XLSX I/O — SAX-based reader/writer that process large workbooks
//! without materializing the full in-memory model.
//!
//! # Scope (v2.0.0)
//! This implements the streaming capstone: cell values + styles + core sheet
//! structure, parsed/written row-by-row. Rich parts (comments, images,
//! drawings, tables, conditional formatting, data validations, hyperlinks,
//! sheet views, protection, header/footer, page setup, defined names) are
//! intentionally out of scope for the first streaming release — matching
//! ExcelJS `stream.xlsx` limitations. The whole-workbook reader/writer remain
//! the full-fidelity route.
//!
//! # Design notes
//! - Reader opens each `xl/worksheets/sheetN.xml` as a stream and SAX-parses
//!   `<sheetData>` one row at a time; shared strings + styles are read once up
//!   front (small vs. cell data), so memory is bounded by data size, not part
//!   count. See `design.md` D3.
//! - Writer emits parts via `zip::ZipWriter`, one sheet at a time, so only a
//!   single sheet's XML is buffered at once.
//! - Style resolution reuses the existing style modules (same `Style` model as
//!   the in-memory reader/writer). See `design.md` D5.

use std::collections::HashMap;
use std::io::{Cursor, Read, Seek, Write};

use quick_xml::escape::escape;
use quick_xml::events::Event;
use quick_xml::Reader as XmlReader;

use crate::error::ExcelrsError;
use crate::model::style::Style;
use crate::reader::styles as reader_styles;
use crate::writer::styles as writer_styles;

// ---------------------------------------------------------------------------
// Streaming model (lightweight; reuses the `Style` model for fidelity)
// ---------------------------------------------------------------------------

/// A cell value as seen on the streaming path.
#[derive(Clone, Debug, PartialEq)]
pub enum StreamValue {
    /// Numeric value (numbers, dates stored as serials).
    Number(f64),
    /// Shared / inline / formula-cached string.
    Text(String),
    /// Boolean.
    Bool(bool),
    /// Formula text (cached value is not retained on the streaming path).
    Formula(String),
}

/// A single cell in a streamed row. `col` is 1-indexed (Excel convention).
#[derive(Clone, Debug)]
pub struct StreamCell {
    pub col: u32,
    pub value: StreamValue,
    pub style: Option<Style>,
}

/// A streamed worksheet row. `r` is the 1-indexed row number.
#[derive(Clone, Debug)]
pub struct StreamRow {
    pub r: u32,
    pub cells: Vec<StreamCell>,
    pub style: Option<Style>,
}

/// A streamed worksheet: name + ordered rows.
#[derive(Clone, Debug)]
pub struct StreamSheet {
    pub name: String,
    pub rows: Vec<StreamRow>,
}

/// Per-cell emission record used by the streaming writer.
struct CellEmit {
    col: u32,
    kind: u8, // 0 number, 1 text(idx), 2 bool, 3 formula
    num: f64,
    str_idx: u32,
    bool_val: bool,
    formula: String,
    style_pos: usize,
}

/// Per-row emission record used by the streaming writer.
struct RowEmit {
    r: u32,
    cells: Vec<CellEmit>,
    style_pos: usize,
}

const MAX_ENTRY_BYTES: u64 = 16 * 1024 * 1024;
const MAX_EVENTS: usize = 5_000_000;

// ---------------------------------------------------------------------------
// Streaming reader
// ---------------------------------------------------------------------------

/// Stream a workbook's sheets (rows/cells) from `.xlsx` bytes without building
/// the full in-memory model.
///
/// Shared strings + styles are read once up front (small vs. cell data); each
/// sheet is then SAX-parsed row-by-row.
pub fn stream_read(data: &[u8]) -> Result<Vec<StreamSheet>, ExcelrsError> {
    let mut archive = zip::ZipArchive::new(Cursor::new(data)).map_err(|e| ExcelrsError::Zip(e.to_string()))?;

    // Sheet order + names come from xl/workbook.xml, mapped to sheet numbers via
    // xl/_rels/workbook.xml.rels (r:id → worksheets/sheetN.xml).
    let ordered = parse_workbook_sheet_targets(data)?;
    let sheet_count = ordered.len();

    let (style_table, _maps) = reader_styles::parse_styles_and_sheet_maps(data, sheet_count)?;
    let shared = parse_shared_strings(data)?;

    let mut sheets = Vec::with_capacity(sheet_count);
    for (name, sheet_num) in ordered {
        let path = format!("xl/worksheets/sheet{}.xml", sheet_num);
        let xml = match archive.by_name(&path) {
            Ok(mut entry) => {
                if entry.size() > MAX_ENTRY_BYTES {
                    return Err(ExcelrsError::Read(format!(
                        "worksheet '{path}' exceeds streaming size limit ({MAX_ENTRY_BYTES} bytes)"
                    )));
                }
                let mut s = String::new();
                entry.read_to_string(&mut s)?;
                s
            }
            Err(_) => String::new(),
        };
        let rows = parse_sheet_rows(&xml, &style_table, &shared)?;
        sheets.push(StreamSheet { name, rows });
    }

    Ok(sheets)
}

/// Parse `xl/workbook.xml` + its rels, returning `(sheet_name, sheet_number)`
/// in document order.
fn parse_workbook_sheet_targets(data: &[u8]) -> Result<Vec<(String, u32)>, ExcelrsError> {
    let mut archive = zip::ZipArchive::new(Cursor::new(data)).map_err(|e| ExcelrsError::Zip(e.to_string()))?;

    // r:id → target (e.g. "worksheets/sheet3.xml")
    let mut rid_to_target: HashMap<String, String> = HashMap::new();
    if let Ok(mut rels) = archive.by_name("xl/_rels/workbook.xml.rels") {
        if rels.size() > MAX_ENTRY_BYTES {
            return Err(ExcelrsError::Read(format!(
                "workbook.xml.rels exceeds streaming size limit ({MAX_ENTRY_BYTES} bytes)"
            )));
        }
        let mut xml = String::new();
        rels.read_to_string(&mut xml)?;
        let mut reader = XmlReader::from_str(&xml);
        let mut buf = Vec::new();
        loop {
            buf.clear();
            match reader.read_event_into(&mut buf) {
                Ok(Event::Empty(ref e)) | Ok(Event::Start(ref e)) if e.name().as_ref() == b"Relationship" => {
                    let mut rid = None;
                    let mut target = None;
                    for attr in e.attributes().flatten() {
                        match attr.key.as_ref() {
                            b"Id" => rid = Some(String::from_utf8_lossy(&attr.value).into_owned()),
                            b"Target" => target = Some(String::from_utf8_lossy(&attr.value).into_owned()),
                            _ => {}
                        }
                    }
                    if let (Some(r), Some(t)) = (rid, target) {
                        rid_to_target.insert(r, t);
                    }
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
        }
    }

    // document-order <sheet name r:id> in xl/workbook.xml
    let mut workbook_xml = String::new();
    if let Ok(mut wb) = archive.by_name("xl/workbook.xml") {
        if wb.size() > MAX_ENTRY_BYTES {
            return Err(ExcelrsError::Read(format!(
                "workbook.xml exceeds streaming size limit ({MAX_ENTRY_BYTES} bytes)"
            )));
        }
        wb.read_to_string(&mut workbook_xml)?;
    }
    let mut reader = XmlReader::from_str(&workbook_xml);
    let mut buf = Vec::new();
    let mut in_sheets = false;
    let mut result: Vec<(String, u32)> = Vec::new();
    loop {
        buf.clear();
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"sheets" => in_sheets = true,
            Ok(Event::End(ref e)) if e.name().as_ref() == b"sheets" => in_sheets = false,
            Ok(Event::Empty(ref e)) | Ok(Event::Start(ref e)) if in_sheets && e.name().as_ref() == b"sheet" => {
                let mut name = None;
                let mut rid = None;
                for attr in e.attributes().flatten() {
                    match attr.key.as_ref() {
                        b"name" => name = Some(String::from_utf8_lossy(&attr.value).into_owned()),
                        b"r:id" => rid = Some(String::from_utf8_lossy(&attr.value).into_owned()),
                        _ => {}
                    }
                }
                if let (Some(name), Some(rid)) = (name, rid) {
                    if let Some(target) = rid_to_target.get(&rid) {
                        if let Some(num) = sheet_number_from_target(target) {
                            result.push((name, num));
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }

    if result.is_empty() {
        // Fallback: a single default sheet.
        result.push(("Sheet1".to_string(), 1));
    }
    Ok(result)
}

/// Extract the trailing sheet number from a target like `worksheets/sheet3.xml`.
fn sheet_number_from_target(target: &str) -> Option<u32> {
    let file = target.rsplit('/').next().unwrap_or(target);
    let stem: String = file.chars().filter(|c| c.is_ascii_digit()).collect();
    stem.parse().ok()
}

/// Parse `xl/sharedStrings.xml` into an index-ordered vector of strings.
fn parse_shared_strings(data: &[u8]) -> Result<Vec<String>, ExcelrsError> {
    let mut archive = zip::ZipArchive::new(Cursor::new(data)).map_err(|e| ExcelrsError::Zip(e.to_string()))?;
    let mut entry = match archive.by_name("xl/sharedStrings.xml") {
        Ok(e) => e,
        Err(_) => return Ok(Vec::new()),
    };
    if entry.size() > MAX_ENTRY_BYTES {
        return Err(ExcelrsError::Read(format!(
            "sharedStrings.xml exceeds streaming size limit ({MAX_ENTRY_BYTES} bytes)"
        )));
    }
    let mut xml = String::new();
    entry.read_to_string(&mut xml)?;

    let mut reader = XmlReader::from_str(&xml);
    let mut buf = Vec::new();
    let mut strings: Vec<String> = Vec::new();
    let mut cur: Option<String> = None;
    let mut in_t = false;
    let mut in_rph = false;
    let mut events: u64 = 0;
    loop {
        buf.clear();
        events += 1;
        if events > MAX_EVENTS as u64 {
            return Err(ExcelrsError::Read(format!(
                "sharedStrings.xml exceeds event limit ({MAX_EVENTS})"
            )));
        }
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"si" => {
                cur = Some(String::new());
            }
            Ok(Event::Empty(ref e)) if e.name().as_ref() == b"si" => {
                strings.push(String::new());
            }
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"rPh" || e.name().as_ref() == b"phoneticPr" => {
                in_rph = true;
            }
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"t" => {
                in_t = true;
            }
            Ok(Event::Text(ref e)) if in_t && !in_rph => {
                if let Some(c) = cur.as_mut() {
                    c.push_str(&e.unescape().unwrap_or_default());
                }
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == b"t" => {
                in_t = false;
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == b"rPh" || e.name().as_ref() == b"phoneticPr" => {
                in_rph = false;
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == b"si" => {
                if let Some(s) = cur.take() {
                    strings.push(s);
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }
    Ok(strings)
}

/// SAX-parse `<sheetData>` into ordered `StreamRow`s.
fn parse_sheet_rows(
    xml: &str,
    style_table: &reader_styles::StyleTableRead,
    shared: &[String],
) -> Result<Vec<StreamRow>, ExcelrsError> {
    let mut reader = XmlReader::from_str(xml);
    let mut buf = Vec::new();
    let mut rows: Vec<StreamRow> = Vec::new();

    let mut in_sheet_data = false;
    let mut in_row = false;
    let mut row_num: u32 = 0;
    let mut row_style: Option<Style> = None;
    let mut cells: Vec<StreamCell> = Vec::new();

    // per-cell parse state
    let mut in_cell = false;
    let mut cell_col: u32 = 0;
    let mut cell_style: Option<Style> = None;
    let mut cell_type: String = String::new();
    let mut cell_ref: String = String::new();
    let mut has_formula = false;
    let mut formula_buf = String::new();
    let mut value_buf = String::new();
    let mut in_inline = false;
    let mut inline_buf = String::new();
    let mut in_f = false;

    let mut events: u64 = 0;
    loop {
        buf.clear();
        events += 1;
        if events > MAX_EVENTS as u64 {
            return Err(ExcelrsError::Read(format!(
                "sheet exceeds event limit ({MAX_EVENTS})"
            )));
        }
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"sheetData" => in_sheet_data = true,
            Ok(Event::End(ref e)) if e.name().as_ref() == b"sheetData" => in_sheet_data = false,
            Ok(Event::Start(ref e)) if in_sheet_data && e.name().as_ref() == b"row" => {
                in_row = true;
                row_num = 0;
                row_style = None;
                cells = Vec::new();
                for attr in e.attributes().flatten() {
                    match attr.key.as_ref() {
                        b"r" => row_num = String::from_utf8_lossy(&attr.value).trim().parse().unwrap_or(0),
                        b"s" => {
                            let idx: u32 = String::from_utf8_lossy(&attr.value).trim().parse().unwrap_or(0);
                            row_style = style_table.resolve_style(idx);
                        }
                        _ => {}
                    }
                }
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == b"row" => {
                in_row = false;
                rows.push(StreamRow {
                    r: row_num,
                    cells: std::mem::take(&mut cells),
                    style: row_style.clone(),
                });
            }
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) if in_row && e.name().as_ref() == b"c" => {
                in_cell = true;
                cell_col = 0;
                cell_style = None;
                cell_type.clear();
                cell_ref.clear();
                has_formula = false;
                formula_buf.clear();
                value_buf.clear();
                in_inline = false;
                inline_buf.clear();
                for attr in e.attributes().flatten() {
                    match attr.key.as_ref() {
                        b"r" => cell_ref = String::from_utf8_lossy(&attr.value).into_owned(),
                        b"s" => {
                            let idx: u32 = String::from_utf8_lossy(&attr.value).trim().parse().unwrap_or(0);
                            cell_style = style_table.resolve_style(idx);
                        }
                        b"t" => cell_type = String::from_utf8_lossy(&attr.value).into_owned(),
                        _ => {}
                    }
                }
                if cell_col == 0 {
                    cell_col = col_from_ref(&cell_ref);
                }
            }
            Ok(Event::Start(ref e)) if in_cell && e.name().as_ref() == b"f" => {
                has_formula = true;
                in_f = true;
                formula_buf.clear();
            }
            Ok(Event::Empty(ref e)) if in_cell && e.name().as_ref() == b"f" => {
                has_formula = true;
                formula_buf.clear();
            }
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) if in_cell && e.name().as_ref() == b"is" => {
                in_inline = true;
            }
            Ok(Event::Text(ref e)) => {
                let t = e.unescape().unwrap_or_default();
                if in_cell && !in_inline {
                    value_buf.push_str(&t);
                }
                if in_inline {
                    inline_buf.push_str(&t);
                }
                if in_f {
                    formula_buf.push_str(&t);
                }
            }
            Ok(Event::CData(ref e)) if in_cell && !in_inline => {
                value_buf.push_str(std::str::from_utf8(e.as_ref()).unwrap_or_default());
            }
            Ok(Event::End(ref e)) if in_cell && e.name().as_ref() == b"c" => {
                in_cell = false;
                let value = build_cell_value(&cell_type, &value_buf, &inline_buf, &formula_buf, has_formula, shared);
                cells.push(StreamCell {
                    col: cell_col,
                    value,
                    style: cell_style.clone(),
                });
            }
            Ok(Event::End(ref e)) if in_cell && e.name().as_ref() == b"is" => {
                in_inline = false;
            }
            Ok(Event::End(ref e)) if in_cell && e.name().as_ref() == b"f" => {
                in_f = false;
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }

    Ok(rows)
}

/// Build a `StreamValue` from raw parsed cell fields.
fn build_cell_value(
    cell_type: &str,
    value: &str,
    inline: &str,
    formula: &str,
    has_formula: bool,
    shared: &[String],
) -> StreamValue {
    if has_formula && !formula.is_empty() {
        return StreamValue::Formula(formula.to_string());
    }
    match cell_type {
        "b" => StreamValue::Bool(value.trim() == "1"),
        "s" => {
            let idx: usize = value.trim().parse().unwrap_or(0);
            StreamValue::Text(shared.get(idx).cloned().unwrap_or_default())
        }
        "str" => StreamValue::Text(value.to_string()),
        "inlineStr" => StreamValue::Text(inline.to_string()),
        "e" => StreamValue::Text(value.to_string()),
        _ => match value.trim().parse::<f64>() {
            Ok(n) => StreamValue::Number(n),
            Err(_) => StreamValue::Text(value.to_string()),
        },
    }
}

/// Extract the 1-indexed column number from a cell reference like `AB12`.
fn col_from_ref(ref_: &str) -> u32 {
    let mut col: u32 = 0;
    for c in ref_.chars() {
        if c.is_ascii_uppercase() {
            col = col * 26 + (c as u32 - b'A' as u32 + 1);
        }
    }
    col
}

// ---------------------------------------------------------------------------
// Streaming writer
// ---------------------------------------------------------------------------

/// Serialize streamed sheets into an in-memory `.xlsx` buffer.
///
/// Sheets are written one at a time via `zip::ZipWriter`, so only a single
/// sheet's XML is buffered at once. Emitted parts mirror the whole-workbook
/// writer's minimal valid structure (content types, rels, workbook, shared
/// strings, styles, sheet parts) so the result is readable by Excel / ExcelJS
/// / the `excelrs` in-memory reader.
pub fn stream_write(sheets: &[StreamSheet]) -> Result<Vec<u8>, ExcelrsError> {
    let sheets: Vec<StreamSheet> = if sheets.is_empty() {
        vec![StreamSheet {
            name: "Sheet1".into(),
            rows: Vec::new(),
        }]
    } else {
        sheets.to_vec()
    };
    let sheet_count = sheets.len();

    let mut out = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(Cursor::new(&mut out));

        // --- Pass 1: shared strings + style collection (row-major) ---
        let mut string_table: Vec<String> = Vec::new();
        let mut string_indices: HashMap<String, u32> = HashMap::new();
        let mut cell_styles: Vec<Option<Style>> = Vec::new();
        let mut row_styles: Vec<Option<Style>> = Vec::new();
        let mut sheet_emits: Vec<Vec<RowEmit>> = Vec::with_capacity(sheet_count);

        for sh in sheets.iter() {
            let mut row_emits = Vec::with_capacity(sh.rows.len());
            for row in sh.rows.iter() {
                let mut cell_emits = Vec::with_capacity(row.cells.len());
                for cell in row.cells.iter() {
                    let style_pos = cell_styles.len();
                    cell_styles.push(cell.style.clone());
                    let emit = match &cell.value {
                        StreamValue::Number(n) => CellEmit {
                            col: cell.col,
                            kind: 0,
                            num: *n,
                            str_idx: 0,
                            bool_val: false,
                            formula: String::new(),
                            style_pos,
                        },
                        StreamValue::Text(s) => {
                            let idx = *string_indices.entry(s.clone()).or_insert_with(|| {
                                let i = string_table.len() as u32;
                                string_table.push(s.clone());
                                i
                            });
                            CellEmit {
                                col: cell.col,
                                kind: 1,
                                num: 0.0,
                                str_idx: idx,
                                bool_val: false,
                                formula: String::new(),
                                style_pos,
                            }
                        }
                        StreamValue::Bool(b) => CellEmit {
                            col: cell.col,
                            kind: 2,
                            num: 0.0,
                            str_idx: 0,
                            bool_val: *b,
                            formula: String::new(),
                            style_pos,
                        },
                        StreamValue::Formula(f) => CellEmit {
                            col: cell.col,
                            kind: 3,
                            num: 0.0,
                            str_idx: 0,
                            bool_val: false,
                            formula: f.clone(),
                            style_pos,
                        },
                    };
                    cell_emits.push(emit);
                }
                let row_style_pos = row_styles.len();
                row_styles.push(row.style.clone());
                row_emits.push(RowEmit {
                    r: row.r,
                    cells: cell_emits,
                    style_pos: row_style_pos,
                });
            }
            sheet_emits.push(row_emits);
        }

        let cell_count = cell_styles.len();
        let mut all_styles: Vec<Option<Style>> = cell_styles;
        all_styles.extend(row_styles);
        let style_table = writer_styles::build_style_table(&all_styles);

        // --- Write OOXML parts ---
        start_file(&mut zip, "[Content_Types].xml")?;
        write_content_types(&mut zip, sheet_count)?;

        start_file(&mut zip, "_rels/.rels")?;
        write_rels_rels(&mut zip)?;

        start_file(&mut zip, "xl/workbook.xml")?;
        write_workbook_xml(&mut zip, &sheets)?;

        start_file(&mut zip, "xl/_rels/workbook.xml.rels")?;
        write_workbook_rels(&mut zip, sheet_count)?;

        start_file(&mut zip, "xl/sharedStrings.xml")?;
        write_shared_strings(&mut zip, &string_table)?;

        start_file(&mut zip, "xl/styles.xml")?;
        writer_styles::emit_styles_xml(&mut zip, &style_table)?;

        for (i, row_emits) in sheet_emits.iter().enumerate() {
            let path = format!("xl/worksheets/sheet{}.xml", i + 1);
            start_file(&mut zip, &path)?;
            write_sheet_xml(&mut zip, row_emits, &style_table, cell_count)?;
        }

        zip.finish()
            .map_err(|e| ExcelrsError::Write(format!("Failed to finalise zip: {e}")))?;
    }

    Ok(out)
}

// ---------------------------------------------------------------------------
// Writer helpers
// ---------------------------------------------------------------------------

fn start_file<W: Write + Seek>(zip: &mut zip::ZipWriter<W>, name: &str) -> Result<(), ExcelrsError> {
    let options: zip::write::FileOptions<'_, ()> =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    zip.start_file(name, options)
        .map_err(|e| ExcelrsError::Write(format!("Failed to write '{name}': {e}")))
}

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
                r#"<Override PartName="/xl/worksheets/sheet{i}.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>"#
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

fn write_workbook_xml<W: Write>(w: &mut W, sheets: &[StreamSheet]) -> Result<(), ExcelrsError> {
    write_str(w, r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#)?;
    write_str(
        w,
        r#"<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">"#,
    )?;
    write_str(w, "<sheets>")?;
    for (i, sh) in sheets.iter().enumerate() {
        let name = escape(&sh.name);
        let rid = i + 3;
        write_str(w, &format!(r#"<sheet name="{name}" sheetId="{rid}" r:id="rId{rid}"/>"#))?;
    }
    write_str(w, "</sheets>")?;
    write_str(w, "</workbook>")?;
    Ok(())
}

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
                r#"<Relationship Id="rId{rid}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet{i}.xml"/>"#,
                rid = i + 2,
            ),
        )?;
    }
    write_str(w, "</Relationships>")?;
    Ok(())
}

fn write_shared_strings<W: Write>(w: &mut W, table: &[String]) -> Result<(), ExcelrsError> {
    let count = table.len();
    write_str(w, r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#)?;
    write_str(
        w,
        &format!(
            r#"<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" count="{count}" uniqueCount="{count}">"#
        ),
    )?;
    for s in table {
        if s.starts_with(' ') || s.ends_with(' ') {
            write_str(w, &format!("<si><t xml:space=\"preserve\">{}</t></si>", escape(s)))?;
        } else {
            write_str(w, &format!("<si><t>{}</t></si>", escape(s)))?;
        }
    }
    write_str(w, "</sst>")?;
    Ok(())
}

fn write_sheet_xml<W: Write>(
    w: &mut W,
    row_emits: &[RowEmit],
    style_table: &writer_styles::StyleTable,
    cell_count: usize,
) -> Result<(), ExcelrsError> {
    write_str(w, r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#)?;
    write_str(
        w,
        r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">"#,
    )?;
    write_str(w, "<sheetData>")?;
    for re in row_emits {
        let row_xf = style_table.cell_indices[cell_count + re.style_pos];
        let row_attr = if row_xf != 0 {
            format!(" r=\"{}\" s=\"{row_xf}\"", re.r)
        } else {
            format!(" r=\"{}\"", re.r)
        };
        write_str(w, &format!("<row{row_attr}>"))?;
        for ce in &re.cells {
            let xf = style_table.cell_indices[ce.style_pos];
            let cell_ref = format!("{}{}", col_to_letter(ce.col), re.r);
            let (t_attr, body) = match ce.kind {
                0 => ("".to_string(), format!("<v>{}</v>", ce.num)),
                1 => (" t=\"s\"".to_string(), format!("<v>{}</v>", ce.str_idx)),
                2 => (
                    " t=\"b\"".to_string(),
                    format!("<v>{}</v>", if ce.bool_val { 1 } else { 0 }),
                ),
                3 => (
                    " t=\"str\"".to_string(),
                    format!("<f>{}</f><v>0</v>", escape(&ce.formula)),
                ),
                _ => ("".to_string(), String::new()),
            };
            let s_attr = if xf != 0 { format!(" s=\"{xf}\"") } else { String::new() };
            write_str(w, &format!("<c r=\"{cell_ref}\"{t_attr}{s_attr}>{body}</c>"))?;
        }
        write_str(w, "</row>")?;
    }
    write_str(w, "</sheetData>")?;
    write_str(w, "</worksheet>")?;
    Ok(())
}

/// 1-indexed column number → Excel column letters.
fn col_to_letter(mut col: u32) -> String {
    let mut s = String::new();
    while col > 0 {
        let rem = (col - 1) % 26;
        s.push((b'A' + rem as u8) as char);
        col = (col - 1) / 26;
    }
    s.chars().rev().collect()
}

fn write_str<W: Write>(w: &mut W, s: &str) -> Result<(), ExcelrsError> {
    w.write_all(s.as_bytes())
        .map_err(|e| ExcelrsError::Write(format!("Write error: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn num(v: f64) -> StreamValue {
        StreamValue::Number(v)
    }
    fn txt(s: &str) -> StreamValue {
        StreamValue::Text(s.to_string())
    }

    #[test]
    fn stream_write_then_read_roundtrip() {
        let sheets = vec![StreamSheet {
            name: "Data".into(),
            rows: vec![
                StreamRow {
                    r: 1,
                    cells: vec![
                        StreamCell {
                            col: 1,
                            value: txt("name"),
                            style: None,
                        },
                        StreamCell {
                            col: 2,
                            value: txt("score"),
                            style: None,
                        },
                    ],
                    style: None,
                },
                StreamRow {
                    r: 2,
                    cells: vec![
                        StreamCell {
                            col: 1,
                            value: txt("Alice"),
                            style: None,
                        },
                        StreamCell {
                            col: 2,
                            value: num(42.0),
                            style: None,
                        },
                        StreamCell {
                            col: 3,
                            value: StreamValue::Bool(true),
                            style: None,
                        },
                    ],
                    style: None,
                },
            ],
        }];

        let bytes = stream_write(&sheets).expect("write");
        let read = stream_read(&bytes).expect("read");
        assert_eq!(read.len(), 1);
        assert_eq!(read[0].name, "Data");
        assert_eq!(read[0].rows.len(), 2);
        assert_eq!(read[0].rows[1].cells[0].value, txt("Alice"));
        assert_eq!(read[0].rows[1].cells[1].value, num(42.0));
        assert_eq!(read[0].rows[1].cells[2].value, StreamValue::Bool(true));
    }

    #[test]
    fn stream_write_is_readable_by_inmemory_reader() {
        let sheets = vec![StreamSheet {
            name: "Sheet1".into(),
            rows: vec![StreamRow {
                r: 1,
                cells: vec![
                    StreamCell {
                        col: 1,
                        value: txt("hello"),
                        style: None,
                    },
                    StreamCell {
                        col: 2,
                        value: num(3.5),
                        style: None,
                    },
                ],
                style: None,
            }],
        }];
        let bytes = stream_write(&sheets).unwrap();
        let inner = crate::reader::xlsx::workbook_inner_from_bytes(&bytes)
            .expect("in-memory reader should parse stream output");
        assert_eq!(inner.worksheet_count(), 1);
        let ws = &inner.worksheets()[0];
        let c = ws.get_cell_by_rc(1, 1).value_raw();
        assert_eq!(c.string.as_deref(), Some("hello"));
    }

    #[test]
    fn stream_read_preserves_values_from_inmemory_writer() {
        use crate::model::workbook_inner::WorkbookInner;
        use crate::model::worksheet::Worksheet;
        use crate::writer::xlsx::workbook_to_bytes;

        let mut inner = WorkbookInner::new();
        let mut ws = Worksheet::new("Src".into());
        ws.set_id(1);
        ws.insert_cell_value(1, 1, crate::model::cell::CellValue::string("greeting"));
        ws.insert_cell_value(2, 1, crate::model::cell::CellValue::number(99.0));
        inner.worksheets.push(ws);

        let bytes = workbook_to_bytes(&inner).unwrap();
        let read = stream_read(&bytes).unwrap();
        assert_eq!(read[0].name, "Src");
        assert_eq!(read[0].rows[0].cells[0].value, txt("greeting"));
        assert_eq!(read[0].rows[1].cells[0].value, num(99.0));
    }

    #[test]
    fn stream_formula_roundtrip() {
        let sheets = vec![StreamSheet {
            name: "S".into(),
            rows: vec![StreamRow {
                r: 1,
                cells: vec![StreamCell {
                    col: 4,
                    value: StreamValue::Formula("B1&C1".into()),
                    style: None,
                }],
                style: None,
            }],
        }];
        let bytes = stream_write(&sheets).expect("write");
        let read = stream_read(&bytes).expect("read");
        assert_eq!(read.len(), 1);
        assert_eq!(read[0].rows[0].cells[0].value, StreamValue::Formula("B1&C1".into()));
    }

    #[test]
    fn shared_strings_handles_empty_and_phonetic() {
        // Minimal xlsx zip containing only xl/sharedStrings.xml.
        let sst = concat!(
            "<sst xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\">",
            "<si/>",
            "<si><t>alpha</t></si>",
            "<si><t>\u{6771}\u{4eac}</t><rPh sb=\"0\" eb=\"1\"><t>\u{30c8}\u{30a6}\u{30ad}\u{30e7}\u{30a6}</t></rPh></si>",
            "</sst>",
        );
        let mut buf = Vec::new();
        {
            let mut w = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            let opts = zip::write::SimpleFileOptions::default();
            w.start_file("xl/sharedStrings.xml", opts).unwrap();
            w.write_all(sst.as_bytes()).unwrap();
            w.finish().unwrap();
        }
        let strings = parse_shared_strings(&buf).expect("parse");
        assert_eq!(strings, vec!["", "alpha", "\u{6771}\u{4eac}"]);
    }
}
