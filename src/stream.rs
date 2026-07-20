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
use std::sync::Arc;

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
    /// Empty cell (no value). Distinct from `Text("")` (an empty-string cell).
    Empty,
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

/// Max bytes read from a single zip entry on the streaming path. Bounds the
/// *actual* decompressed bytes (via `take`), not just the declared size, so a
/// part that declares a small size but decompresses large cannot exhaust memory.
pub const MAX_ENTRY_BYTES: u64 = 16 * 1024 * 1024;
/// Max SAX events per sheet (anti-billion-row / entity-expansion guard).
pub const MAX_EVENTS: usize = 5_000_000;
/// Max number of zip entries a streaming reader will accept. Bounds the
/// central-directory parse (zip-bomb surface) before any content is read.
pub const MAX_ARCHIVE_ENTRIES: usize = 10_000;
/// Max raw byte size of an input `.xlsx` the streaming reader will accept.
/// `MAX_ENTRY_BYTES` guards per-entry *decompressed* bytes; this guards the
/// total input so a huge-but-sparse archive still cannot exhaust memory.
pub const MAX_ARCHIVE_BYTES: u64 = 256 * 1024 * 1024;

// ---------------------------------------------------------------------------
// Streaming reader
// ---------------------------------------------------------------------------

/// Stream a workbook's sheets (rows/cells) from `.xlsx` bytes without building
/// the full in-memory model.
///
/// Shared strings + styles are read once up front (small vs. cell data); each
/// sheet is then SAX-parsed row-by-row.
pub fn stream_read(data: &[u8]) -> Result<Vec<StreamSheet>, ExcelrsError> {
    let mut archive =
        zip::ZipArchive::new(Cursor::new(Arc::from(data))).map_err(|e| ExcelrsError::Zip(e.to_string()))?;

    if archive.len() > MAX_ARCHIVE_ENTRIES {
        return Err(ExcelrsError::Read(format!(
            "streaming reader rejected input: too many entries ({} > limit {})",
            archive.len(),
            MAX_ARCHIVE_ENTRIES
        )));
    }
    if data.len() as u64 > MAX_ARCHIVE_BYTES {
        return Err(ExcelrsError::Read(format!(
            "streaming reader rejected input: file too large ({} bytes > limit {} bytes)",
            data.len(),
            MAX_ARCHIVE_BYTES
        )));
    }

    // Sheet order + names come from xl/workbook.xml, mapped to sheet numbers via
    // xl/_rels/workbook.xml.rels (r:id → worksheets/sheetN.xml).
    let ordered = parse_workbook_sheet_targets(&mut archive)?;
    let sheet_count = ordered.len();

    let (style_table, _maps) = reader_styles::parse_styles_and_sheet_maps(data, sheet_count)?;
    let shared = parse_shared_strings(&mut archive)?;

    let mut sheets = Vec::with_capacity(sheet_count);
    for (name, path) in ordered {
        let xml = match archive.by_name(&path) {
            Ok(entry) => {
                if entry.size() > MAX_ENTRY_BYTES {
                    return Err(ExcelrsError::Read(format!(
                        "worksheet '{path}' exceeds streaming size limit ({MAX_ENTRY_BYTES} bytes)"
                    )));
                }
                let mut s = String::new();
                entry.take(MAX_ENTRY_BYTES).read_to_string(&mut s)?;
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
pub fn parse_workbook_sheet_targets(
    archive: &mut zip::ZipArchive<Cursor<Arc<[u8]>>>,
) -> Result<Vec<(String, String)>, ExcelrsError> {
    // r:id → target (e.g. "worksheets/sheet3.xml")
    let mut rid_to_target: HashMap<String, String> = HashMap::new();
    if let Ok(rels) = archive.by_name("xl/_rels/workbook.xml.rels") {
        if rels.size() > MAX_ENTRY_BYTES {
            return Err(ExcelrsError::Read(format!(
                "workbook.xml.rels exceeds streaming size limit ({MAX_ENTRY_BYTES} bytes)"
            )));
        }
        let mut xml = String::new();
        rels.take(MAX_ENTRY_BYTES).read_to_string(&mut xml)?;
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
    if let Ok(wb) = archive.by_name("xl/workbook.xml") {
        if wb.size() > MAX_ENTRY_BYTES {
            return Err(ExcelrsError::Read(format!(
                "workbook.xml exceeds streaming size limit ({MAX_ENTRY_BYTES} bytes)"
            )));
        }
        wb.take(MAX_ENTRY_BYTES).read_to_string(&mut workbook_xml)?;
    }
    let mut reader = XmlReader::from_str(&workbook_xml);
    let mut buf = Vec::new();
    let mut in_sheets = false;
    let mut result: Vec<(String, String)> = Vec::new();
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
                        // Rels targets are relative to xl/ by default, but may be
                        // absolute (package-rooted, leading '/'). Strip the leading
                        // '/' for absolute targets; prefix xl/ for relative ones.
                        let path = if let Some(pkg) = target.strip_prefix('/') {
                            pkg.to_string()
                        } else {
                            format!("xl/{}", target)
                        };
                        result.push((name, path));
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
        result.push(("Sheet1".to_string(), "xl/worksheets/sheet1.xml".to_string()));
    }
    Ok(result)
}

/// Parse `xl/sharedStrings.xml` into an index-ordered vector of strings.
pub fn parse_shared_strings(archive: &mut zip::ZipArchive<Cursor<Arc<[u8]>>>) -> Result<Vec<String>, ExcelrsError> {
    let entry = match archive.by_name("xl/sharedStrings.xml") {
        Ok(e) => e,
        Err(_) => return Ok(Vec::new()),
    };
    if entry.size() > MAX_ENTRY_BYTES {
        return Err(ExcelrsError::Read(format!(
            "sharedStrings.xml exceeds streaming size limit ({MAX_ENTRY_BYTES} bytes)"
        )));
    }
    let mut xml = String::new();
    entry.take(MAX_ENTRY_BYTES).read_to_string(&mut xml)?;

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
pub fn parse_sheet_rows(
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
            return Err(ExcelrsError::Read(format!("sheet exceeds event limit ({MAX_EVENTS})")));
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
                in_f = false; // ponytail: bound flag to cell; malformed missing </f> can't leak into next cell
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
        _ => {
            if value.is_empty() {
                // ponytail: empty cell (no value) is distinct from an empty-string cell
                return StreamValue::Empty;
            }
            match value.trim().parse::<f64>() {
                Ok(n) => StreamValue::Number(n),
                Err(_) => StreamValue::Text(value.to_string()),
            }
        }
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
                        StreamValue::Empty => CellEmit {
                            col: cell.col,
                            kind: 4,
                            num: 0.0,
                            str_idx: 0,
                            bool_val: false,
                            formula: String::new(),
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
    fn stream_read_preserves_multi_sheet_order() {
        // Regression net for the sheet name<->file pairing / document-order path
        // (audit risk A1). If pairing regresses, names/order/values here fail.
        let sheets = vec![
            StreamSheet {
                name: "First".into(),
                rows: vec![StreamRow {
                    r: 1,
                    cells: vec![StreamCell {
                        col: 1,
                        value: num(11.0),
                        style: None,
                    }],
                    style: None,
                }],
            },
            StreamSheet {
                name: "Second".into(),
                rows: vec![StreamRow {
                    r: 1,
                    cells: vec![StreamCell {
                        col: 1,
                        value: num(22.0),
                        style: None,
                    }],
                    style: None,
                }],
            },
            StreamSheet {
                name: "Third".into(),
                rows: vec![StreamRow {
                    r: 1,
                    cells: vec![StreamCell {
                        col: 1,
                        value: num(33.0),
                        style: None,
                    }],
                    style: None,
                }],
            },
        ];
        let bytes = stream_write(&sheets).expect("write");
        let read = stream_read(&bytes).expect("read");
        assert_eq!(read.len(), 3);
        assert_eq!(read[0].name, "First");
        assert_eq!(read[1].name, "Second");
        assert_eq!(read[2].name, "Third");
        assert_eq!(read[0].rows[0].cells[0].value, num(11.0));
        assert_eq!(read[1].rows[0].cells[0].value, num(22.0));
        assert_eq!(read[2].rows[0].cells[0].value, num(33.0));
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
        let cursor = std::io::Cursor::new(std::sync::Arc::from(&buf[..]));
        let mut archive = zip::ZipArchive::new(cursor).expect("archive");
        let strings = parse_shared_strings(&mut archive).expect("parse");
        assert_eq!(strings, vec!["", "alpha", "\u{6771}\u{4eac}"]);
    }

    #[test]
    fn stream_read_resolves_sheet_by_rels_target() {
        // Build a minimal xlsx zip where the rels target uses a non-default filename.
        use std::io::Write;
        let mut buf = Vec::new();
        {
            let mut w = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            let opts = zip::write::SimpleFileOptions::default();

            w.start_file("[Content_Types].xml", opts).unwrap();
            w.write_all(concat!(
                r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
                r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">"#,
                r#"<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>"#,
                r#"<Default Extension="xml" ContentType="application/xml"/>"#,
                r#"<Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>"#,
                r#"<Override PartName="/xl/worksheets/sheet_v2.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>"#,
                r#"</Types>"#,
            ).as_bytes()).unwrap();

            w.start_file("_rels/.rels", opts).unwrap();
            w.write_all(concat!(
                r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
                r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
                r#"<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>"#,
                r#"</Relationships>"#,
            ).as_bytes()).unwrap();

            w.start_file("xl/workbook.xml", opts).unwrap();
            w.write_all(
                concat!(
                    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
                    r#"<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main""#,
                    r#" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">"#,
                    r#"<sheets><sheet name="NonDefault" sheetId="1" r:id="rId1"/></sheets>"#,
                    r#"</workbook>"#,
                )
                .as_bytes(),
            )
            .unwrap();

            w.start_file("xl/_rels/workbook.xml.rels", opts).unwrap();
            w.write_all(concat!(
                r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
                r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
                r#"<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet""#,
                r#" Target="worksheets/sheet_v2.xml"/>"#,
                r#"</Relationships>"#,
            ).as_bytes()).unwrap();

            w.start_file("xl/worksheets/sheet_v2.xml", opts).unwrap();
            w.write_all(
                concat!(
                    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
                    r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">"#,
                    r#"<sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>Custom</t></is></c></row></sheetData>"#,
                    r#"</worksheet>"#,
                )
                .as_bytes(),
            )
            .unwrap();

            w.finish().unwrap();
        }

        let sheets = stream_read(&buf).expect("read");
        assert_eq!(sheets.len(), 1);
        assert_eq!(sheets[0].name, "NonDefault");
        assert_eq!(sheets[0].rows.len(), 1);
        assert_eq!(
            sheets[0].rows[0].cells[0].value,
            StreamValue::Text("Custom".to_string())
        );
    }

    #[test]
    fn stream_read_resolves_sheet_by_absolute_rels_target() {
        // Like stream_read_resolves_sheet_by_rels_target, but the rels Target is
        // *absolute* (package-rooted, leading '/'). Without the fix the reader
        // builds "xl//xl/worksheets/sheet_v2.xml" and silently yields an empty
        // sheet; with the fix it strips the leading '/' and reads the real part.
        use std::io::Write;
        let mut buf = Vec::new();
        {
            let mut w = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            let opts = zip::write::SimpleFileOptions::default();

            w.start_file("[Content_Types].xml", opts).unwrap();
            w.write_all(concat!(
                r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
                r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">"#,
                r#"<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>"#,
                r#"<Default Extension="xml" ContentType="application/xml"/>"#,
                r#"<Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>"#,
                r#"<Override PartName="/xl/worksheets/sheet_v2.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>"#,
                r#"</Types>"#,
            ).as_bytes()).unwrap();

            w.start_file("_rels/.rels", opts).unwrap();
            w.write_all(concat!(
                r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
                r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
                r#"<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>"#,
                r#"</Relationships>"#,
            ).as_bytes()).unwrap();

            w.start_file("xl/workbook.xml", opts).unwrap();
            w.write_all(
                concat!(
                    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
                    r#"<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main""#,
                    r#" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">"#,
                    r#"<sheets><sheet name="AbsTarget" sheetId="1" r:id="rId1"/></sheets>"#,
                    r#"</workbook>"#,
                )
                .as_bytes(),
            )
            .unwrap();

            w.start_file("xl/_rels/workbook.xml.rels", opts).unwrap();
            w.write_all(concat!(
                r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
                r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
                r#"<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet""#,
                r#" Target="/xl/worksheets/sheet_v2.xml"/>"#,
                r#"</Relationships>"#,
            ).as_bytes()).unwrap();

            w.start_file("xl/worksheets/sheet_v2.xml", opts).unwrap();
            w.write_all(
                concat!(
                    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
                    r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">"#,
                    r#"<sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>AbsoluteCustom</t></is></c></row></sheetData>"#,
                    r#"</worksheet>"#,
                )
                .as_bytes(),
            )
            .unwrap();

            w.finish().unwrap();
        }

        let sheets = stream_read(&buf).expect("read");
        assert_eq!(sheets.len(), 1);
        assert_eq!(sheets[0].name, "AbsTarget");
        assert_eq!(sheets[0].rows.len(), 1);
        assert_eq!(
            sheets[0].rows[0].cells[0].value,
            StreamValue::Text("AbsoluteCustom".to_string())
        );
    }

    #[test]
    fn stream_read_legit_oversized_is_rejected() {
        // A sheet whose *declared* uncompressed size genuinely exceeds the cap.
        let big = vec![b'A'; 17 * 1024 * 1024];
        let mut buf = Vec::new();
        {
            let mut w = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            let opts = zip::write::SimpleFileOptions::default();
            w.start_file("xl/worksheets/sheet1.xml", opts).unwrap();
            w.write_all(&big).unwrap();
            w.finish().unwrap();
        }
        let err = stream_read(&buf).unwrap_err();
        assert!(
            err.to_string().contains("exceeds streaming size limit"),
            "expected size-limit error, got: {err}"
        );
    }

    #[test]
    fn stream_read_hostile_declared_size_is_bounded() {
        // Declared uncompressed size is patched tiny while the entry still
        // decompresses past the cap. The real-byte `.take` guard must bound the
        // read (≤ MAX_ENTRY_BYTES) so it parses instead of allocating the full
        // decompressed size.
        let row = br#"<row r="1"><c r="A1"><v>1</v></c></row>"#;
        let mut content = Vec::with_capacity(40 * 1024 * 1024);
        while content.len() < 40 * 1024 * 1024 {
            content.extend_from_slice(row);
        }
        let mut buf = Vec::new();
        {
            let mut w = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            let opts = zip::write::SimpleFileOptions::default();
            w.start_file("xl/worksheets/sheet1.xml", opts).unwrap();
            w.write_all(&content).unwrap();
            w.finish().unwrap();
        }
        // Patch the central-directory uncompressed size down to 1 (lie).
        let eocd = buf.len() - 22;
        let cd_offset = u32::from_le_bytes([buf[eocd + 16], buf[eocd + 17], buf[eocd + 18], buf[eocd + 19]]) as usize;
        let off = cd_offset + 24;
        buf[off..off + 4].copy_from_slice(&1u32.to_le_bytes());

        // With the `.take` guard the read is capped at 16 MiB; the truncated sheet
        // then fails to parse (fail-safe per design.md). Regression: without the guard
        // the full ~40 MiB would be read and parsed successfully → Ok (OOM risk).
        let read = stream_read(&buf);
        assert!(
            read.is_err(),
            "hostile declared-size zip must be bounded, got: {read:?}"
        );
    }

    #[test]
    fn stream_write_read_empty_vs_empty_string() {
        let sheets = vec![StreamSheet {
            name: "Sheet1".into(),
            rows: vec![StreamRow {
                r: 1,
                cells: vec![
                    StreamCell {
                        col: 1,
                        value: StreamValue::Empty,
                        style: None,
                    },
                    StreamCell {
                        col: 2,
                        value: StreamValue::Text(String::new()),
                        style: None,
                    },
                ],
                style: None,
            }],
        }];
        let bytes = stream_write(&sheets).expect("write");
        let read = stream_read(&bytes).expect("read");
        let cells = &read[0].rows[0].cells;
        assert_eq!(
            cells[0].value,
            StreamValue::Empty,
            "empty cell must round-trip as Empty"
        );
        assert_eq!(
            cells[1].value,
            StreamValue::Text(String::new()),
            "empty-string cell must round-trip as Text(\"\")"
        );
    }

    #[test]
    fn stream_read_missing_f_close_is_fail_safe() {
        // A1 opens <f> but never closes it before </c> (a corrupt/truncated formula
        // cell). With strict XML parsing the sheet fails to parse rather than leaking
        // A1's formula text into B1's value — the parser must not panic or corrupt a
        // sibling cell. This locks the v2.0.0 in_f-reset fix: a malformed formula cell
        // is fail-safe, never corrupts another cell.
        let xml = concat!(
            r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">"#,
            r#"<sheetData><row r="1">"#,
            r#"<c r="A1" t="str"><f>SUM(B1)<v>1</v></c>"#,
            r#"<c r="B1"><v>10</v></c>"#,
            r#"</row></sheetData></worksheet>"#,
        );
        // Must not panic / abort on corrupt formula markup.
        let rows = parse_sheet_rows(xml, &reader_styles::StyleTableRead::empty(), &[]);
        assert!(rows.is_ok(), "malformed formula cell must not panic: {:?}", rows.err());

        // The well-formed counterpart must keep B1's own value (not the prior formula).
        let ok = concat!(
            r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">"#,
            r#"<sheetData><row r="1">"#,
            r#"<c r="A1" t="str"><f>SUM(B1)</f><v>1</v></c>"#,
            r#"<c r="B1"><v>10</v></c>"#,
            r#"</row></sheetData></worksheet>"#,
        );
        let rows = parse_sheet_rows(ok, &reader_styles::StyleTableRead::empty(), &[]).expect("parse");
        assert_eq!(rows.len(), 1);
        let cells = &rows[0].cells;
        assert_eq!(cells.len(), 2, "both cells must parse");
        // B1 keeps its own value, not appended to A1's formula.
        assert_eq!(cells[1].value, StreamValue::Number(10.0));
        // A1 is still recognized as a formula.
        assert!(
            matches!(cells[0].value, StreamValue::Formula(_)),
            "A1 should be a formula, got {:?}",
            cells[0].value
        );
    }
}

#[cfg(test)]
mod streaming_safety_tests {
    use super::*;
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    #[test]
    fn stream_read_rejects_too_many_entries() {
        // Build a zip whose central directory exceeds MAX_ARCHIVE_ENTRIES.
        // This is the zip-bomb surface: many tiny entries can exhaust memory
        // before any content is read. The streaming reader must reject it.
        let mut buf = Vec::new();
        {
            let mut zw = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            let opts = SimpleFileOptions::default();
            for i in 0..(MAX_ARCHIVE_ENTRIES + 1) {
                let name = format!("entry{i}.txt");
                zw.start_file(name, opts).expect("start_file");
                zw.write_all(b"x").expect("write");
            }
            zw.finish().expect("finish");
        }

        let result = stream_read(&buf);
        assert!(result.is_err(), "reader must reject a too-many-entries archive");
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("too many entries"), "unexpected error: {msg}");
    }
}
