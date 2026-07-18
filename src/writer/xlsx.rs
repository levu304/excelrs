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

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::io::{Seek, Write};
use std::path::Path;

use quick_xml::escape::escape;

use crate::error::ExcelrsError;
use crate::model::cell::Cell;
use crate::model::comment::CellComment;
use crate::model::defined_name::DefinedName;
use crate::model::image::WorksheetImage;
use crate::model::style::{Dxf, Style};
use crate::model::table::Table;
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
        // (v1.0.0) pre-pass: gather comments / images / media extensions so
        // content types and per-sheet parts can be emitted consistently.
        let mut media_exts: BTreeSet<String> = BTreeSet::new();
        let mut sheet_comments: Vec<Vec<(String, CellComment)>> = Vec::with_capacity(sheet_count);
        let mut sheet_images: Vec<Vec<WorksheetImage>> = Vec::with_capacity(sheet_count);
        let mut media_counter: u32 = 0;
        let mut sheet_tables: Vec<Vec<Table>> = Vec::with_capacity(sheet_count);
        for ws in worksheets.iter() {
            let comments = ws.get_cell_comments();
            let mut imgs = ws.get_images_inner();
            for img in imgs.iter_mut() {
                media_counter += 1;
                img.media_index = media_counter;
                media_exts.insert(img.extension.to_lowercase());
            }
            sheet_comments.push(comments);
            sheet_images.push(imgs);
            sheet_tables.push(ws.get_tables_inner());
        }

        // (v1.1.0) Table names and displayNames must be unique across the whole
        // workbook — OOXML requires per-workbook uniqueness (Excel enforces it).
        // A table's own name and displayName may legitimately be equal (the default),
        // so de-duplicate identifiers within each table before checking globally.
        {
            let mut seen_names: HashSet<String> = HashSet::new();
            for tbl in sheet_tables.iter().flatten() {
                let mut ids: Vec<String> = Vec::with_capacity(2);
                ids.push(tbl.name.clone());
                if tbl.display_name != tbl.name {
                    ids.push(tbl.display_name.clone());
                }
                for id in ids {
                    if !seen_names.insert(id.clone()) {
                        return Err(ExcelrsError::Write(format!(
                            "Duplicate table name or displayName '{}': table names and displayNames must be unique across the workbook",
                            id
                        )));
                    }
                }
            }
        }

        start_file(&mut zip, "[Content_Types].xml")?;
        write_content_types(
            &mut zip,
            sheet_count,
            &media_exts,
            &sheet_images,
            &sheet_comments,
            &sheet_tables,
        )?;

        // _rels/.rels
        start_file(&mut zip, "_rels/.rels")?;
        write_rels_rels(&mut zip)?;

        // xl/workbook.xml
        start_file(&mut zip, "xl/workbook.xml")?;
        write_workbook_xml(&mut zip, &worksheets, inner)?;

        // xl/_rels/workbook.xml.rels
        start_file(&mut zip, "xl/_rels/workbook.xml.rels")?;
        write_workbook_rels(&mut zip, sheet_count)?;

        // xl/sharedStrings.xml
        start_file(&mut zip, "xl/sharedStrings.xml")?;
        write_shared_strings(&mut zip, &string_table)?;

        // xl/styles.xml (v0.2.0: full dedup'd style table)
        start_file(&mut zip, "xl/styles.xml")?;
        // Collect effective styles across every worksheet.
        // Precedence: cell-level wins, then column-level, then Normal (None).
        // Row-level styles are collected separately after cell styles.
        let mut cell_styles: Vec<Option<Style>> = Vec::new();
        let mut row_styles: Vec<Option<Style>> = Vec::new();
        // Per-worksheet boundary tracking: (cell_count, row_count)
        let mut ws_boundaries: Vec<(usize, usize)> = Vec::new();
        for ws in worksheets.iter() {
            let col_style_map: BTreeMap<u32, Option<Style>> =
                ws.columns().iter().map(|c| (c.col_num(), c.style())).collect();
            // Cell styles (existing logic)
            let mut cell_count = 0usize;
            for row in ws.rows() {
                let written = row.written_cells();
                for cell in written {
                    let mut style = effective_cell_style_with_fallback(cell, &col_style_map);
                    // v0.13.0: Date cells need a date number format to round-trip as
                    // dates; inject a default one unless the style already has a format.
                    let cv = cell.value_raw();
                    if cv.value_type == "Date" {
                        let needs_fmt = style.as_ref().is_none_or(|s| s.num_fmt.is_none());
                        if needs_fmt {
                            let serial = cv.date_serial.unwrap_or(0.0);
                            let mut s = style.unwrap_or_default();
                            s.num_fmt = Some(crate::model::cell::date_format_for_serial(serial));
                            style = Some(s);
                        }
                    }
                    cell_styles.push(style);
                    cell_count += 1;
                }
            }
            // Row styles — include all rows (None maps to Normal index 0)
            let mut row_count = 0usize;
            for row in ws.rows() {
                row_styles.push(row.style().clone());
                row_count += 1;
            }
            ws_boundaries.push((cell_count, row_count));
        }
        let all_styles: Vec<Option<Style>> = {
            let mut v = cell_styles.clone();
            v.extend(row_styles);
            v
        };
        // (v1.2.0) Collect conditional-format dxfs across all worksheets.
        // Foreign dxfs from the source file (inner.dxfs) are preserved as the
        // base; rule styles add new dxfs (deduped by canonical key). dxfIds and
        // a document-order unique priority are assigned in-place on the rules.
        let mut dxfs: Vec<Dxf> = inner.dxfs.clone();
        let mut dxf_map: HashMap<String, u32> = HashMap::new();
        for (i, d) in dxfs.iter().enumerate() {
            if let Ok(key) = serde_json::to_string(d) {
                dxf_map.insert(key, i as u32);
            }
        }
        for ws in worksheets.iter() {
            ws.assign_conditional_formatting_dxf_ids(&mut dxfs, &mut dxf_map);
        }

        let mut style_table = styles::build_style_table(&all_styles);
        style_table.dxfs = dxfs;
        styles::emit_styles_xml(&mut zip, &style_table)?;

        // xl/worksheets/sheet{N}.xml
        let mut cell_offset = 0usize;
        let mut row_offset = 0usize;
        let mut table_part_counter: u32 = 0;
        let cell_styles_total = cell_styles.len();
        for (i, ws) in worksheets.iter().enumerate() {
            let sheet_path = format!("xl/worksheets/sheet{}.xml", i + 1);
            start_file(&mut zip, &sheet_path)?;

            let (cell_count, row_count) = ws_boundaries[i];
            let ws_cell_indices = &style_table.cell_indices[cell_offset..cell_offset + cell_count];
            cell_offset += cell_count;
            let ws_row_indices_base = &style_table.cell_indices[cell_styles_total..];
            let ws_row_indices = &ws_row_indices_base[row_offset..row_offset + row_count];
            row_offset += row_count;

            // Collect hyperlinks and data validations for this sheet
            let hyperlinks = collect_sheet_hyperlinks(ws);
            let data_validations = ws.get_data_validations();

            // (v1.0.0) comments / drawing flags for this sheet
            let has_comments = !sheet_comments[i].is_empty();
            let has_drawing = !sheet_images[i].is_empty();
            // Relationship ids: comments = hl+1, vmlDrawing (legacyDrawing) =
            // hl+2, drawing = hl+3 (or hl+1 when there are no comments).
            let hl = hyperlinks.len() as u32;
            let comment_rid: Option<u32> = if has_comments { Some(hl + 2) } else { None };
            let drawing_rid: Option<u32> = if has_drawing {
                Some(if has_comments { hl + 3 } else { hl + 1 })
            } else {
                None
            };

            // (v1.1.0) table flags + relationship ids for this sheet
            let has_tables = !sheet_tables[i].is_empty();
            let mut table_base = hl;
            if has_comments {
                table_base += 2;
            }
            if has_drawing {
                table_base += 1;
            }
            let mut table_part = table_part_counter;
            let mut table_rids: Vec<u32> = Vec::new();
            let mut table_rels: Vec<(u32, u32)> = Vec::new();
            if has_tables {
                for (k, _t) in sheet_tables[i].iter().enumerate() {
                    table_part += 1;
                    let rid = table_base + (k as u32) + 1;
                    table_rids.push(rid);
                    table_rels.push((rid, table_part));
                }
            }
            table_part_counter = table_part;

            write_sheet_xml(
                &mut zip,
                ws,
                &string_indices,
                ws_cell_indices,
                ws_row_indices,
                &hyperlinks,
                &data_validations,
                drawing_rid,
                comment_rid,
                &table_rids,
            )?;

            // (v1.0.0) comments part
            if has_comments {
                let cpath = format!("xl/comments{}.xml", i + 1);
                start_file(&mut zip, &cpath)?;
                write_comments_xml(&mut zip, &sheet_comments[i])?;
            }
            // (v1.0.0) vmlDrawing part: anchors comments to cells via legacyDrawing
            if has_comments {
                let vpath = format!("xl/drawings/vmlDrawing{}.vml", i + 1);
                start_file(&mut zip, &vpath)?;
                write_vml_drawing_xml(&mut zip, &sheet_comments[i])?;
            }

            // (v1.0.0) drawing part + media files
            if has_drawing {
                for img in sheet_images[i].iter() {
                    let ext = img.extension.to_lowercase();
                    let mpath = format!("xl/media/image{}.{}", img.media_index, ext);
                    start_file(&mut zip, &mpath)?;
                    zip.write_all(&img.buffer)
                        .map_err(|e| ExcelrsError::Write(format!("Failed to write media: {e}")))?;
                }
                let dpath = format!("xl/drawings/drawing{}.xml", i + 1);
                start_file(&mut zip, &dpath)?;
                write_drawing_xml(&mut zip, &sheet_images[i])?;
                let drel = format!("xl/drawings/_rels/drawing{}.xml.rels", i + 1);
                start_file(&mut zip, &drel)?;
                write_drawing_rels(&mut zip, &sheet_images[i])?;
            }

            // (v1.1.0) table parts
            if has_tables {
                for (k, t) in sheet_tables[i].iter().enumerate() {
                    let part = table_rels[k].1;
                    let tpath = format!("xl/tables/table{}.xml", part);
                    start_file(&mut zip, &tpath)?;
                    write_tables_xml(&mut zip, t, part)?;
                }
            }

            // Sheet relationships (hyperlinks + comments + drawing + tables)
            if !hyperlinks.is_empty() || has_comments || has_drawing || has_tables {
                let rel_path = format!("xl/worksheets/_rels/sheet{}.xml.rels", i + 1);
                start_file(&mut zip, &rel_path)?;
                write_sheet_rels(&mut zip, &hyperlinks, i + 1, has_comments, has_drawing, &table_rels)?;
            }
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
            for cell in row.written_cells() {
                let cv = cell.value_raw();
                match cv.value_type.as_str() {
                    "String" => {
                        if let Some(s) = cv.string {
                            string_indices.entry(s.clone()).or_insert_with(|| {
                                let idx = string_table.len() as u32;
                                string_table.push(s);
                                idx
                            });
                        }
                    }
                    "Hyperlink" => {
                        // Collect display text (prefer hyperlink_text, fallback to URL)
                        let text = cv.hyperlink_text.as_deref().or(cv.hyperlink.as_deref());
                        if let Some(s) = text {
                            string_indices.entry(s.to_string()).or_insert_with(|| {
                                let idx = string_table.len() as u32;
                                string_table.push(s.to_string());
                                idx
                            });
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    (string_table, string_indices)
}

// ---------------------------------------------------------------------------
// Sheet hyperlink collection
// ---------------------------------------------------------------------------

/// A hyperlink reference on a single worksheet.
struct SheetHyperlink {
    cell_ref: String,
    rid: String,
    url: String,
}

/// Walk all cells in a worksheet and collect hyperlink references.
fn collect_sheet_hyperlinks(ws: &Worksheet) -> Vec<SheetHyperlink> {
    let mut out = Vec::new();
    for row in ws.rows() {
        for cell in row.written_cells() {
            let cv = cell.value_raw();
            if cv.value_type == "Hyperlink" {
                if let Some(ref url) = cv.hyperlink {
                    let ref_addr = cell.address();
                    let rid = format!("rId{}", out.len() + 1);
                    out.push(SheetHyperlink {
                        cell_ref: ref_addr,
                        rid,
                        url: url.clone(),
                    });
                }
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// (v1.0.0) comments / drawings helpers
// ---------------------------------------------------------------------------

/// Map a media file extension to its IANA/MIME content type.
fn media_content_type(ext: &str) -> &'static str {
    match ext {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "tif" | "tiff" => "image/tiff",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        _ => "application/octet-stream",
    }
}

/// Write `xl/commentsN.xml` from a list of `(cellRef, CellComment)` pairs.
fn write_comments_xml<W: Write>(w: &mut W, comments: &[(String, CellComment)]) -> Result<(), ExcelrsError> {
    write_str(w, r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#)?;
    write_str(
        w,
        r#"<comments xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">"#,
    )?;
    let mut authors: Vec<String> = Vec::new();
    for (_, c) in comments {
        if let Some(a) = &c.author {
            if !authors.contains(a) {
                authors.push(a.clone());
            }
        }
    }
    if authors.is_empty() {
        authors.push(String::new());
    }
    write_str(w, "<authors>")?;
    for a in &authors {
        write_str(w, &format!("<author>{}</author>", escape(a)))?;
    }
    write_str(w, "</authors>")?;
    write_str(w, "<commentList>")?;
    for (ref_addr, c) in comments {
        let author_id = match &c.author {
            Some(a) => authors.iter().position(|x| x == a).unwrap_or(0) as u32,
            None => 0,
        };
        write_str(
            w,
            &format!(r#"<comment ref="{}" authorId="{}">"#, escape(ref_addr), author_id),
        )?;
        write_str(w, "<text>")?;
        write_str(w, &format!("<t>{}</t>", escape(&c.text)))?;
        write_str(w, "</text>")?;
        write_str(w, "</comment>")?;
    }
    write_str(w, "</commentList>")?;
    write_str(w, "</comments>")?;
    Ok(())
}

/// Write `xl/drawings/vmlDrawingN.vml`, anchoring each comment `<v:shape>` to its
/// cell. Mirrors the comment vmlDrawing part Excel / ExcelJS emit (F2, v1.0.0).
fn write_vml_drawing_xml<W: Write>(w: &mut W, comments: &[(String, CellComment)]) -> Result<(), ExcelrsError> {
    write_str(w, "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>")?;
    write_str(
        w,
        "<xml xmlns:v=\"urn:schemas-microsoft-com:vml\" xmlns:o=\"urn:schemas-microsoft-com:office:office\" xmlns:x=\"urn:schemas-microsoft-com:office:excel\">",
    )?;
    for (i, (ref_addr, _c)) in comments.iter().enumerate() {
        let (row0, col0) = cell_ref_to_zero(ref_addr);
        let id = 1024 + i as u32;
        write_str(
            w,
            &format!(
                "<v:shape id=\"_x0000_s{id}\" type=\"#_x0000_t202\" style=\"position:absolute;visibility:hidden;width:120pt;height:60pt\" fillcolor=\"#ffffe1\" strokecolor=\"#000000\" o:insetmode=\"auto\">"
            ),
        )?;
        write_str(w, "<v:fill color2=\"#ffffe1\"/>")?;
        write_str(w, "<v:shadow on=\"t\" obscured=\"t\"/>")?;
        write_str(w, "<v:path o:connecttype=\"none\"/>")?;
        write_str(w, "<v:textbox style=\"mso-direction-alt:auto\"/>")?;
        write_str(w, "<x:ClientData ObjectType=\"Note\">")?;
        write_str(w, "<x:MoveWithCells/>")?;
        write_str(w, "<x:SizeWithCells/>")?;
        write_str(w, "<x:Anchor>1, 15, 0, 2, 3, 15, 2, 4</x:Anchor>")?;
        write_str(w, "<x:AutoFill>False</x:AutoFill>")?;
        write_str(w, &format!("<x:Row>{row0}</x:Row>"))?;
        write_str(w, &format!("<x:Column>{col0}</x:Column>"))?;
        write_str(w, "<x:Visible/>")?;
        write_str(w, &format!("<x:CommentRow>{row0}</x:CommentRow>"))?;
        write_str(w, &format!("<x:CommentColumn>{col0}</x:CommentColumn>"))?;
        write_str(w, "</x:ClientData>")?;
        write_str(w, "</v:shape>")?;
    }
    write_str(w, "</xml>")?;
    Ok(())
}

fn cell_ref_to_zero(ref_: &str) -> (u32, u32) {
    let ref_ = ref_.trim();
    let mut col: u32 = 0;
    let mut row_digits = String::new();
    for ch in ref_.chars() {
        if ch.is_ascii_alphabetic() {
            col = col * 26 + (ch.to_ascii_uppercase() as u32 - b'A' as u32 + 1);
        } else {
            row_digits.push(ch);
        }
    }
    let row: u32 = row_digits.parse().unwrap_or(1);
    (row.saturating_sub(1), col.saturating_sub(1))
}

/// Write `xl/drawings/drawingN.xml` from a sheet's images.
fn write_drawing_xml<W: Write>(w: &mut W, images: &[WorksheetImage]) -> Result<(), ExcelrsError> {
    write_str(w, r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#)?;
    write_str(
        w,
        r#"<xdr:wsDr xmlns:xdr="http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">"#,
    )?;
    for (idx, img) in images.iter().enumerate() {
        let rid = idx + 1;
        let a = &img.anchor;
        let from = format!(
            "<xdr:from><xdr:col>{}</xdr:col><xdr:colOff>{}</xdr:colOff><xdr:row>{}</xdr:row><xdr:rowOff>{}</xdr:rowOff></xdr:from>",
            a.col, a.x, a.row, a.y
        );
        let (open, close) = if a.anchor_type == "twoCell" {
            ("<xdr:twoCellAnchor>", "</xdr:twoCellAnchor>")
        } else {
            ("<xdr:oneCellAnchor>", "</xdr:oneCellAnchor>")
        };
        let mut body = String::new();
        body.push_str(open);
        body.push_str(&from);
        if a.anchor_type == "twoCell" {
            body.push_str(&format!(
                "<xdr:to><xdr:col>{}</xdr:col><xdr:colOff>{}</xdr:colOff><xdr:row>{}</xdr:row><xdr:rowOff>{}</xdr:rowOff></xdr:to>",
                a.col2, a.x2, a.row2, a.y2
            ));
        }
        body.push_str(&format!(
            "<xdr:pic><xdr:nvPicPr><xdr:cNvPr id=\"{rid}\" name=\"Picture {rid}\"/><xdr:cNvPicPr/></xdr:nvPicPr><xdr:blipFill><a:blip r:embed=\"rId{rid}\"/><a:stretch><a:fillRect/></a:stretch></xdr:blipFill><xdr:spPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"0\" cy=\"0\"/></a:xfrm><a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></xdr:spPr></xdr:pic>"
        ));
        body.push_str("<xdr:clientData/>");
        body.push_str(close);
        write_str(w, &body)?;
    }
    write_str(w, "</xdr:wsDr>")?;
    Ok(())
}

/// Write `xl/tables/tableN.xml` for a single table (v1.1.0).
fn write_tables_xml<W: Write>(w: &mut W, table: &Table, table_num: u32) -> Result<(), ExcelrsError> {
    write_str(w, r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#)?;
    let open = format!(
        r#"<table xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" id="{table_num}" name="{}" displayName="{}" ref="{}" totalsRowShown="{}" headerRowCount="{}">"#,
        escape(&table.name),
        escape(&table.display_name),
        escape(&table.ref_range),
        if table.totals_row { 1 } else { 0 },
        if table.header_row { 1 } else { 0 },
    );
    write_str(w, &open)?;
    if let Some(af) = &table.autofilter_ref {
        write_str(w, &format!(r#"<autoFilter ref="{}"/>"#, escape(af)))?;
    }
    let ncols = table.columns.len();
    write_str(w, &format!(r#"<tableColumns count="{ncols}">"#))?;
    for (i, col) in table.columns.iter().enumerate() {
        let id = i + 1;
        let mut attrs = format!(r#"<tableColumn id="{id}" name="{}""#, escape(&col.name));
        if let Some(f) = &col.totals_row_function {
            attrs.push_str(&format!(r#" totalsRowFunction="{}""#, escape(f)));
        }
        if let Some(l) = &col.totals_row_label {
            attrs.push_str(&format!(r#" totalsRowLabel="{}""#, escape(l)));
        }
        attrs.push_str("/>");
        write_str(w, &attrs)?;
    }
    write_str(w, "</tableColumns>")?;
    if let Some(style) = &table.style {
        let theme = style.theme.clone().unwrap_or_else(|| "TableStyleMedium2".to_string());
        write_str(
            w,
            &format!(
                r#"<tableStyleInfo name="{}" showFirstColumn="{}" showLastColumn="{}" showRowStripes="{}" showColumnStripes="{}"/>"#,
                escape(&theme),
                if style.show_first_column.unwrap_or(false) { 1 } else { 0 },
                if style.show_last_column.unwrap_or(false) { 1 } else { 0 },
                if style.show_row_stripes.unwrap_or(false) { 1 } else { 0 },
                if style.show_column_stripes.unwrap_or(false) {
                    1
                } else {
                    0
                },
            ),
        )?;
    }
    write_str(w, "</table>")?;
    Ok(())
}

/// Write `xl/drawings/_rels/drawingN.xml.rels` mapping each image rId to media.
fn write_drawing_rels<W: Write>(w: &mut W, images: &[WorksheetImage]) -> Result<(), ExcelrsError> {
    write_str(w, r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#)?;
    write_str(
        w,
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
    )?;
    for (idx, img) in images.iter().enumerate() {
        let rid = idx + 1;
        let ext = img.extension.to_lowercase();
        write_str(
            w,
            &format!(
                r#"<Relationship Id="rId{rid}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/image{}.{}"/>"#,
                img.media_index, ext
            ),
        )?;
    }
    write_str(w, "</Relationships>")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// [Content_Types].xml
// ---------------------------------------------------------------------------

fn write_content_types<W: Write>(
    w: &mut W,
    sheet_count: usize,
    media_exts: &BTreeSet<String>,
    sheet_images: &[Vec<WorksheetImage>],
    sheet_comments: &[Vec<(String, CellComment)>],
    _sheet_tables: &[Vec<Table>],
) -> Result<(), ExcelrsError> {
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
    // (v1.0.0) media + drawings + comments content types
    for ext in media_exts {
        write_str(
            w,
            &format!(
                r#"<Default Extension="{ext}" ContentType="{}"/>"#,
                media_content_type(ext)
            ),
        )?;
    }
    for (i, imgs) in sheet_images.iter().enumerate() {
        if !imgs.is_empty() {
            let n = i + 1;
            write_str(
                w,
                &format!(
                    r#"<Override PartName="/xl/drawings/drawing{n}.xml" ContentType="application/vnd.openxmlformats-officedocument.drawing+xml"/>"#,
                ),
            )?;
        }
    }
    for (i, comments) in sheet_comments.iter().enumerate() {
        if !comments.is_empty() {
            let n = i + 1;
            write_str(
                w,
                &format!(
                    r#"<Override PartName="/xl/comments{n}.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.comments+xml"/>"#,
                ),
            )?;
        }
    }
    // (v1.0.0) vmlDrawing content type for comment anchors (F2)
    if sheet_comments.iter().any(|c| !c.is_empty()) {
        write_str(
            w,
            r##"<Default Extension="vml" ContentType="application/vnd.openxmlformats-officedocument.vmlDrawing"/>"##,
        )?;
    }
    // (v1.1.0) table part content types (global numbering matches the writer loop)
    let mut table_part = 0u32;
    for sheet_tbls in _sheet_tables.iter() {
        for _ in sheet_tbls.iter() {
            table_part += 1;
            write_str(
                w,
                &format!(
                    r#"<Override PartName="/xl/tables/table{table_part}.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.table+xml"/>"#,
                ),
            )?;
        }
    }
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

fn write_workbook_xml<W: Write>(
    w: &mut W,
    worksheets: &[Worksheet],
    inner: &WorkbookInner,
) -> Result<(), ExcelrsError> {
    write_str(w, r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#)?;
    write_str(
        w,
        r#"<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">"#,
    )?;
    // <bookViews> — (v1.0.0)
    emit_book_views(w, inner)?;
    write_str(w, "<sheets>")?;
    for (i, ws) in worksheets.iter().enumerate() {
        let name = ws.name();
        let name_esc = escape(&name);
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

    // <calcPr> — (v1.0.0)
    emit_calc_pr(w, inner)?;

    // Combine explicit defined names with per-sheet print area / print titles
    // derived from each worksheet's page setup (v1.0.0).
    let mut all_dns: Vec<DefinedName> = inner.defined_names().to_vec();
    for ws in worksheets.iter() {
        if let Some(ps) = ws.get_page_setup_inner() {
            if let Some(pa) = &ps.print_area {
                let dn = DefinedName::sheet_scoped("_xlnm.Print_Area", format!("{}!{}", ws.name(), pa), ws.name());
                if !all_dns.iter().any(|x| x.name == dn.name && x.sheet == dn.sheet) {
                    all_dns.push(dn);
                }
            }
            if let Some(pt) = &ps.print_titles {
                let dn = DefinedName::sheet_scoped("_xlnm.Print_Titles", format!("{}!{}", ws.name(), pt), ws.name());
                if !all_dns.iter().any(|x| x.name == dn.name && x.sheet == dn.sheet) {
                    all_dns.push(dn);
                }
            }
        }
    }

    if !all_dns.is_empty() {
        write_str(w, "<definedNames>")?;
        for dn in &all_dns {
            let sheet_attr = match &dn.sheet {
                Some(s) => match worksheets.iter().position(|ws| ws.name() == s.as_str()) {
                    Some(idx) => format!(r#" localSheetId="{}""#, idx),
                    None => {
                        return Err(ExcelrsError::Write(format!(
                            "Defined name '{}' references sheet '{}' which does not exist in the workbook",
                            dn.name, s
                        )))
                    }
                },
                None => String::new(),
            };
            let name_esc = escape(&dn.name);
            let value_esc = escape(&dn.value);
            write_str(
                w,
                &format!(
                    r#"<definedName name="{}"{}>{}</definedName>"#,
                    name_esc, sheet_attr, value_esc
                ),
            )?;
        }
        write_str(w, "</definedNames>")?;
    }

    write_str(w, "</workbook>")?;
    Ok(())
}

fn emit_book_views<W: Write>(w: &mut W, inner: &WorkbookInner) -> Result<(), ExcelrsError> {
    if inner.views.is_empty() {
        return Ok(());
    }
    write_str(w, "<bookViews>")?;
    for v in &inner.views {
        let mut attrs = String::new();
        if let Some(x) = v.x_window {
            attrs.push_str(&format!(" xWindow=\"{}\"", x));
        }
        if let Some(x) = v.y_window {
            attrs.push_str(&format!(" yWindow=\"{}\"", x));
        }
        if let Some(x) = v.window_width {
            attrs.push_str(&format!(" windowWidth=\"{}\"", x));
        }
        if let Some(x) = v.window_height {
            attrs.push_str(&format!(" windowHeight=\"{}\"", x));
        }
        if let Some(x) = v.active_tab {
            attrs.push_str(&format!(" activeTab=\"{}\"", x));
        }
        if let Some(x) = v.first_sheet {
            attrs.push_str(&format!(" firstSheet=\"{}\"", x));
        }
        if let Some(x) = v.minimized {
            attrs.push_str(if x { " minimized=\"1\"" } else { " minimized=\"0\"" });
        }
        if let Some(x) = v.show_horizontal_scroll {
            attrs.push_str(if x {
                " showHorizontalScroll=\"1\""
            } else {
                " showHorizontalScroll=\"0\""
            });
        }
        if let Some(x) = v.show_vertical_scroll {
            attrs.push_str(if x {
                " showVerticalScroll=\"1\""
            } else {
                " showVerticalScroll=\"0\""
            });
        }
        if let Some(x) = v.tab_ratio {
            attrs.push_str(&format!(" tabRatio=\"{}\"", x));
        }
        if let Some(x) = &v.visibility {
            attrs.push_str(&format!(" visibility=\"{}\"", escape(x)));
        }
        write_str(w, &format!("<workbookView{}/>", attrs))?;
    }
    write_str(w, "</bookViews>")?;
    Ok(())
}

fn emit_calc_pr<W: Write>(w: &mut W, inner: &WorkbookInner) -> Result<(), ExcelrsError> {
    let calc = match &inner.calc_properties {
        Some(c) => c,
        None => return Ok(()),
    };
    let mut attrs = String::new();
    if let Some(x) = calc.full_calc_on_load {
        attrs.push_str(if x {
            " fullCalcOnLoad=\"1\""
        } else {
            " fullCalcOnLoad=\"0\""
        });
    }
    if let Some(x) = calc.calc_id {
        attrs.push_str(&format!(" calcId=\"{}\"", x));
    }
    if let Some(x) = &calc.calc_mode {
        attrs.push_str(&format!(" calcMode=\"{}\"", escape(x)));
    }
    if let Some(x) = calc.ref_full_calc {
        attrs.push_str(if x { " refFullCalc=\"1\"" } else { " refFullCalc=\"0\"" });
    }
    if let Some(x) = calc.iterate {
        attrs.push_str(if x { " iterate=\"1\"" } else { " iterate=\"0\"" });
    }
    if let Some(x) = calc.iterate_count {
        attrs.push_str(&format!(" iterateCount=\"{}\"", x));
    }
    if let Some(x) = calc.iterate_delta {
        attrs.push_str(&format!(" iterateDelta=\"{}\"", x));
    }
    write_str(w, &format!("<calcPr{}/>", attrs))?;
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
// xl/worksheets/_rels/sheet{N}.xml.rels
// ---------------------------------------------------------------------------

/// Write the relationships XML for a single worksheet (hyperlinks + comments + drawing + tables).
fn write_sheet_rels<W: Write>(
    w: &mut W,
    hyperlinks: &[SheetHyperlink],
    sheet_num: usize,
    has_comments: bool,
    has_drawing: bool,
    table_rels: &[(u32, u32)],
) -> Result<(), ExcelrsError> {
    write_str(w, r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#)?;
    write_str(
        w,
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
    )?;
    for h in hyperlinks {
        write_str(
            w,
            &format!(
                r#"<Relationship Id="{rid}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="{url}" TargetMode="External"/>"#,
                rid = h.rid,
                url = escape(&h.url),
            ),
        )?;
    }
    // (v1.0.0) comments + drawing relationships
    let mut rel_id = hyperlinks.len() as u32 + 1;
    if has_comments {
        write_str(
            w,
            &format!(
                r#"<Relationship Id="rId{rel_id}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/comments" Target="../comments{sheet_num}.xml"/>"#,
            ),
        )?;
        rel_id += 1;
        write_str(
            w,
            &format!(
                r#"<Relationship Id="rId{rel_id}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/vmlDrawing" Target="../drawings/vmlDrawing{sheet_num}.vml"/>"#,
            ),
        )?;
        rel_id += 1;
    }
    if has_drawing {
        write_str(
            w,
            &format!(
                r#"<Relationship Id="rId{rel_id}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/drawing" Target="../drawings/drawing{sheet_num}.xml"/>"#,
            ),
        )?;
    }
    // (v1.1.0) table relationships
    for (rid, part) in table_rels {
        write_str(
            w,
            &format!(
                r#"<Relationship Id="rId{rid}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/table" Target="../tables/table{part}.xml"/>"#,
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
// Sheet views / protection / autoFilter emission helpers (v0.11.0)
// ---------------------------------------------------------------------------

fn emit_sheet_views<W: Write>(w: &mut W, ws: &Worksheet) -> Result<(), ExcelrsError> {
    let views = ws.get_views_inner();
    if views.is_empty() {
        return Ok(());
    }
    write_str(w, "<sheetViews>")?;
    for sv in &views {
        let state_attr = sv.state.as_deref().unwrap_or("");
        write_str(
            w,
            &format!(
                "<sheetView{}>",
                if !state_attr.is_empty() {
                    format!(" state=\"{}\"", escape(state_attr))
                } else {
                    String::new()
                }
            ),
        )?;
        let has_pane =
            sv.x_split.is_some() || sv.y_split.is_some() || sv.top_left_cell.is_some() || sv.active_pane.is_some();
        if has_pane {
            let mut pane_attrs = String::new();
            if let Some(x) = sv.x_split {
                pane_attrs.push_str(&format!(" xSplit=\"{}\"", x));
            }
            if let Some(y) = sv.y_split {
                pane_attrs.push_str(&format!(" ySplit=\"{}\"", y));
            }
            if let Some(t) = &sv.top_left_cell {
                pane_attrs.push_str(&format!(" topLeftCell=\"{}\"", escape(t)));
            }
            if let Some(a) = &sv.active_pane {
                pane_attrs.push_str(&format!(" activePane=\"{}\"", escape(a)));
            }
            write_str(w, &format!("<pane{}/>", pane_attrs))?;
        }
        write_str(w, "</sheetView>")?;
    }
    write_str(w, "</sheetViews>")?;
    Ok(())
}

fn emit_sheet_protection<W: Write>(w: &mut W, ws: &Worksheet) -> Result<(), ExcelrsError> {
    let prot = ws.get_protection_inner();
    let sp = match prot {
        Some(ref p) => p,
        None => return Ok(()),
    };
    let mut attrs = String::new();
    if let Some(v) = sp.locked {
        attrs.push_str(if v { " locked=\"1\"" } else { " locked=\"0\"" });
    }
    if let Some(v) = sp.auto_filter {
        attrs.push_str(if v { " autoFilter=\"1\"" } else { " autoFilter=\"0\"" });
    }
    if let Some(v) = sp.delete_columns {
        attrs.push_str(if v {
            " deleteColumns=\"1\""
        } else {
            " deleteColumns=\"0\""
        });
    }
    if let Some(v) = sp.delete_rows {
        attrs.push_str(if v { " deleteRows=\"1\"" } else { " deleteRows=\"0\"" });
    }
    if let Some(v) = sp.format_cells {
        attrs.push_str(if v { " formatCells=\"1\"" } else { " formatCells=\"0\"" });
    }
    if let Some(v) = sp.format_columns {
        attrs.push_str(if v {
            " formatColumns=\"1\""
        } else {
            " formatColumns=\"0\""
        });
    }
    if let Some(v) = sp.format_rows {
        attrs.push_str(if v { " formatRows=\"1\"" } else { " formatRows=\"0\"" });
    }
    if let Some(v) = sp.insert_columns {
        attrs.push_str(if v {
            " insertColumns=\"1\""
        } else {
            " insertColumns=\"0\""
        });
    }
    if let Some(v) = sp.insert_hyperlinks {
        attrs.push_str(if v {
            " insertHyperlinks=\"1\""
        } else {
            " insertHyperlinks=\"0\""
        });
    }
    if let Some(v) = sp.insert_rows {
        attrs.push_str(if v { " insertRows=\"1\"" } else { " insertRows=\"0\"" });
    }
    if let Some(v) = sp.pivot_tables {
        attrs.push_str(if v { " pivotTables=\"1\"" } else { " pivotTables=\"0\"" });
    }
    if let Some(v) = sp.select_locked_cells {
        attrs.push_str(if v {
            " selectLockedCells=\"1\""
        } else {
            " selectLockedCells=\"0\""
        });
    }
    if let Some(v) = sp.select_unlocked_cells {
        attrs.push_str(if v {
            " selectUnlockedCells=\"1\""
        } else {
            " selectUnlockedCells=\"0\""
        });
    }
    if let Some(v) = sp.sort {
        attrs.push_str(if v { " sort=\"1\"" } else { " sort=\"0\"" });
    }
    if let Some(v) = &sp.password_hash {
        attrs.push_str(&format!(" passwordHash=\"{}\"", escape(v)));
    }
    if let Some(v) = &sp.salt_value {
        attrs.push_str(&format!(" saltValue=\"{}\"", escape(v)));
    }
    write_str(w, &format!("<sheetProtection{}/>", attrs))?;
    Ok(())
}

fn emit_auto_filter<W: Write>(w: &mut W, ws: &Worksheet) -> Result<(), ExcelrsError> {
    let range = ws.get_auto_filter_range();
    if let Some(ref r) = range {
        write_str(w, &format!("<autoFilter ref=\"{}\"/>", escape(r)))?;
    }
    Ok(())
}

fn emit_header_footer<W: Write>(w: &mut W, ws: &Worksheet) -> Result<(), ExcelrsError> {
    let hf = match ws.get_header_footer_inner() {
        Some(hf) => hf,
        None => return Ok(()),
    };
    let mut attrs = String::new();
    if let Some(v) = hf.align_with_margins {
        attrs.push_str(if v {
            " alignWithMargins=\"1\""
        } else {
            " alignWithMargins=\"0\""
        });
    }
    if let Some(v) = hf.different_first {
        attrs.push_str(if v {
            " differentFirst=\"1\""
        } else {
            " differentFirst=\"0\""
        });
    }
    if let Some(v) = hf.different_odd_even {
        attrs.push_str(if v {
            " differentOddEven=\"1\""
        } else {
            " differentOddEven=\"0\""
        });
    }
    write_str(w, &format!("<headerFooter{}>", attrs))?;
    if let Some(v) = &hf.odd_header {
        write_str(w, &format!("<oddHeader>{}</oddHeader>", escape(v)))?;
    }
    if let Some(v) = &hf.odd_footer {
        write_str(w, &format!("<oddFooter>{}</oddFooter>", escape(v)))?;
    }
    if let Some(v) = &hf.even_header {
        write_str(w, &format!("<evenHeader>{}</evenHeader>", escape(v)))?;
    }
    if let Some(v) = &hf.even_footer {
        write_str(w, &format!("<evenFooter>{}</evenFooter>", escape(v)))?;
    }
    if let Some(v) = &hf.first_header {
        write_str(w, &format!("<firstHeader>{}</firstHeader>", escape(v)))?;
    }
    if let Some(v) = &hf.first_footer {
        write_str(w, &format!("<firstFooter>{}</firstFooter>", escape(v)))?;
    }
    write_str(w, "</headerFooter>")?;
    Ok(())
}

fn emit_page_setup<W: Write>(w: &mut W, ws: &Worksheet) -> Result<(), ExcelrsError> {
    let ps = match ws.get_page_setup_inner() {
        Some(ps) => ps,
        None => return Ok(()),
    };
    // <pageMargins> — attribute order: left, right, top, bottom, header, footer
    if let Some(m) = &ps.margins {
        let f = |v: Option<f64>| v.unwrap_or(0.0);
        write_str(
            w,
            &format!(
                "<pageMargins left=\"{}\" right=\"{}\" top=\"{}\" bottom=\"{}\" header=\"{}\" footer=\"{}\"/>",
                f(m.left),
                f(m.right),
                f(m.top),
                f(m.bottom),
                f(m.header),
                f(m.footer)
            ),
        )?;
    }
    // <pageSetup>
    let mut attrs = String::new();
    if let Some(v) = &ps.orientation {
        attrs.push_str(&format!(" orientation=\"{}\"", escape(v)));
    }
    if let Some(v) = ps.paper_size {
        attrs.push_str(&format!(" paperSize=\"{}\"", v));
    }
    if let Some(v) = ps.fit_to_page {
        attrs.push_str(if v { " fitToPage=\"1\"" } else { " fitToPage=\"0\"" });
    }
    if let Some(v) = ps.fit_to_width {
        attrs.push_str(&format!(" fitToWidth=\"{}\"", v));
    }
    if let Some(v) = ps.fit_to_height {
        attrs.push_str(&format!(" fitToHeight=\"{}\"", v));
    }
    if let Some(v) = ps.horizontal_dpi {
        attrs.push_str(&format!(" horizontalDpi=\"{}\"", v));
    }
    if let Some(v) = ps.vertical_dpi {
        attrs.push_str(&format!(" verticalDpi=\"{}\"", v));
    }
    if let Some(v) = ps.black_and_white {
        attrs.push_str(if v {
            " blackAndWhite=\"1\""
        } else {
            " blackAndWhite=\"0\""
        });
    }
    if let Some(v) = ps.drawing_printed {
        attrs.push_str(if v {
            " drawingPrinted=\"1\""
        } else {
            " drawingPrinted=\"0\""
        });
    }
    if let Some(v) = &ps.cell_comments {
        attrs.push_str(&format!(" cellComments=\"{}\"", escape(v)));
    }
    if let Some(v) = ps.copies {
        attrs.push_str(&format!(" copies=\"{}\"", v));
    }
    write_str(w, &format!("<pageSetup{}/>", attrs))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// xl/worksheets/sheet{N}.xml
// ---------------------------------------------------------------------------

// (v1.2.0) Conditional-formatting rule emission
// ---------------------------------------------------------------------------

fn emit_cf_color_attrs(col: &crate::model::conditional_formatting::CfColor) -> String {
    let mut s = String::new();
    if let Some(a) = &col.argb {
        s.push_str(&format!(r#" rgb="{}""#, escape(a)));
    }
    if let Some(t) = col.theme {
        s.push_str(&format!(r#" theme="{}""#, t));
    }
    if let Some(i) = col.indexed {
        s.push_str(&format!(r#" indexed="{}""#, i));
    }
    if let Some(t) = col.tint {
        s.push_str(&format!(r#" tint="{}""#, t));
    }
    s
}

fn emit_cf_rule<W: Write>(w: &mut W, rule: &crate::model::conditional_formatting::CfRule) -> Result<(), ExcelrsError> {
    let mut attrs = format!(r#"type="{}" priority="{}""#, escape(&rule.r#type), rule.priority);
    if let Some(op) = &rule.operator {
        attrs.push_str(&format!(r#" operator="{}""#, escape(op)));
    }
    if let Some(id) = rule.dxf_id {
        attrs.push_str(&format!(r#" dxfId="{}""#, id));
    }
    if let Some(tp) = &rule.time_period {
        attrs.push_str(&format!(r#" timePeriod="{}""#, escape(tp)));
    }
    if let Some(rank) = rule.rank {
        attrs.push_str(&format!(r#" rank="{}""#, rank));
    }
    if let Some(p) = rule.percent {
        attrs.push_str(&format!(r#" percent="{}""#, if p { "1" } else { "0" }));
    }
    if let Some(b) = rule.bottom {
        attrs.push_str(&format!(r#" bottom="{}""#, if b { "1" } else { "0" }));
    }
    if let Some(t) = &rule.text {
        attrs.push_str(&format!(r#" text="{}""#, escape(t)));
    }
    if let Some(rev) = rule.reverse {
        attrs.push_str(&format!(r#" reverse="{}""#, if rev { "1" } else { "0" }));
    }
    if let Some(sv) = rule.show_value {
        attrs.push_str(&format!(r#" showValue="{}""#, if sv { "1" } else { "0" }));
    }
    write_str(w, &format!("<cfRule {}>", attrs))?;

    if let Some(formulas) = &rule.formula {
        for f in formulas {
            write_str(w, &format!("<formula>{}</formula>", escape(f)))?;
        }
    }

    match rule.r#type.as_str() {
        "colorScale" => {
            if let (Some(cfvo), Some(colors)) = (&rule.cfvo, &rule.color) {
                write_str(w, "<colorScale>")?;
                for c in cfvo {
                    let val = c
                        .value
                        .as_ref()
                        .map(|v| format!(r#" val="{}""#, escape(v)))
                        .unwrap_or_default();
                    write_str(w, &format!(r#"<cfvo type="{}"{}"/>"#, escape(&c.r#type), val))?;
                }
                for col in colors {
                    write_str(w, &format!("<color{}/>", emit_cf_color_attrs(col)))?;
                }
                write_str(w, "</colorScale>")?;
            }
        }
        "dataBar" => {
            if let (Some(cfvo), Some(col)) = (&rule.cfvo, &rule.data_bar_color) {
                write_str(w, "<dataBar>")?;
                for c in cfvo {
                    let val = c
                        .value
                        .as_ref()
                        .map(|v| format!(r#" val="{}""#, escape(v)))
                        .unwrap_or_default();
                    write_str(w, &format!(r#"<cfvo type="{}"{}"/>"#, escape(&c.r#type), val))?;
                }
                write_str(w, &format!("<color{}/>", emit_cf_color_attrs(col)))?;
                write_str(w, "</dataBar>")?;
            }
        }
        "iconSet" => {
            let icon_set = rule.icon_set.as_deref().unwrap_or("3TrafficLights");
            write_str(w, &format!(r#"<iconSet iconSet="{}""#, escape(icon_set)))?;
            if rule.reverse.unwrap_or(false) {
                write_str(w, r#" reverse="1""#)?;
            }
            if !rule.show_value.unwrap_or(true) {
                write_str(w, r#" showValue="0""#)?;
            }
            write_str(w, ">")?;
            if let Some(cfvo) = &rule.cfvo {
                for c in cfvo {
                    let val = c
                        .value
                        .as_ref()
                        .map(|v| format!(r#" val="{}""#, escape(v)))
                        .unwrap_or_default();
                    write_str(w, &format!(r#"<cfvo type="{}"{}"/>"#, escape(&c.r#type), val))?;
                }
            }
            write_str(w, "</iconSet>")?;
        }
        _ => {}
    }

    write_str(w, "</cfRule>")?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn write_sheet_xml<W: Write>(
    w: &mut W,
    ws: &Worksheet,
    string_indices: &HashMap<String, u32>,
    cell_style_indices: &[u32],
    row_style_indices: &[u32],
    hyperlinks: &[SheetHyperlink],
    data_validations: &[crate::model::data_validation::DataValidation],
    drawing_rid: Option<u32>,
    comment_rid: Option<u32>,
    table_rids: &[u32],
) -> Result<(), ExcelrsError> {
    write_str(w, r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#)?;
    write_str(
        w,
        r#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">"#,
    )?;

    // <dimension ref="A1:Z1000"/> — used range
    let dimension = compute_dimension(ws);
    if let Some(dim) = dimension {
        write_str(w, &format!("<dimension ref=\"{}\"/>", dim))?;
    }

    // <sheetViews> — freeze/split panes (v0.11.0)
    emit_sheet_views(w, ws)?;

    write_str(w, "<sheetData>")?;

    write_cells_with_styles(w, ws, string_indices, cell_style_indices, row_style_indices)?;

    write_str(w, "</sheetData>")?;

    // <sheetProtection> — (v0.11.0)
    emit_sheet_protection(w, ws)?;

    // <autoFilter> — (v0.11.0)
    emit_auto_filter(w, ws)?;

    // <mergeCells> — item 1 (v0.5.0)
    let merged_ranges = ws.get_merged_ranges();
    if !merged_ranges.is_empty() {
        write_str(w, &format!(r#"<mergeCells count="{}">"#, merged_ranges.len()))?;
        for range in &merged_ranges {
            write_str(w, &format!(r#"<mergeCell ref="{}"/>"#, escape(range)))?;
        }
        write_str(w, "</mergeCells>")?;
    }

    // (v1.2.0) conditional formatting — after mergeCells, before dataValidations
    let conditional_formats = ws.get_conditional_formatting_inner();
    if !conditional_formats.is_empty() {
        for cf in &conditional_formats {
            if cf.rules.is_empty() {
                continue;
            }
            let cf_line = format!(r#"<conditionalFormatting sqref="{}">"#, escape(&cf.sqref));
            write_str(w, &cf_line)?;
            for rule in &cf.rules {
                emit_cf_rule(w, rule)?;
            }
            write_str(w, "</conditionalFormatting>")?;
        }
    }

    // <dataValidations> - item 3 (v0.8.0)
    if !data_validations.is_empty() {
        write_str(w, &format!(r#"<dataValidations count="{}">"#, data_validations.len()))?;
        for dv in data_validations {
            // Build the dataValidation XML element
            let mut attrs = format!("sqref=\"{}\" type=\"{}\"", escape(&dv.sqref), escape(&dv.r#type));

            if let Some(op) = &dv.operator {
                attrs.push_str(&format!(" operator=\"{}\"", escape(op)));
            }

            if let Some(ab) = dv.allow_blank {
                attrs.push_str(&format!(" allowBlank=\"{}\"", if ab { "1" } else { "0" }));
            }

            if let Some(sim) = dv.show_input_message {
                attrs.push_str(&format!(" showInputMessage=\"{}\"", if sim { "1" } else { "0" }));
            }

            if let Some(sem) = dv.show_error_message {
                attrs.push_str(&format!(" showErrorMessage=\"{}\"", if sem { "1" } else { "0" }));
            }

            if let Some(pt) = &dv.prompt_title {
                attrs.push_str(&format!(" promptTitle=\"{}\"", escape(pt)));
            }

            if let Some(p) = &dv.prompt {
                attrs.push_str(&format!(" prompt=\"{}\"", escape(p)));
            }

            if let Some(et) = &dv.error_title {
                attrs.push_str(&format!(" errorTitle=\"{}\"", escape(et)));
            }

            if let Some(err) = &dv.error {
                attrs.push_str(&format!(" error=\"{}\"", escape(err)));
            }

            if let Some(es) = &dv.error_style {
                attrs.push_str(&format!(" errorStyle=\"{}\"", escape(es)));
            }

            write_str(w, &format!("<dataValidation {}>", attrs))?;
            write_str(w, &format!("<formula1>{}</formula1>", escape(&dv.formula1)))?;
            if let Some(f2) = &dv.formula2 {
                write_str(w, &format!("<formula2>{}</formula2>", escape(f2)))?;
            }
            write_str(w, "</dataValidation>")?;
        }
        write_str(w, "</dataValidations>")?;
    }

    // <hyperlinks> — item 2 (v0.5.0)
    if !hyperlinks.is_empty() {
        write!(w, r#"<hyperlinks count="{}">"#, hyperlinks.len())?;
        for h in hyperlinks {
            write_str(
                w,
                &format!(r#"<hyperlink ref="{}" r:id="{}"/>"#, escape(&h.cell_ref), h.rid),
            )?;
        }
        write_str(w, "</hyperlinks>")?;
    }

    // <pageMargins> + <pageSetup> — (v1.0.0)
    emit_page_setup(w, ws)?;
    // <headerFooter> — (v1.0.0)
    emit_header_footer(w, ws)?;
    // <drawing> — (v1.0.0) link to the drawing part via relationship
    if let Some(rid) = drawing_rid {
        write_str(w, &format!(r#"<drawing r:id="rId{rid}"/>"#))?;
    }

    if let Some(rid) = comment_rid {
        write_str(w, &format!(r#"<legacyDrawing r:id="rId{rid}"/>"#))?;
    }

    // (v1.1.0) table parts — link to table parts via relationship
    if !table_rids.is_empty() {
        write_str(w, &format!(r#"<tableParts count="{}">"#, table_rids.len()))?;
        for rid in table_rids {
            write_str(w, &format!(r#"<tablePart r:id="rId{rid}"/>"#))?;
        }
        write_str(w, "</tableParts>")?;
    }

    write_str(w, "</worksheet>")?;
    Ok(())
}

/// Iterate a worksheet's cells in order, writing `<row>` and `<c>` elements
/// with the style index at each cell.  Also emits `<row s="N">` for rows
/// with a row-level style (including styled empty rows).
/// Returns `Err` if `cell_style_indices` is exhausted before the last cell
/// (writer internal invariant).
fn write_cells_with_styles<W: Write>(
    w: &mut W,
    ws: &Worksheet,
    string_indices: &HashMap<String, u32>,
    cell_style_indices: &[u32],
    row_style_indices: &[u32],
) -> Result<(), ExcelrsError> {
    let mut cell_si = cell_style_indices.iter();
    let mut row_si = row_style_indices.iter();
    for row in ws.rows() {
        let cells = row.written_cells();
        let row_style_idx = *row_si
            .next()
            .ok_or_else(|| ExcelrsError::Write("row_style_indices exhausted mid-sheet (writer bug)".into()))?;
        let has_row_style = row.style().is_some();

        if cells.is_empty() && !has_row_style {
            // Skip completely empty rows (no data, no row style)
            continue;
        }

        // Emit <row> with optional s="N" for row-level style
        if has_row_style {
            write!(w, r#"<row r="{}" s="{}">"#, row.number(), row_style_idx)?;
        } else {
            write!(w, r#"<row r="{}">"#, row.number())?;
        }

        for cell in cells {
            let style_idx = cell_si
                .next()
                .copied()
                .ok_or_else(|| ExcelrsError::Write("cell_style_indices exhausted mid-sheet (writer bug)".into()))?;
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
    let cv = cell
        .value_raw()
        .validate()
        .map_err(|e| ExcelrsError::Write(format!("invalid cell value: {e}")))?;
    let address = cell.address();
    let formula = cell.formula();

    // Open the cell element with style attribute
    write!(w, r#"<c r="{}" s="{}""#, address, style_index)?;

    // Determine cell type and write value attribute
    let cell_type_attr = match cv.value_type.as_str() {
        "String" => Some("t=\"s\""),
        "Boolean" => Some("t=\"b\""),
        "Error" => Some("t=\"e\""),
        "RichText" => Some("t=\"inlineStr\""),
        "Hyperlink" => Some("t=\"s\""),
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
        "RichText" => {
            if let Some(runs) = &cv.rich_text {
                write_str(w, "<is>")?;
                for run in runs {
                    write_str(w, "<r>")?;
                    if let Some(ref font) = run.font {
                        write_str(w, "<rPr>")?;
                        if let Some(sz) = font.size {
                            write_str(w, &format!("<sz val=\"{}\"/>", sz))?;
                        }
                        if let Some(ref name) = font.name {
                            write_str(w, &format!("<rFont val=\"{}\"/>", escape(name)))?;
                        }
                        if let Some(true) = font.bold {
                            write_str(w, "<b/>")?;
                        }
                        if let Some(true) = font.italic {
                            write_str(w, "<i/>")?;
                        }
                        if let Some(ref color) = font.color {
                            write_str(w, &format!("<color rgb=\"{}\"/>", escape(color)))?;
                        }
                        write_str(w, "</rPr>")?;
                    }
                    write_str(w, &format!("<t>{}</t>", escape(&run.text)))?;
                    write_str(w, "</r>")?;
                }
                write_str(w, "</is>")?;
            }
        }
        "Hyperlink" => {
            // Write the display text as a shared string value
            if let Some(text) = &cv.hyperlink_text {
                if let Some(idx) = string_indices.get(text) {
                    write_str(w, &format!("<v>{}</v>", idx))?;
                }
            } else if let Some(url) = &cv.hyperlink {
                if let Some(idx) = string_indices.get(url) {
                    write_str(w, &format!("<v>{}</v>", idx))?;
                }
            }
        }
        "Date" => {
            // v0.13.0: emit the Excel serial as the cell value. With a date number
            // format (injected into the style table) Excel renders it as a date;
            // no `t` attribute is written (dates are numeric cells).
            if let Some(serial) = cv.date_serial {
                write_str(w, &format!("<v>{}</v>", serial))?;
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
        let written = row.written_cells();
        if written.is_empty() {
            continue;
        }
        let r = row.number();
        if r < min_row {
            min_row = r;
        }
        if r > max_row {
            max_row = r;
        }
        for cell in written {
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
    use crate::model::cell::{Cell, CellValue, RichTextRun};
    use crate::model::style::Font;
    use crate::model::workbook_inner::WorkbookInner;
    use crate::reader::xlsx::workbook_inner_from_bytes;
    use std::collections::{BTreeMap, HashMap};

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
        assert_eq!(a1.value_raw().value_type, "Number");
        assert_eq!(a1.value_raw().number, Some(42.0));

        let b1 = ws.get_cell_by_address("B1".into());
        assert_eq!(b1.value_raw().value_type, "String");
        assert_eq!(b1.value_raw().string.as_deref(), Some("Hello"));

        let c1 = ws.get_cell_by_address("C1".into());
        assert_eq!(c1.value_raw().value_type, "Boolean");
        assert_eq!(c1.value_raw().boolean, Some(true));

        let a2 = ws.get_cell_by_address("A2".into());
        assert_eq!(a2.value_raw().value_type, "Number");
        assert_eq!(a2.value_raw().number, Some(std::f64::consts::PI));
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
        assert_eq!(a1.value_raw().string.as_deref(), Some("data"));
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
            ws.get_cell_by_address("A1".into()).value_raw().string.as_deref(),
            Some("apple")
        );
        assert_eq!(
            ws.get_cell_by_address("B1".into()).value_raw().string.as_deref(),
            Some("banana")
        );
        assert_eq!(
            ws.get_cell_by_address("C1".into()).value_raw().string.as_deref(),
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
        assert_eq!(r1c1.value_raw().string.as_deref(), Some("Name"));
        let r1c2 = ws.get_cell_by_address("B1".into());
        assert_eq!(r1c2.value_raw().string.as_deref(), Some("Age"));
        let r1c3 = ws.get_cell_by_address("C1".into());
        assert_eq!(r1c3.value_raw().string.as_deref(), Some("Active"));

        // Row 2
        let r2c1 = ws.get_cell_by_address("A2".into());
        assert_eq!(r2c1.value_raw().string.as_deref(), Some("Alice"));
        let r2c2 = ws.get_cell_by_address("B2".into());
        assert_eq!(r2c2.value_raw().number, Some(30.0));
        let r2c3 = ws.get_cell_by_address("C2".into());
        assert_eq!(r2c3.value_raw().boolean, Some(true));
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

    /// Regression: the <dataValidations> open tag must be well-formed.
    /// A raw-string delimiter typo previously emitted `<dataValidations count="1">#`
    /// (stray '#'), invalid because CT_DataValidations permits only
    /// <dataValidation> children. The reader tolerates stray text, so round-trip
    /// tests did not catch it.
    #[test]
    fn test_write_datavalidation_no_stray_hash() {
        use crate::model::data_validation::DataValidation;

        let mut ws = Worksheet::new("DV".into());
        ws.set_id(1);
        ws.add_data_validation(DataValidation {
            sqref: "A1".into(),
            r#type: "whole".into(),
            operator: Some("between".into()),
            formula1: "1".into(),
            formula2: Some("10".into()),
            allow_blank: Some(false),
            show_input_message: Some(false),
            show_error_message: Some(false),
            prompt: None,
            prompt_title: None,
            error: None,
            error_title: None,
            error_style: None,
        })
        .unwrap();

        let mut inner = WorkbookInner::new();
        inner.worksheets.push(ws);
        let bytes = workbook_to_bytes(&inner).unwrap();

        use std::io::{Cursor, Read};
        let mut archive = zip::read::ZipArchive::new(Cursor::new(&bytes)).unwrap();
        let mut sheet_xml = String::new();
        archive
            .by_name("xl/worksheets/sheet1.xml")
            .unwrap()
            .read_to_string(&mut sheet_xml)
            .unwrap();

        assert!(
            sheet_xml.contains("<dataValidations count=\"1\">"),
            "dataValidations open tag must be well-formed, got: {sheet_xml}"
        );
        assert!(
            !sheet_xml.contains("dataValidations count=\"1\">#"),
            "stray '#' must not appear after the open tag, got: {sheet_xml}"
        );
        assert!(
            sheet_xml.contains("<formula1>1</formula1>"),
            "formula1 must be present, got: {sheet_xml}"
        );
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
        col_a.set_style(serde_json::json!({ "numFmt": "0.00%" })).unwrap();
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
        cell.set_style(serde_json::json!({ "numFmt": "0.00%" })).unwrap();
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

    /// write_cells_with_styles returns Err when cell_style_indices is exhausted early.
    #[test]
    fn test_write_cells_with_styles_exhaustion() {
        let ws = build_test_worksheet();

        let mut buf = Vec::new();
        let string_indices = HashMap::new();
        // worksheet has 4 cells but slice is length 1 → should error, not panic
        let cell_style_indices = vec![0u32];
        // Row style indices must be correct length (2 rows in build_test_worksheet)
        let row_style_indices = vec![0u32, 0u32];

        let result = write_cells_with_styles(&mut buf, &ws, &string_indices, &cell_style_indices, &row_style_indices);
        match result {
            Err(ExcelrsError::Write(msg)) => {
                assert!(
                    msg.contains("cell_style_indices"),
                    "error should mention cell_style_indices: {msg}"
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
                "numFmt": "0.00%",
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

    // -- Regression: getCell().style / .value persist through round-trip (v0.4.0) --

    /// Write a workbook where styles and values are set via `getCell().style = {...}`
    /// and `getCell().value = x` (not via `setCellStyle`/`addRow`), then read back
    /// and verify the data persists. Catches the Arc<Mutex<CellInner>> regression.
    #[test]
    fn test_round_trip_cell_mutation_via_get_cell() {
        use crate::reader::xlsx::workbook_inner_from_bytes;

        let mut inner = WorkbookInner::new();
        let ws = inner.add_worksheet("GetCellMut".into());

        // Populate via add_row, then mutate via getCell
        ws.add_row(vec![serde_json::json!(10), serde_json::json!("x")]);
        ws.add_row(vec![serde_json::json!(20)]);

        // Mutate style via getCell (simulates JS cell.style = {...})
        let mut cell = ws.get_cell_by_address("A1".into());
        cell.set_style(serde_json::json!({
            "font": { "bold": true, "color": "FF00FF00" },
        }))
        .unwrap();

        // Mutate value via getCell (simulates JS cell.value = 42)
        let mut cell = ws.get_cell_by_address("B1".into());
        cell.set_value_raw(CellValue {
            value_type: "String".into(),
            string: Some("mutated".into()),
            ..Default::default()
        });

        // Also style on a second cell
        let mut cell = ws.get_cell_by_address("A2".into());
        cell.set_style(serde_json::json!({
            "fill": { "kind": "solid", "foreground": "FFFF0000" },
        }))
        .unwrap();

        // Round-trip through writer + reader
        let bytes = crate::writer::xlsx::workbook_to_bytes(&inner).unwrap();
        let read_back = workbook_inner_from_bytes(&bytes).unwrap();
        let ws = &read_back.worksheets()[0];

        // Verify A1: bold + green font
        let cell = ws.get_cell_by_address("A1".into());
        let style = cell.style().expect("A1 style should round-trip");
        assert_eq!(style.font.as_ref().and_then(|f| f.bold), Some(true));
        assert_eq!(style.font.as_ref().and_then(|f| f.color.as_deref()), Some("FF00FF00"));

        // Verify B1: value was mutated
        let cell = ws.get_cell_by_address("B1".into());
        assert_eq!(cell.value_raw().string.as_deref(), Some("mutated"));

        // Verify A2: red fill
        let cell = ws.get_cell_by_address("A2".into());
        let style = cell.style().expect("A2 style should round-trip");
        assert_eq!(
            style.fill.as_ref().and_then(|f| f.foreground.as_deref()),
            Some("FFFF0000")
        );
        assert_eq!(style.fill.as_ref().map(|f| f.kind.as_str()), Some("solid"));
    }

    /// Merged cell ranges survive a write → read round-trip (v0.5.0 writer,
    /// this change adds the reader path). The anchor keeps its value and the
    /// other cells in the range carry no phantom value.
    #[test]
    fn test_round_trip_merge_cells() {
        use crate::model::cell::CellValue;
        use crate::reader::xlsx::workbook_inner_from_bytes;

        let mut inner = WorkbookInner::new();
        let ws = inner.add_worksheet("Merge".into());
        // Only the top-left anchor (B2) holds a value; the rest of the merge
        // range is left empty so we can assert no phantom value on read.
        ws.insert_cell_value(2, 2, CellValue::string("anchor"));
        ws.merge_cells("B2:D4".into()).unwrap();

        let bytes = crate::writer::xlsx::workbook_to_bytes(&inner).unwrap();
        let read_back = workbook_inner_from_bytes(&bytes).unwrap();
        let ws = &read_back.worksheets()[0];

        let ranges = ws.get_merged_ranges();
        assert!(
            ranges.iter().any(|r| r == "B2:D4"),
            "merged range B2:D4 should round-trip"
        );

        // Anchor keeps its value.
        let anchor = ws.get_cell_by_address("B2".into());
        assert_eq!(anchor.value_raw().string.as_deref(), Some("anchor"));
        // Non-master cell carries no value.
        let other = ws.get_cell_by_address("C3".into());
        assert_eq!(other.value_raw().value_type.as_str(), "Null");
    }

    /// Row-level style survives a write → read round-trip. The writer emits
    /// `<row s="N">`; this change restores it into Row.style on read.
    #[test]
    fn test_round_trip_row_style() {
        use crate::reader::xlsx::workbook_inner_from_bytes;

        let mut inner = WorkbookInner::new();
        let ws = inner.add_worksheet("RowStyle".into());
        ws.add_row(vec![serde_json::json!("a"), serde_json::json!("b")]);
        ws.add_row(vec![serde_json::json!("c"), serde_json::json!("d")]);
        ws.get_row(2)
            .set_style(serde_json::json!({
                "font": { "bold": true, "color": "FFFF0000" },
                "fill": { "kind": "solid", "foreground": "FFFFFFFF" },
            }))
            .unwrap();

        let bytes = crate::writer::xlsx::workbook_to_bytes(&inner).unwrap();
        let read_back = workbook_inner_from_bytes(&bytes).unwrap();
        let ws = &read_back.worksheets()[0];

        let row = ws.get_row(2);
        let style = row.style().expect("row 2 style should round-trip");
        assert_eq!(style.font.as_ref().unwrap().bold, Some(true));
        assert_eq!(style.font.as_ref().unwrap().color.as_deref(), Some("FFFF0000"));
        assert_eq!(
            style.fill.as_ref().and_then(|f| f.foreground.as_deref()),
            Some("FFFFFFFF")
        );
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

    fn build_hyperlink_workbook() -> WorkbookInner {
        let mut inner = WorkbookInner::new();
        let ws = inner.add_worksheet("Sheet1".into());
        ws.insert_cell_value(
            1,
            1,
            CellValue::hyperlink("https://example.com/target", Some("Example".into())),
        );
        ws.insert_cell_value(1, 2, CellValue::hyperlink("https://x.com/u", None));
        inner
    }

    fn workbook_inner_from_path(path: &Path) -> Result<WorkbookInner, ExcelrsError> {
        use std::io::Read;
        let mut file = std::fs::File::open(path).map_err(ExcelrsError::Io)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data).map_err(ExcelrsError::Io)?;
        workbook_inner_from_bytes(&data)
    }

    #[test]
    fn test_read_does_not_create_phantom_cells() {
        // Regression: ws.getCellByRc(r,c) must NOT emit phantom <c> for cells
        // that were only read (not written). Inspects raw sheet XML inside the ZIP
        // because calamine's reader already skips empty cells.
        use std::io::Read;

        let mut inner = WorkbookInner::new();
        let ws = inner.add_worksheet("Phantom".into());
        ws.add_row(vec![serde_json::json!(1)]);

        // Read a cell at col 5 (E1) — row exists, so get_or_create inserts a
        // null Cell. After fix, the writer must skip it.
        let _cell = ws.get_cell_by_rc(1, 5);

        // Write to ZIP, extract sheet1.xml
        let bytes = crate::writer::xlsx::workbook_to_bytes(&inner).unwrap();
        let cursor = std::io::Cursor::new(&bytes[..]);
        let mut archive = zip::ZipArchive::new(cursor).unwrap();
        let mut sheet_xml = String::new();
        archive
            .by_name("xl/worksheets/sheet1.xml")
            .unwrap()
            .read_to_string(&mut sheet_xml)
            .unwrap();

        // Before fix: sheet contains `<c r="E1"`  (phantom cell written)
        // After fix:  only `<c r="A1"` from add_row([1])
        assert!(
            !sheet_xml.contains("E1"),
            "sheet must not contain phantom cell E1: {sheet_xml}"
        );
        assert!(sheet_xml.contains(r#"c r="A1""#), "sheet must contain real cell A1");
    }

    /// Rich-text with a valid font color is emitted correctly.
    /// Note: XML injection through font color is now blocked at the validation
    /// layer (Font::validate rejects non-hex colors before they reach the writer).
    #[test]
    fn test_rich_text_valid_font_color_emitted() {
        let mut cell = Cell::new("A1".into(), 1, 1);
        cell.set_value_raw(CellValue::rich_text(vec![RichTextRun {
            text: "hello".into(),
            font: Some(Font {
                color: Some("FFFF0000".into()),
                bold: Some(true),
                ..Default::default()
            }),
        }]));
        let mut buf = Vec::new();
        write_cell_xml(&mut buf, &cell, &HashMap::new(), 0).unwrap();
        let xml = String::from_utf8(buf).unwrap();
        assert!(
            xml.contains(r##"<color rgb="FFFF0000"/>"##),
            "font color missing: {xml}"
        );
        assert!(xml.contains("<b/>"), "bold missing: {xml}");
        assert!(xml.contains("<t>hello</t>"), "text missing: {xml}");
        assert!(xml.contains(r##"t="inlineStr""##), "must be inlineStr: {xml}");
    }

    /// Invalid rich-text font color must be rejected at write time.
    #[test]
    fn test_invalid_rich_text_font_rejected_at_write() {
        use crate::model::workbook_inner::WorkbookInner;

        let mut inner = WorkbookInner::new();
        let ws = inner.add_worksheet("S".into());
        ws.insert_cell_value(
            1,
            1,
            CellValue::rich_text(vec![RichTextRun {
                text: "x".into(),
                font: Some(Font {
                    color: Some("ZZZZZZ".into()),
                    ..Default::default()
                }),
            }]),
        );
        let res = crate::writer::xlsx::workbook_to_bytes(&inner);
        assert!(
            res.is_err(),
            "invalid rich-text font color must be rejected at write: {:?}",
            res.ok()
        );
    }

    /// CellValue::validate must reject NaN font size in rich text.
    #[test]
    fn test_cell_value_rich_text_font_validated() {
        let cv = CellValue::rich_text(vec![RichTextRun {
            text: "x".into(),
            font: Some(Font {
                size: Some(f64::NAN),
                ..Default::default()
            }),
        }]);
        assert!(cv.validate().is_err(), "NaN font size in rich text should be rejected");
    }

    // ---- BUG 5: Hyperlink data loss tests ----

    /// Verify that a worksheet with hyperlinks gets a per-sheet .rels file
    /// with external Relationship entries pointing to the target URLs.
    #[test]
    fn test_hyperlink_writes_sheet_rels_part() {
        use std::io::Read;

        let inner = build_hyperlink_workbook();
        let bytes = workbook_to_bytes(&inner).unwrap();
        let mut archive = zip::read::ZipArchive::new(std::io::Cursor::new(&bytes)).unwrap();

        // .rels part must exist
        let mut rels = String::new();
        archive
            .by_name("xl/worksheets/_rels/sheet1.xml.rels")
            .expect("sheet .rels part should exist")
            .read_to_string(&mut rels)
            .unwrap();

        assert!(
            rels.contains("<Relationship"),
            "sheet .rels must contain Relationship elements: {rels}"
        );
        assert!(
            rels.contains(r##"TargetMode="External""##),
            "hyperlink Relationship must have TargetMode=External: {rels}"
        );
        assert!(
            rels.contains("https://example.com/target"),
            "sheet .rels must contain the hyperlink URL"
        );
        assert!(
            rels.contains("https://x.com/u"),
            "sheet .rels must contain the second hyperlink URL"
        );
    }

    /// Verify that sheet XML includes xmlns:r namespace and <hyperlinks> block
    /// with correct ref and r:id attributes.
    #[test]
    fn test_hyperlink_emits_hyperlinks_block() {
        use std::io::Read;

        let inner = build_hyperlink_workbook();
        let bytes = workbook_to_bytes(&inner).unwrap();
        let mut archive = zip::read::ZipArchive::new(std::io::Cursor::new(&bytes)).unwrap();
        let mut sheet_xml = String::new();
        archive
            .by_name("xl/worksheets/sheet1.xml")
            .unwrap()
            .read_to_string(&mut sheet_xml)
            .unwrap();

        assert!(
            sheet_xml.contains(r##"xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships""##),
            "sheet must declare xmlns:r"
        );
        assert!(
            sheet_xml.contains("<hyperlinks"),
            "sheet must contain <hyperlinks> block"
        );
        assert!(
            sheet_xml.contains(r##"ref="A1" r:id="rId1""##),
            "sheet must reference A1 with rId1"
        );
        assert!(
            sheet_xml.contains(r##"ref="B1" r:id="rId2""##),
            "sheet must reference B1 with rId2"
        );
    }

    /// Verify that hyperlink URLs are preserved in the .rels file and that
    /// cell display text is written as a shared-string value (t="s").
    #[test]
    fn test_hyperlink_url_preserved_round_trip() {
        use std::io::Read;

        let inner = build_hyperlink_workbook();
        let bytes = workbook_to_bytes(&inner).unwrap();
        let mut archive = zip::read::ZipArchive::new(std::io::Cursor::new(&bytes)).unwrap();

        // Check .rels for exact URL
        let mut rels = String::new();
        archive
            .by_name("xl/worksheets/_rels/sheet1.xml.rels")
            .unwrap()
            .read_to_string(&mut rels)
            .unwrap();
        assert!(
            rels.contains("Target=\"https://example.com/target\""),
            ".rels must contain the original hyperlink URL: {rels}"
        );

        // Check sheet XML: A1 still has t="s" for display text
        let mut sheet_xml = String::new();
        archive
            .by_name("xl/worksheets/sheet1.xml")
            .unwrap()
            .read_to_string(&mut sheet_xml)
            .unwrap();
        assert!(
            sheet_xml.contains(r##"c r="A1" s="0" t="s""##),
            "hyperlink cell A1 should be t=\"s\" (shared string): {sheet_xml}"
        );
    }

    /// When hyperlink_text is None, the display text should fall back to the
    /// URL itself in the shared strings table.
    #[test]
    fn test_hyperlink_no_text_falls_back_to_url() {
        use std::io::Read;

        let inner = build_hyperlink_workbook();
        let bytes = workbook_to_bytes(&inner).unwrap();
        let mut archive = zip::read::ZipArchive::new(std::io::Cursor::new(&bytes)).unwrap();

        // Read shared strings
        let mut ss = String::new();
        archive
            .by_name("xl/sharedStrings.xml")
            .unwrap()
            .read_to_string(&mut ss)
            .unwrap();

        // The second hyperlink has no hyperlink_text, so URL "https://x.com/u"
        // must appear as a shared string entry
        assert!(
            ss.contains("https://x.com/u"),
            "shared strings must contain the URL as display text fallback: {ss}"
        );

        // Also verify the cell in sheet.xml references the correct shared string index
        let mut sheet_xml = String::new();
        archive
            .by_name("xl/worksheets/sheet1.xml")
            .unwrap()
            .read_to_string(&mut sheet_xml)
            .unwrap();

        // B1 is the hyperlink with no display text; it must have t="s" and <v> pointing
        // to the shared string index for "https://x.com/u"
        assert!(
            sheet_xml.contains(r##"c r="B1" s="0" t="s""##),
            "hyperlink cell B1 should be t=\"s\": {sheet_xml}"
        );
        // Verify <v> exists (the index may vary; just confirm there's a numeric <v>)
        assert!(sheet_xml.contains("<v>"), "B1 should have a shared-string index <v>");
    }

    // ---- P1: sheet name XML escaping (lock-in) ----

    /// Sheet names with special XML chars must be escaped in workbook.xml.
    /// This is a lock-in test: the escaping is already implemented.
    #[test]
    fn test_sheet_name_xml_escaped() {
        use std::io::{Cursor, Read};

        let mut ws = Worksheet::new("A & B <x> \"q\"".into());
        ws.set_id(1);
        let mut inner = WorkbookInner::new();
        inner.worksheets.push(ws);
        let bytes = workbook_to_bytes(&inner).unwrap();

        // Extract workbook.xml
        let mut archive = zip::ZipArchive::new(Cursor::new(&bytes)).unwrap();
        let mut wb = String::new();
        archive
            .by_name("xl/workbook.xml")
            .unwrap()
            .read_to_string(&mut wb)
            .unwrap();

        assert!(
            wb.contains(r##"name="A &amp; B &lt;x&gt; &quot;q&quot;"##),
            "sheet name must be XML-escaped: {wb}"
        );
        assert!(
            !wb.contains(r##"name="A & B"##),
            "raw unescaped chars would break workbook.xml: {wb}"
        );
    }

    // ---- P2a: row style emission (lock-in) ----

    /// A row with a style but no cells must still be emitted with s="M".
    #[test]
    fn test_row_style_emitted_in_sheet_xml() {
        use std::io::{Cursor, Read};

        let mut ws = Worksheet::new("Styled".into());
        ws.set_id(1);
        // Row 2 gets a style but no cells -- must still emit <row r="2" s="N">
        ws.get_row(2)
            .set_style(serde_json::json!({ "numFmt": "0.00%" }))
            .unwrap();
        let mut inner = WorkbookInner::new();
        inner.worksheets.push(ws);
        let bytes = workbook_to_bytes(&inner).unwrap();

        let mut archive = zip::ZipArchive::new(Cursor::new(&bytes)).unwrap();
        let mut sheet = String::new();
        archive
            .by_name("xl/worksheets/sheet1.xml")
            .unwrap()
            .read_to_string(&mut sheet)
            .unwrap();

        assert!(
            sheet.contains(r##"<row r="2" s=""##),
            "styled row 2 must emit <row r=\"2\" s=\"M\">: {sheet}"
        );
    }

    // ---- P2b: hyperlink per-sheet rId isolation (lock-in) ----

    /// Hyperlinks in different sheets get independent rId numbering and
    /// separate .rels files.
    #[test]
    fn test_hyperlink_per_sheet_rid_isolation() {
        use std::io::{Cursor, Read};

        // Sheet 1: hyperlink at A1
        let mut ws1 = Worksheet::new("Sheet1".into());
        ws1.set_id(1);
        ws1.insert_cell_value(1, 1, CellValue::hyperlink("https://example.com/s1", Some("S1".into())));

        // Sheet 2: different hyperlink at A1
        let mut ws2 = Worksheet::new("Sheet2".into());
        ws2.set_id(2);
        ws2.insert_cell_value(1, 1, CellValue::hyperlink("https://example.com/s2", Some("S2".into())));

        let mut inner = WorkbookInner::new();
        inner.worksheets.push(ws1);
        inner.worksheets.push(ws2);
        let bytes = workbook_to_bytes(&inner).unwrap();

        let mut archive = zip::ZipArchive::new(Cursor::new(&bytes)).unwrap();

        // Sheet 1 rels: rId1 -> s1 url
        let mut rels1 = String::new();
        archive
            .by_name("xl/worksheets/_rels/sheet1.xml.rels")
            .unwrap()
            .read_to_string(&mut rels1)
            .unwrap();
        assert!(rels1.contains(r##"Id="rId1""##));
        assert!(rels1.contains("https://example.com/s1"));

        // Sheet 2 rels: rId1 -> s2 url (isolated!)
        let mut rels2 = String::new();
        archive
            .by_name("xl/worksheets/_rels/sheet2.xml.rels")
            .unwrap()
            .read_to_string(&mut rels2)
            .unwrap();
        assert!(rels2.contains(r##"Id="rId1""##));
        assert!(rels2.contains("https://example.com/s2"));

        // Cross-check: no leakage
        assert!(
            !rels1.contains("https://example.com/s2"),
            "rId must not leak across sheets"
        );
        assert!(
            !rels2.contains("https://example.com/s1"),
            "rId must not leak across sheets"
        );

        // Sheet XML must also have <hyperlinks> block for each sheet
        let mut sheet1 = String::new();
        archive
            .by_name("xl/worksheets/sheet1.xml")
            .unwrap()
            .read_to_string(&mut sheet1)
            .unwrap();
        assert!(
            sheet1.contains("<hyperlinks"),
            "Sheet 1 must have <hyperlinks>: {sheet1}"
        );
        assert!(sheet1.contains(r##"ref="A1" r:id="rId1""##));

        let mut sheet2 = String::new();
        archive
            .by_name("xl/worksheets/sheet2.xml")
            .unwrap()
            .read_to_string(&mut sheet2)
            .unwrap();
        assert!(
            sheet2.contains("<hyperlinks"),
            "Sheet 2 must have <hyperlinks>: {sheet2}"
        );
        assert!(sheet2.contains(r##"ref="A1" r:id="rId1""##));
    }

    /// F2: hyperlink display text survives round-trip
    #[test]
    fn test_hyperlink_display_text_round_trip() {
        use crate::reader::xlsx::workbook_inner_from_bytes;

        let mut ws = Worksheet::new("HyperlinkText".into());
        ws.set_id(1);

        // Insert a hyperlink with display text
        ws.insert_cell_value(
            1,
            1,
            CellValue::hyperlink("https://example.com/target", Some("Display Me".into())),
        );

        let mut inner = WorkbookInner::new();
        inner.worksheets.push(ws);

        let bytes = workbook_to_bytes(&inner).unwrap();
        let re_read = workbook_inner_from_bytes(&bytes).unwrap();

        let cell = re_read.worksheets()[0].get_cell_by_address("A1".into());
        assert_eq!(
            cell.value_raw().hyperlink_text.as_deref(),
            Some("Display Me"),
            "hyperlink display text must survive round-trip"
        );
        assert_eq!(
            cell.value_raw().hyperlink.as_deref(),
            Some("https://example.com/target"),
            "hyperlink URL must survive round-trip"
        );
    }

    #[test]
    fn test_write_defined_name_unresolved_sheet_errors() {
        use crate::model::workbook_inner::WorkbookInner;
        let mut inner = WorkbookInner::new();
        inner.worksheets.push(Worksheet::new("Sheet1".into()));
        inner.set_defined_names(vec![DefinedName::sheet_scoped("X", "1", "GhostSheet")]);
        let mut out = Vec::new();
        let result = write_workbook_xml(&mut out, &inner.worksheets, &inner);
        assert!(result.is_err(), "should error on unresolved sheet scope");
    }

    #[test]
    fn test_write_defined_name_resolved_sheet_ok() {
        use crate::model::workbook_inner::WorkbookInner;
        let ws = Worksheet::new("Sheet1".into());
        let mut inner = WorkbookInner::new();
        inner.worksheets.push(ws);
        inner.set_defined_names(vec![DefinedName::sheet_scoped("X", "1", "Sheet1")]);
        let mut out = Vec::new();
        let result = write_workbook_xml(&mut out, &inner.worksheets, &inner);
        assert!(result.is_ok(), "existing sheet should succeed");
        let output = String::from_utf8(out).unwrap();
        assert!(output.contains(r##"localSheetId="0""##), "should emit localSheetId");
    }

    /// F3: password_hash/salt_value survive round-trip
    #[test]
    fn test_password_hash_salt_round_trip() {
        use crate::model::sheet_protection::SheetProtection;
        use crate::reader::xlsx::workbook_inner_from_bytes;

        let mut ws = Worksheet::new("PwHash".into());
        ws.set_id(1);

        let sp = SheetProtection {
            password_hash: Some("abc123".into()),
            salt_value: Some("xyz789".into()),
            ..Default::default()
        };
        ws.set_protection(Some(sp));

        let mut inner = WorkbookInner::new();
        inner.worksheets.push(ws);

        let bytes = workbook_to_bytes(&inner).unwrap();
        let re_read = workbook_inner_from_bytes(&bytes).unwrap();

        let read_sp = re_read.worksheets()[0].protection();
        assert!(read_sp.is_some(), "protection should survive round-trip");
        let read_sp = read_sp.unwrap();
        assert_eq!(
            read_sp.password_hash.as_deref(),
            Some("abc123"),
            "password_hash should round-trip"
        );
        assert_eq!(
            read_sp.salt_value.as_deref(),
            Some("xyz789"),
            "salt_value should round-trip"
        );
    }

    /// F3 stronger: XML-special-character values survive round-trip escaping
    #[test]
    fn test_password_hash_salt_xml_escaping() {
        use crate::model::sheet_protection::SheetProtection;
        use crate::reader::xlsx::workbook_inner_from_bytes;

        let mut ws = Worksheet::new("PwEsc".into());
        ws.set_id(1);

        let raw_hash = r##"abc"123&456<789>0"##;
        let raw_salt = r##"x&y<z>"1"##;
        let sp = SheetProtection {
            password_hash: Some(raw_hash.into()),
            salt_value: Some(raw_salt.into()),
            ..Default::default()
        };
        ws.set_protection(Some(sp));

        let mut inner = WorkbookInner::new();
        inner.worksheets.push(ws);

        let bytes = workbook_to_bytes(&inner).unwrap();
        let re_read = workbook_inner_from_bytes(&bytes).unwrap();

        let read_sp = re_read.worksheets()[0].protection();
        assert!(read_sp.is_some(), "protection should survive round-trip");
        let read_sp = read_sp.unwrap();
        assert_eq!(
            read_sp.password_hash.as_deref(),
            Some(raw_hash),
            "password_hash with XML special chars should round-trip"
        );
        assert_eq!(
            read_sp.salt_value.as_deref(),
            Some(raw_salt),
            "salt_value with XML special chars should round-trip"
        );
    }

    // ---- v0.11.0 round-trip: autoFilter, views, protection, hyperlinks ----

    #[test]
    fn test_round_trip_v0_11_0_features() {
        use crate::model::cell::CellValue;
        use crate::model::sheet_protection::SheetProtection;
        use crate::model::sheet_view::SheetView;
        use crate::reader::xlsx::workbook_inner_from_bytes;

        let mut inner = WorkbookInner::new();
        let mut ws = Worksheet::new("V0110".into());
        ws.set_id(1);

        // Set auto-filter
        ws.set_auto_filter(Some("A1:C1".into()));
        assert_eq!(ws.auto_filter().as_deref(), Some("A1:C1"));

        // Set views (freeze pane)
        let sv = SheetView {
            state: Some("frozen".into()),
            x_split: Some(2),
            y_split: Some(1),
            top_left_cell: Some("C2".into()),
            active_pane: Some("bottomRight".into()),
        };
        ws.set_views(vec![sv]);
        assert_eq!(ws.views().len(), 1);
        assert_eq!(ws.views()[0].x_split, Some(2));

        // Set protection
        let sp = SheetProtection {
            select_locked_cells: Some(true),
            format_cells: Some(false),
            sort: Some(true),
            password_hash: Some("abc123".into()),
            salt_value: Some("xyz789".into()),
            ..Default::default()
        };
        ws.set_protection(Some(sp));
        assert!(ws.protection().is_some());

        // Add a string cell + hyperlink
        ws.add_row(vec![serde_json::json!("click me")]);
        let cv = CellValue::hyperlink("https://example.com", Some("click me".into()));
        ws.insert_cell_value(1, 1, cv);

        inner.worksheets.push(ws);

        // Write -> read back
        let bytes = workbook_to_bytes(&inner).unwrap();
        let re_read = workbook_inner_from_bytes(&bytes).unwrap();

        let ws2 = &re_read.worksheets()[0];
        assert_eq!(ws2.name(), "V0110");

        // Assert autoFilter survived
        assert_eq!(
            ws2.auto_filter().as_deref(),
            Some("A1:C1"),
            "autoFilter should round-trip"
        );

        // Assert views survived
        assert_eq!(ws2.views().len(), 1, "views should round-trip");
        assert_eq!(ws2.views()[0].state.as_deref(), Some("frozen"));
        assert_eq!(ws2.views()[0].x_split, Some(2));
        assert_eq!(ws2.views()[0].y_split, Some(1));

        // Assert protection survived
        let read_sp = ws2.protection();
        assert!(read_sp.is_some(), "protection should round-trip");
        let read_sp = read_sp.unwrap();
        assert_eq!(read_sp.select_locked_cells, Some(true));
        assert_eq!(read_sp.format_cells, Some(false));
        assert_eq!(read_sp.sort, Some(true));
        assert_eq!(
            read_sp.password_hash.as_deref(),
            Some("abc123"),
            "password_hash should round-trip"
        );
        assert_eq!(
            read_sp.salt_value.as_deref(),
            Some("xyz789"),
            "salt_value should round-trip"
        );

        // Assert hyperlink survived
        let cell_a1 = ws2.get_cell_by_address("A1".into());
        let val = cell_a1.value_raw();
        assert_eq!(
            val.hyperlink.as_deref(),
            Some("https://example.com"),
            "hyperlink should round-trip"
        );
    }
}
