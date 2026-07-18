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
use crate::model::cell::{CellValue, RichTextRun};
use crate::model::header_footer::HeaderFooter;
use crate::model::page_setup::{PageMargins, PageSetup};
use crate::model::style::{Font, Style};
use crate::model::workbook::Workbook;
use crate::model::workbook_inner::WorkbookInner;

use crate::model::comment::CellComment;
use crate::model::image::{ImageAnchor, WorksheetImage};
use crate::model::table::{Table, TableColumn, TableRow, TableStyle};
use crate::model::worksheet::Worksheet;

use super::styles::{self, SheetStyleMap, StyleTableRead};
use super::workbook;

// ---------------------------------------------------------------------------

/// Maximum decompressed bytes per zip entry (16 MiB). Used to prevent zip-bomb OOM.
const MAX_ENTRY_BYTES: u64 = 16 * 1024 * 1024;

/// Maximum XML reader events per sheet parse. Used to prevent runaway parsing.
const MAX_EVENTS: usize = 5_000_000;
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
    let mut inner = workbook_to_inner_model(&mut workbook, &style_table, &sheet_style_maps)?;

    // Step 3.5: parse data validations from sheet XML and attach to worksheets
    let per_sheet_validations = parse_sheet_data_validations(data, sheet_count)?;
    for (i, dvs) in per_sheet_validations.into_iter().enumerate() {
        for dv in dvs {
            inner.worksheets[i].insert_data_validation(dv);
        }
    }

    // Step 3.55: parse conditional formatting from sheet XML and attach to worksheets
    let per_sheet_cf = parse_sheet_conditional_formattings(data, sheet_count, &style_table.dxfs)?;
    for (i, cfs) in per_sheet_cf.into_iter().enumerate() {
        for cf in cfs {
            inner.worksheets[i].insert_conditional_formatting(cf);
        }
    }
    // Preserve foreign dxfs (e.g. pivot tables) for round-trip fidelity.
    inner.dxfs = style_table.dxfs.clone();

    // Step 3.6: parse auto-filter ranges from sheet XML and attach
    let per_sheet_auto_filters = parse_sheet_auto_filters(data, sheet_count)?;
    for (i, af) in per_sheet_auto_filters.into_iter().enumerate() {
        if let Some(ref range) = af {
            inner.worksheets[i].set_auto_filter_range(Some(range.clone()));
        }
    }

    // Step 3.7: parse sheet views (freeze/split panes) and attach
    let per_sheet_views = parse_sheet_views(data, sheet_count)?;
    for (i, views) in per_sheet_views.into_iter().enumerate() {
        if !views.is_empty() {
            inner.worksheets[i].set_views_inner(views);
        }
    }

    // Step 3.8: parse sheet protection flags and attach
    let per_sheet_protection = parse_sheet_protection(data, sheet_count)?;
    for (i, prot) in per_sheet_protection.into_iter().enumerate() {
        if let Some(sp) = prot {
            inner.worksheets[i].set_protection_inner(Some(sp));
        }
    }

    // Step 3.9: parse hyperlinks + resolve r:id via sheet rels
    let per_sheet_hyperlinks = parse_sheet_hyperlinks(data, sheet_count)?;
    for (i, links) in per_sheet_hyperlinks.into_iter().enumerate() {
        for (ref_, url) in &links {
            // Resolve cell address to set a Hyperlink CellValue
            if let Some((row, col)) = ref_to_rowcol(ref_) {
                let existing = inner.worksheets[i].get_cell_by_rc(row, col);
                let display_text = existing.value_raw().string.filter(|s| !s.is_empty());
                let cv = CellValue::hyperlink(url.clone(), display_text);
                inner.worksheets[i].insert_cell_value(row, col, cv);
            }
        }
    }

    // Step 3.10: parse rich-text inline strings and attach
    let per_sheet_rich_text = parse_sheet_rich_text(data, sheet_count);
    for (i, cells) in per_sheet_rich_text.into_iter().enumerate() {
        for (row, col, runs) in &cells {
            let cv = CellValue::rich_text(runs.clone());
            inner.worksheets[i].insert_cell_value(*row, *col, cv);
        }
    }

    // Step 3.11: parse header/footer and page setup and attach (v1.0.0)
    let per_sheet_hf = parse_sheet_header_footers(data, sheet_count)?;
    for (i, hf) in per_sheet_hf.into_iter().enumerate() {
        if let Some(hf) = hf {
            inner.worksheets[i].set_header_footer_inner(Some(hf));
        }
    }
    let per_sheet_ps = parse_sheet_page_setups(data, sheet_count)?;
    for (i, ps) in per_sheet_ps.into_iter().enumerate() {
        if let Some(ps) = ps {
            inner.worksheets[i].set_page_setup_inner(Some(ps));
        }
    }

    // Step 3.12: parse cell comments and attach (v1.0.0)
    let per_sheet_comments = parse_sheet_comments(data, sheet_count)?;
    for (i, comments) in per_sheet_comments.into_iter().enumerate() {
        for (ref_addr, comment) in comments {
            if let Some((row, col)) = ref_to_rowcol(&ref_addr) {
                inner.worksheets[i].insert_cell_comment(row, col, comment);
            }
        }
    }

    // Step 3.13: parse images and attach (v1.0.0)
    let per_sheet_images = parse_sheet_images(data, sheet_count)?;
    for (i, imgs) in per_sheet_images.into_iter().enumerate() {
        for img in imgs {
            inner.worksheets[i].insert_image(img);
        }
    }

    // Step 3.14: parse worksheet tables and attach (v1.1.0)
    let per_sheet_tables = parse_sheet_tables(data, sheet_count)?;
    for (i, tables) in per_sheet_tables.into_iter().enumerate() {
        for mut t in tables {
            t.rows = reconstruct_table_rows(&inner.worksheets[i], &t);
            inner.worksheets[i].insert_table(t);
        }
    }

    // Step 3.15: merged cell ranges — writer emits <mergeCells> since v0.5.0,
    // but the reader never restored them. Attach per-sheet ranges.
    let per_sheet_merges = parse_sheet_merge_cells(data, sheet_count)?;
    for (i, ranges) in per_sheet_merges.into_iter().enumerate() {
        for range in ranges {
            inner.worksheets[i].insert_merge_range(range);
        }
    }

    // Step 3.16: row-level styles — writer emits <row s="N">, but the reader
    // never restored Row.style. Resolve the xf index through the same style
    // table used for cells and attach to the row (mirrors insert_cell_style).
    let per_sheet_row_styles = parse_sheet_row_styles(data, sheet_count, &style_table)?;
    for (i, styles) in per_sheet_row_styles.into_iter().enumerate() {
        for (row_num, style) in styles {
            inner.worksheets[i].insert_row_style(row_num, style);
        }
    }

    // Step 3.17: row outline levels (grouping) — writer emits <row outlineLevel="N">.
    let per_sheet_row_outline = parse_sheet_row_outline_levels(data, sheet_count)?;
    for (i, levels) in per_sheet_row_outline.into_iter().enumerate() {
        for (row_num, level) in levels {
            inner.worksheets[i].insert_row_outline_level(row_num, level);
        }
    }

    // Step 3.18: column outline levels (grouping) — writer emits <col outlineLevel="N">.
    let per_sheet_col_outline = parse_sheet_col_outline_levels(data, sheet_count)?;
    for (i, levels) in per_sheet_col_outline.into_iter().enumerate() {
        for (col_num, level) in levels {
            inner.worksheets[i].insert_column_outline_level(col_num, level);
        }
    }

    // Step 3.19: row page breaks — writer emits <rowBreaks>.
    let per_sheet_row_breaks = parse_sheet_row_breaks(data, sheet_count)?;
    for (i, breaks) in per_sheet_row_breaks.into_iter().enumerate() {
        for b in breaks {
            inner.worksheets[i].insert_row_break(b);
        }
    }

    // Step 3.20: column page breaks — writer emits <colBreaks>.
    let per_sheet_col_breaks = parse_sheet_col_breaks(data, sheet_count)?;
    for (i, breaks) in per_sheet_col_breaks.into_iter().enumerate() {
        for b in breaks {
            inner.worksheets[i].insert_col_break(b);
        }
    }

    // Step 4: parse defined names from xl/workbook.xml
    let defined_names = workbook::parse_defined_names(data, &sheet_names)?;
    inner.set_defined_names(defined_names);

    // Step 4.5: resolve _xlnm.Print_Area / _xlnm.Print_Titles into page setup (v1.0.0)
    for dn in inner.defined_names() {
        let field = match dn.name.as_str() {
            "_xlnm.Print_Area" => "area",
            "_xlnm.Print_Titles" => "titles",
            _ => continue,
        };
        let range = match dn.value.split_once('!') {
            Some((_, r)) => r.to_string(),
            None => dn.value.clone(),
        };
        let sheet_name = dn.sheet.clone().unwrap_or_default();
        if let Some(idx) = inner.worksheets.iter().position(|ws| ws.name() == sheet_name) {
            let mut ps = inner.worksheets[idx].get_page_setup_inner().unwrap_or_default();
            match field {
                "area" => ps.print_area = Some(range),
                "titles" => ps.print_titles = Some(range),
                _ => {}
            }
            inner.worksheets[idx].set_page_setup_inner(Some(ps));
        }
    }

    // Step 4.6: parse workbook views & calc properties and attach (v1.0.0)
    let (views, calc) = parse_workbook_views_calc(data)?;
    inner.set_views(views);
    inner.set_calc_properties(calc);

    Ok(inner)
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

/// Parse data validations from `xl/worksheets/sheet{N}.xml` files.
/// Returns a Vec of Vec where each inner Vec corresponds to a sheet's data validations.
fn parse_sheet_data_validations(
    data: &[u8],
    sheet_count: usize,
) -> Result<Vec<Vec<crate::model::data_validation::DataValidation>>, ExcelrsError> {
    use std::io::Cursor;
    let cursor = Cursor::new(data);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|e| ExcelrsError::Zip(e.to_string()))?;

    let mut all_dv: Vec<Vec<crate::model::data_validation::DataValidation>> = Vec::with_capacity(sheet_count);

    for i in 0..sheet_count {
        let path = format!("xl/worksheets/sheet{}.xml", i + 1);
        let dv = match archive.by_name(&path) {
            Ok(entry) => {
                let mut xml = String::new();
                entry.take(MAX_ENTRY_BYTES).read_to_string(&mut xml)?;
                parse_datavalidations_from_xml(&xml)?
            }
            Err(_) => Vec::new(),
        };
        all_dv.push(dv);
    }

    Ok(all_dv)
}

/// Parse conditional formatting from `xl/worksheets/sheet{N}.xml` files.
/// Returns a Vec of Vec where each inner Vec corresponds to a sheet's conditional formats.
fn parse_sheet_conditional_formattings(
    data: &[u8],
    sheet_count: usize,
    dxfs: &[crate::model::style::Dxf],
) -> Result<Vec<Vec<crate::model::conditional_formatting::ConditionalFormat>>, ExcelrsError> {
    use std::io::Cursor;
    let cursor = Cursor::new(data);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|e| ExcelrsError::Zip(e.to_string()))?;

    let mut all_cf: Vec<Vec<crate::model::conditional_formatting::ConditionalFormat>> = Vec::with_capacity(sheet_count);

    for i in 0..sheet_count {
        let path = format!("xl/worksheets/sheet{}.xml", i + 1);
        let cfs = match archive.by_name(&path) {
            Ok(entry) => {
                let mut xml = String::new();
                entry.take(MAX_ENTRY_BYTES).read_to_string(&mut xml)?;
                parse_conditional_formatting_from_xml(&xml, dxfs)?
            }
            Err(_) => Vec::new(),
        };
        all_cf.push(cfs);
    }

    Ok(all_cf)
}

/// Parse `<conditionalFormatting>` elements from sheet XML into ConditionalFormat objects.
fn parse_conditional_formatting_from_xml(
    xml: &str,
    dxfs: &[crate::model::style::Dxf],
) -> Result<Vec<crate::model::conditional_formatting::ConditionalFormat>, ExcelrsError> {
    use crate::model::conditional_formatting::{CfColor, CfRule, Cfvo, ConditionalFormat};
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut result: Vec<ConditionalFormat> = Vec::new();
    let mut reader = Reader::from_str(xml);
    let mut current_cf: Option<ConditionalFormat> = None;
    let mut current_rule: Option<CfRule> = None;
    let mut in_colorscale = false;
    let mut in_databar = false;
    let mut in_formula = false;
    let mut formula_buf = String::new();

    let mut events: u64 = 0;
    loop {
        let mut buf = Vec::new();
        events += 1;
        if events > MAX_EVENTS as u64 {
            break;
        }
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                match e.name().as_ref() {
                    b"conditionalFormatting" => {
                        if let Some(cf) = current_cf.take() {
                            result.push(cf);
                        }
                        let mut cf = ConditionalFormat::default();
                        for attr in e.attributes().flatten() {
                            if attr.key.as_ref() == b"sqref" {
                                cf.sqref = String::from_utf8_lossy(&attr.value).into_owned();
                            }
                        }
                        current_cf = Some(cf);
                    }
                    b"cfRule" => {
                        let mut rule = CfRule::default();
                        for attr in e.attributes().flatten() {
                            let key = String::from_utf8_lossy(attr.key.as_ref()).into_owned();
                            let val = String::from_utf8_lossy(&attr.value).into_owned();
                            match key.as_ref() {
                                "type" => rule.r#type = val,
                                "operator" => rule.operator = Some(val),
                                "priority" => rule.priority = val.parse().unwrap_or(0),
                                "dxfId" => rule.dxf_id = Some(val.parse().unwrap_or(0)),
                                "timePeriod" => rule.time_period = Some(val),
                                "rank" => rule.rank = Some(val.parse().unwrap_or(0)),
                                "percent" => rule.percent = Some(parse_ooxml_bool(&val)),
                                "bottom" => rule.bottom = Some(parse_ooxml_bool(&val)),
                                "text" => rule.text = Some(val),
                                "reverse" => rule.reverse = Some(parse_ooxml_bool(&val)),
                                "showValue" => rule.show_value = Some(parse_ooxml_bool(&val)),
                                _ => {}
                            }
                        }
                        // Resolve the differential style from dxfs if referenced.
                        if let Some(id) = rule.dxf_id {
                            if (id as usize) < dxfs.len() {
                                let d = &dxfs[id as usize];
                                rule.style = Some(crate::model::style::Style {
                                    font: d.font.clone(),
                                    fill: d.fill.clone(),
                                    border: d.border.clone(),
                                    num_fmt: d.num_fmt.clone(),
                                    ..Default::default()
                                });
                            }
                        }
                        current_rule = Some(rule);
                    }
                    b"formula" => {
                        in_formula = true;
                        formula_buf.clear();
                    }
                    b"colorScale" => in_colorscale = true,
                    b"dataBar" => in_databar = true,
                    b"iconSet" => {
                        if let Some(ref mut r) = current_rule {
                            for attr in e.attributes().flatten() {
                                let key = String::from_utf8_lossy(attr.key.as_ref()).into_owned();
                                let val = String::from_utf8_lossy(&attr.value).into_owned();
                                match key.as_ref() {
                                    "iconSet" => r.icon_set = Some(val),
                                    "reverse" => r.reverse = Some(parse_ooxml_bool(&val)),
                                    "showValue" => r.show_value = Some(parse_ooxml_bool(&val)),
                                    _ => {}
                                }
                            }
                        }
                    }
                    b"cfvo" => {
                        let mut cfvo = Cfvo::default();
                        for attr in e.attributes().flatten() {
                            let key = String::from_utf8_lossy(attr.key.as_ref()).into_owned();
                            let val = String::from_utf8_lossy(&attr.value).into_owned();
                            match key.as_ref() {
                                "type" => cfvo.r#type = val,
                                "val" => cfvo.value = Some(val),
                                _ => {}
                            }
                        }
                        if let Some(ref mut r) = current_rule {
                            r.cfvo.get_or_insert_with(Vec::new).push(cfvo);
                        }
                    }
                    b"color" => {
                        let mut c = CfColor::default();
                        for attr in e.attributes().flatten() {
                            let key = String::from_utf8_lossy(attr.key.as_ref()).into_owned();
                            let val = String::from_utf8_lossy(&attr.value).into_owned();
                            match key.as_ref() {
                                "rgb" => c.argb = Some(val),
                                "theme" => c.theme = Some(val.parse().unwrap_or(0)),
                                "indexed" => c.indexed = Some(val.parse().unwrap_or(0)),
                                "tint" => c.tint = Some(val.parse().unwrap_or(0.0)),
                                _ => {}
                            }
                        }
                        if let Some(ref mut r) = current_rule {
                            if in_colorscale {
                                r.color.get_or_insert_with(Vec::new).push(c);
                            } else if in_databar {
                                r.data_bar_color = Some(c);
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(ref e)) if in_formula => {
                formula_buf.push_str(&e.unescape().unwrap_or_default());
            }
            Ok(Event::End(ref e)) => {
                let tag = e.name().as_ref().to_vec();
                match &*tag {
                    b"formula" if in_formula => {
                        if let Some(ref mut r) = current_rule {
                            r.formula.get_or_insert_with(Vec::new).push(formula_buf.clone());
                        }
                        in_formula = false;
                    }
                    b"colorScale" => in_colorscale = false,
                    b"dataBar" => in_databar = false,
                    b"cfRule" => {
                        if let Some(mut r) = current_rule.take() {
                            // containsText-family rules carry the text inside the
                            // formula (quoted); prefer the `text` attribute, else
                            // strip the quotes from the formula.
                            if r.text.is_none()
                                && matches!(
                                    r.operator.as_deref(),
                                    Some("containsText")
                                        | Some("beginsWith")
                                        | Some("endsWith")
                                        | Some("notContainsText")
                                )
                            {
                                if let Some(formulas) = &r.formula {
                                    if let Some(f) = formulas.first() {
                                        r.text = Some(f.trim_matches('"').to_string());
                                    }
                                }
                            }
                            if let Some(ref mut cf) = current_cf {
                                cf.rules.push(r);
                            }
                        }
                    }
                    b"conditionalFormatting" => {
                        if let Some(cf) = current_cf.take() {
                            result.push(cf);
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }

    Ok(result)
}

/// Parse an OOXML boolean attribute. Accepts "1", "0", "true", "false" (case-insensitive).
fn parse_ooxml_bool(val: &str) -> bool {
    matches!(val.to_lowercase().as_str(), "1" | "true")
}

/// Parse <dataValidations> elements from sheet XML and return DataValidation objects.
fn parse_datavalidations_from_xml(
    xml: &str,
) -> Result<Vec<crate::model::data_validation::DataValidation>, ExcelrsError> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut validations = Vec::new();
    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();
    let mut in_dv = false;
    let mut current_dv: Option<crate::model::data_validation::DataValidation> = None;
    let mut formula_buf = String::new();
    let mut in_formula1 = false;
    let mut in_formula2 = false;

    let mut events: u64 = 0;
    loop {
        buf.clear();
        events += 1;
        if events > MAX_EVENTS as u64 {
            break;
        }
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                match e.name().as_ref() {
                    b"dataValidation" => {
                        in_dv = true;
                        let mut dv = crate::model::data_validation::DataValidation {
                            sqref: String::new(),
                            r#type: String::new(),
                            operator: None,
                            formula1: String::new(),
                            formula2: None,
                            allow_blank: None,
                            show_input_message: None,
                            show_error_message: None,
                            prompt: None,
                            prompt_title: None,
                            error: None,
                            error_title: None,
                            error_style: None,
                        };

                        // Parse attributes from dataValidation element
                        for attr in e.attributes().flatten() {
                            let key = String::from_utf8_lossy(attr.key.as_ref());
                            let val = String::from_utf8_lossy(&attr.value);
                            match key.as_ref() {
                                "sqref" => dv.sqref = val.into_owned(),
                                "type" => dv.r#type = val.into_owned(),
                                "operator" => dv.operator = Some(val.into_owned()),
                                "allowBlank" => dv.allow_blank = Some(parse_ooxml_bool(&val)),
                                "showInputMessage" => dv.show_input_message = Some(parse_ooxml_bool(&val)),
                                "showErrorMessage" => dv.show_error_message = Some(parse_ooxml_bool(&val)),
                                "promptTitle" => dv.prompt_title = Some(val.into_owned()),
                                "prompt" => dv.prompt = Some(val.into_owned()),
                                "errorTitle" => dv.error_title = Some(val.into_owned()),
                                "error" => dv.error = Some(val.into_owned()),
                                "errorStyle" => dv.error_style = Some(val.into_owned()),
                                _ => {}
                            }
                        }
                        current_dv = Some(dv);
                    }
                    b"formula1" if in_dv => {
                        in_formula1 = true;
                        formula_buf.clear();
                    }
                    b"formula2" if in_dv => {
                        in_formula2 = true;
                        formula_buf.clear();
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(ref e)) if (in_formula1 || in_formula2) => {
                if let Ok(s) = e.unescape() {
                    formula_buf.push_str(&s);
                }
            }
            Ok(Event::CData(ref e)) if (in_formula1 || in_formula2) => {
                if let Ok(s) = std::str::from_utf8(e.as_ref()) {
                    formula_buf.push_str(s);
                }
            }
            Ok(Event::End(ref e)) => match e.name().as_ref() {
                b"dataValidation" => {
                    if let Some(dv) = current_dv.take() {
                        if dv.validate().is_ok() {
                            validations.push(dv);
                        }
                    }
                    in_dv = false;
                }
                b"formula1" if in_formula1 => {
                    if let Some(ref mut dv) = current_dv {
                        dv.formula1 = formula_buf.clone();
                    }
                    in_formula1 = false;
                }
                b"formula2" if in_formula2 => {
                    if let Some(ref mut dv) = current_dv {
                        dv.formula2 = Some(formula_buf.clone());
                    }
                    in_formula2 = false;
                }
                _ => {}
            },
            Ok(Event::Eof) => break,
            Err(e) => return Err(ExcelrsError::Parse(format!("XML parse error: {e}"))),
            _ => {}
        }
    }

    Ok(validations)
}

// ---------------------------------------------------------------------------
// Sheet auto-filter reader (v0.11.0)
// ---------------------------------------------------------------------------

fn parse_sheet_auto_filters(data: &[u8], sheet_count: usize) -> Result<Vec<Option<String>>, ExcelrsError> {
    use std::io::Cursor;
    let cursor = Cursor::new(data);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|e| ExcelrsError::Zip(e.to_string()))?;
    let mut per_sheet = Vec::with_capacity(sheet_count);

    for i in 0..sheet_count {
        let path = format!("xl/worksheets/sheet{}.xml", i + 1);
        let af = match archive.by_name(&path) {
            Ok(entry) => {
                let mut xml = String::new();
                entry.take(MAX_ENTRY_BYTES).read_to_string(&mut xml)?;
                parse_autofilter_from_xml(&xml)
            }
            Err(_) => None,
        };
        per_sheet.push(af);
    }

    Ok(per_sheet)
}

fn parse_autofilter_from_xml(xml: &str) -> Option<String> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();
    let mut events: u64 = 0;
    loop {
        buf.clear();
        events += 1;
        if events > MAX_EVENTS as u64 {
            break;
        }
        match reader.read_event_into(&mut buf) {
            Ok(Event::Empty(ref e)) | Ok(Event::Start(ref e)) if e.name().as_ref() == b"autoFilter" => {
                for attr in e.attributes().flatten() {
                    if attr.key.as_ref() == b"ref" {
                        return Some(String::from_utf8_lossy(&attr.value).into_owned());
                    }
                }
                return None;
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Sheet views reader (v0.11.0) — freeze/split panes
// ---------------------------------------------------------------------------

fn parse_sheet_views(
    data: &[u8],
    sheet_count: usize,
) -> Result<Vec<Vec<crate::model::sheet_view::SheetView>>, ExcelrsError> {
    use std::io::Cursor;
    let cursor = Cursor::new(data);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|e| ExcelrsError::Zip(e.to_string()))?;
    let mut per_sheet = Vec::with_capacity(sheet_count);

    for i in 0..sheet_count {
        let path = format!("xl/worksheets/sheet{}.xml", i + 1);
        let views = match archive.by_name(&path) {
            Ok(entry) => {
                let mut xml = String::new();
                entry.take(MAX_ENTRY_BYTES).read_to_string(&mut xml)?;
                parse_views_from_xml(&xml)
            }
            Err(_) => Vec::new(),
        };
        per_sheet.push(views);
    }

    Ok(per_sheet)
}

fn parse_views_from_xml(xml: &str) -> Vec<crate::model::sheet_view::SheetView> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();
    let mut views = Vec::new();
    let mut in_sheet_view = false;
    let mut current: Option<crate::model::sheet_view::SheetView> = None;

    let mut events: u64 = 0;
    loop {
        buf.clear();
        events += 1;
        if events > MAX_EVENTS as u64 {
            break;
        }
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"sheetView" => {
                in_sheet_view = true;
                let mut sv = crate::model::sheet_view::SheetView::default();
                for attr in e.attributes().flatten() {
                    match attr.key.as_ref() {
                        b"state" => sv.state = Some(String::from_utf8_lossy(&attr.value).into_owned()),
                        b"topLeftCell" => sv.top_left_cell = Some(String::from_utf8_lossy(&attr.value).into_owned()),
                        _ => {}
                    }
                }
                current = Some(sv);
            }
            Ok(Event::Empty(ref e)) if e.name().as_ref() == b"pane" && in_sheet_view => {
                if let Some(ref mut sv) = current {
                    for attr in e.attributes().flatten() {
                        match attr.key.as_ref() {
                            b"xSplit" => {
                                sv.x_split = Some(std::str::from_utf8(&attr.value).unwrap_or("0").parse().unwrap_or(0))
                            }
                            b"ySplit" => {
                                sv.y_split = Some(std::str::from_utf8(&attr.value).unwrap_or("0").parse().unwrap_or(0))
                            }
                            b"topLeftCell" => {
                                sv.top_left_cell = Some(String::from_utf8_lossy(&attr.value).into_owned())
                            }
                            b"activePane" => sv.active_pane = Some(String::from_utf8_lossy(&attr.value).into_owned()),
                            _ => {}
                        }
                    }
                }
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == b"sheetView" => {
                in_sheet_view = false;
                if let Some(sv) = current.take() {
                    views.push(sv);
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }
    views
}

// ---------------------------------------------------------------------------
// Sheet protection reader (v0.11.0)
// ---------------------------------------------------------------------------

fn parse_boolean_flag(val: &[u8]) -> Option<bool> {
    let s = String::from_utf8_lossy(val);
    match s.to_lowercase().as_str() {
        "1" | "true" => Some(true),
        "0" | "false" => Some(false),
        _ => None,
    }
}

fn parse_sheet_protection(
    data: &[u8],
    sheet_count: usize,
) -> Result<Vec<Option<crate::model::sheet_protection::SheetProtection>>, ExcelrsError> {
    use std::io::Cursor;
    let cursor = Cursor::new(data);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|e| ExcelrsError::Zip(e.to_string()))?;
    let mut per_sheet = Vec::with_capacity(sheet_count);

    for i in 0..sheet_count {
        let path = format!("xl/worksheets/sheet{}.xml", i + 1);
        let prot = match archive.by_name(&path) {
            Ok(entry) => {
                let mut xml = String::new();
                entry.take(MAX_ENTRY_BYTES).read_to_string(&mut xml)?;
                parse_sheet_protection_from_xml(&xml)
            }
            Err(_) => None,
        };
        per_sheet.push(prot);
    }

    Ok(per_sheet)
}

fn parse_sheet_protection_from_xml(xml: &str) -> Option<crate::model::sheet_protection::SheetProtection> {
    use quick_xml::escape::unescape;
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();

    let mut events: u64 = 0;
    loop {
        buf.clear();
        events += 1;
        if events > MAX_EVENTS as u64 {
            break;
        }
        match reader.read_event_into(&mut buf) {
            Ok(Event::Empty(ref e)) | Ok(Event::Start(ref e)) if e.name().as_ref() == b"sheetProtection" => {
                let mut sp = crate::model::sheet_protection::SheetProtection::default();
                for attr in e.attributes().flatten() {
                    match attr.key.as_ref() {
                        b"locked" => sp.locked = parse_boolean_flag(&attr.value),
                        b"autoFilter" => sp.auto_filter = parse_boolean_flag(&attr.value),
                        b"deleteColumns" => sp.delete_columns = parse_boolean_flag(&attr.value),
                        b"deleteRows" => sp.delete_rows = parse_boolean_flag(&attr.value),
                        b"formatCells" => sp.format_cells = parse_boolean_flag(&attr.value),
                        b"formatColumns" => sp.format_columns = parse_boolean_flag(&attr.value),
                        b"formatRows" => sp.format_rows = parse_boolean_flag(&attr.value),
                        b"insertColumns" => sp.insert_columns = parse_boolean_flag(&attr.value),
                        b"insertHyperlinks" => sp.insert_hyperlinks = parse_boolean_flag(&attr.value),
                        b"insertRows" => sp.insert_rows = parse_boolean_flag(&attr.value),
                        b"pivotTables" => sp.pivot_tables = parse_boolean_flag(&attr.value),
                        b"selectLockedCells" => sp.select_locked_cells = parse_boolean_flag(&attr.value),
                        b"selectUnlockedCells" => sp.select_unlocked_cells = parse_boolean_flag(&attr.value),
                        b"sort" => sp.sort = parse_boolean_flag(&attr.value),
                        b"passwordHash" => {
                            let raw = String::from_utf8_lossy(&attr.value).to_string();
                            sp.password_hash = Some(unescape(&raw).map(|c| c.into_owned()).unwrap_or(raw));
                        }
                        b"saltValue" => {
                            let raw = String::from_utf8_lossy(&attr.value).to_string();
                            sp.salt_value = Some(unescape(&raw).map(|c| c.into_owned()).unwrap_or(raw));
                        }
                        _ => {}
                    }
                }
                return Some(sp);
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Header/footer reader (v1.0.0)
// ---------------------------------------------------------------------------

fn parse_sheet_header_footers(data: &[u8], sheet_count: usize) -> Result<Vec<Option<HeaderFooter>>, ExcelrsError> {
    use std::io::Cursor;
    let cursor = Cursor::new(data);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|e| ExcelrsError::Zip(e.to_string()))?;
    let mut per_sheet = Vec::with_capacity(sheet_count);

    for i in 0..sheet_count {
        let path = format!("xl/worksheets/sheet{}.xml", i + 1);
        let hf = match archive.by_name(&path) {
            Ok(entry) => {
                let mut xml = String::new();
                entry.take(MAX_ENTRY_BYTES).read_to_string(&mut xml)?;
                parse_header_footer_from_xml(&xml)
            }
            Err(_) => None,
        };
        per_sheet.push(hf);
    }

    Ok(per_sheet)
}

fn is_hf_child(name: &[u8]) -> bool {
    matches!(
        name,
        b"oddHeader" | b"oddFooter" | b"evenHeader" | b"evenFooter" | b"firstHeader" | b"firstFooter"
    )
}

fn set_hf_child(hf: &mut HeaderFooter, field: &str, value: String) {
    match field {
        "oddHeader" => hf.odd_header = Some(value),
        "oddFooter" => hf.odd_footer = Some(value),
        "evenHeader" => hf.even_header = Some(value),
        "evenFooter" => hf.even_footer = Some(value),
        "firstHeader" => hf.first_header = Some(value),
        "firstFooter" => hf.first_footer = Some(value),
        _ => {}
    }
}

fn parse_header_footer_from_xml(xml: &str) -> Option<HeaderFooter> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();
    let mut hf = HeaderFooter::default();
    let mut in_hf = false;
    let mut text: Option<String> = None;
    let mut current: Option<String> = None;

    let mut events: u64 = 0;
    loop {
        buf.clear();
        events += 1;
        if events > MAX_EVENTS as u64 {
            break;
        }
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == b"headerFooter" => {
                in_hf = true;
                for attr in e.attributes().flatten() {
                    match attr.key.as_ref() {
                        b"alignWithMargins" => hf.align_with_margins = parse_boolean_flag(&attr.value),
                        b"differentFirst" => hf.different_first = parse_boolean_flag(&attr.value),
                        b"differentOddEven" => hf.different_odd_even = parse_boolean_flag(&attr.value),
                        _ => {}
                    }
                }
            }
            Ok(Event::Start(ref e)) if in_hf && is_hf_child(e.name().as_ref()) => {
                current = Some(String::from_utf8_lossy(e.name().as_ref()).into_owned());
            }
            Ok(Event::Text(ref e)) if in_hf && current.is_some() => {
                let t = e.unescape().map(|c| c.into_owned()).unwrap_or_default();
                text = Some(t);
            }
            Ok(Event::End(ref e)) if in_hf && is_hf_child(e.name().as_ref()) => {
                if let (Some(field), Some(value)) = (current.take(), text.take()) {
                    set_hf_child(&mut hf, &field, value);
                }
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == b"headerFooter" => {
                in_hf = false;
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }

    if hf.odd_header.is_none()
        && hf.odd_footer.is_none()
        && hf.even_header.is_none()
        && hf.even_footer.is_none()
        && hf.first_header.is_none()
        && hf.first_footer.is_none()
    {
        None
    } else {
        Some(hf)
    }
}

/// Parse merged cell ranges from every worksheet part. Returns one vector
/// per sheet (index aligned with `sheet_count`), each holding the raw `ref`
/// strings from `<mergeCell ref="A1:C3"/>`.
fn parse_sheet_merge_cells(data: &[u8], sheet_count: usize) -> Result<Vec<Vec<String>>, ExcelrsError> {
    use std::io::{Cursor, Read};
    let cursor = Cursor::new(data);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|e| ExcelrsError::Zip(e.to_string()))?;
    let mut all: Vec<Vec<String>> = Vec::with_capacity(sheet_count);
    for i in 0..sheet_count {
        let path = format!("xl/worksheets/sheet{}.xml", i + 1);
        let ranges = match archive.by_name(&path) {
            Ok(entry) => {
                let mut xml = String::new();
                entry.take(MAX_ENTRY_BYTES).read_to_string(&mut xml)?;
                parse_merge_cells_from_xml(&xml)
            }
            Err(_) => Vec::new(),
        };
        all.push(ranges);
    }
    Ok(all)
}

/// Extract `<mergeCell ref="…"/>` entries from a worksheet XML blob.
fn parse_merge_cells_from_xml(xml: &str) -> Vec<String> {
    use quick_xml::events::Event;
    use quick_xml::Reader;
    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();
    let mut result: Vec<String> = Vec::new();
    let mut events: u64 = 0;
    loop {
        buf.clear();
        events += 1;
        if events > MAX_EVENTS as u64 {
            break;
        }
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) if e.local_name().as_ref() == b"mergeCell" => {
                for attr in e.attributes().flatten() {
                    if attr.key.as_ref() == b"ref" {
                        result.push(String::from_utf8_lossy(&attr.value).into_owned());
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }
    result
}

/// Parse row-level styles from every worksheet part. Returns one vector per
/// sheet (index aligned with `sheet_count`), each holding `(row_number, Style)`
/// pairs resolved from the `<row r="N" s="M">` attribute via `style_table`.
fn parse_sheet_row_styles(
    data: &[u8],
    sheet_count: usize,
    style_table: &StyleTableRead,
) -> Result<Vec<Vec<(u32, Style)>>, ExcelrsError> {
    use std::io::{Cursor, Read};
    let cursor = Cursor::new(data);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|e| ExcelrsError::Zip(e.to_string()))?;
    let mut all: Vec<Vec<(u32, Style)>> = Vec::with_capacity(sheet_count);
    for i in 0..sheet_count {
        let path = format!("xl/worksheets/sheet{}.xml", i + 1);
        let styles = match archive.by_name(&path) {
            Ok(entry) => {
                let mut xml = String::new();
                entry.take(MAX_ENTRY_BYTES).read_to_string(&mut xml)?;
                parse_row_styles_from_xml(&xml, style_table)
            }
            Err(_) => Vec::new(),
        };
        all.push(styles);
    }
    Ok(all)
}

/// Extract `<row r="N" s="M">` style indices from a worksheet XML blob and
/// resolve each to a `Style` through the shared read-side style table.
fn parse_row_styles_from_xml(xml: &str, style_table: &StyleTableRead) -> Vec<(u32, Style)> {
    use quick_xml::events::Event;
    use quick_xml::Reader;
    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();
    let mut result: Vec<(u32, Style)> = Vec::new();
    let mut events: u64 = 0;
    loop {
        buf.clear();
        events += 1;
        if events > MAX_EVENTS as u64 {
            break;
        }
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) if e.name().as_ref() == b"row" => {
                let mut row_num: Option<u32> = None;
                let mut xf_idx: Option<u32> = None;
                for attr in e.attributes().flatten() {
                    let key = attr.key.as_ref();
                    let val = String::from_utf8_lossy(&attr.value);
                    if key == b"r" {
                        row_num = val.trim().parse().ok();
                    } else if key == b"s" {
                        xf_idx = val.trim().parse().ok();
                    }
                }
                if let (Some(r), Some(s)) = (row_num, xf_idx) {
                    if let Some(style) = style_table.resolve_style(s) {
                        result.push((r, style));
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }
    result
}

/// Parse row outline levels from every worksheet part. Returns one vector per
/// sheet (index aligned with `sheet_count`), each holding `(row_number, level)`
/// pairs from the `<row r="N" outlineLevel="M">` attribute.
fn parse_sheet_row_outline_levels(data: &[u8], sheet_count: usize) -> Result<Vec<Vec<(u32, u8)>>, ExcelrsError> {
    use std::io::{Cursor, Read};
    let cursor = Cursor::new(data);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|e| ExcelrsError::Zip(e.to_string()))?;
    let mut all: Vec<Vec<(u32, u8)>> = Vec::with_capacity(sheet_count);
    for i in 0..sheet_count {
        let path = format!("xl/worksheets/sheet{}.xml", i + 1);
        let levels = match archive.by_name(&path) {
            Ok(entry) => {
                let mut xml = String::new();
                entry.take(MAX_ENTRY_BYTES).read_to_string(&mut xml)?;
                parse_row_outline_levels_from_xml(&xml)
            }
            Err(_) => Vec::new(),
        };
        all.push(levels);
    }
    Ok(all)
}

/// Extract `<row r="N" outlineLevel="M">` pairs from a worksheet XML blob.
fn parse_row_outline_levels_from_xml(xml: &str) -> Vec<(u32, u8)> {
    use quick_xml::events::Event;
    use quick_xml::Reader;
    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();
    let mut result: Vec<(u32, u8)> = Vec::new();
    let mut events: u64 = 0;
    loop {
        buf.clear();
        events += 1;
        if events > MAX_EVENTS as u64 {
            break;
        }
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) if e.name().as_ref() == b"row" => {
                let mut row_num: Option<u32> = None;
                let mut level: Option<u8> = None;
                for attr in e.attributes().flatten() {
                    let key = attr.key.as_ref();
                    let val = String::from_utf8_lossy(&attr.value);
                    if key == b"r" {
                        row_num = val.trim().parse().ok();
                    } else if key == b"outlineLevel" {
                        level = val.trim().parse().ok();
                    }
                }
                if let (Some(r), Some(l)) = (row_num, level) {
                    result.push((r, l.min(7)));
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }
    result
}

/// Parse column outline levels from every worksheet part. Returns one vector
/// per sheet (index aligned with `sheet_count`), each holding `(col_number, level)`
/// pairs from `<cols><col min="N" max="N" outlineLevel="M"/>`.
fn parse_sheet_col_outline_levels(data: &[u8], sheet_count: usize) -> Result<Vec<Vec<(u32, u8)>>, ExcelrsError> {
    use std::io::{Cursor, Read};
    let cursor = Cursor::new(data);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|e| ExcelrsError::Zip(e.to_string()))?;
    let mut all: Vec<Vec<(u32, u8)>> = Vec::with_capacity(sheet_count);
    for i in 0..sheet_count {
        let path = format!("xl/worksheets/sheet{}.xml", i + 1);
        let levels = match archive.by_name(&path) {
            Ok(entry) => {
                let mut xml = String::new();
                entry.take(MAX_ENTRY_BYTES).read_to_string(&mut xml)?;
                parse_col_outline_levels_from_xml(&xml)
            }
            Err(_) => Vec::new(),
        };
        all.push(levels);
    }
    Ok(all)
}

/// Extract column outline levels from a worksheet XML blob. A `<col>` may span
/// `min`..`max`; the level is applied to every column in that range.
fn parse_col_outline_levels_from_xml(xml: &str) -> Vec<(u32, u8)> {
    use quick_xml::events::Event;
    use quick_xml::Reader;
    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();
    let mut result: Vec<(u32, u8)> = Vec::new();
    let mut events: u64 = 0;
    loop {
        buf.clear();
        events += 1;
        if events > MAX_EVENTS as u64 {
            break;
        }
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) if e.name().as_ref() == b"col" => {
                let mut min: Option<u32> = None;
                let mut max: Option<u32> = None;
                let mut level: Option<u8> = None;
                for attr in e.attributes().flatten() {
                    let key = attr.key.as_ref();
                    let val = String::from_utf8_lossy(&attr.value);
                    if key == b"min" {
                        min = val.trim().parse().ok();
                    } else if key == b"max" {
                        max = val.trim().parse().ok();
                    } else if key == b"outlineLevel" {
                        level = val.trim().parse().ok();
                    }
                }
                if let (Some(lo), Some(hi), Some(l)) = (min, max, level) {
                    for c in lo..=hi.min(16384) {
                        result.push((c, l.min(7)));
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }
    result
}

/// Parse row page breaks from every worksheet part. Returns one vector per
/// sheet (index aligned with `sheet_count`) of 1-indexed row numbers.
fn parse_sheet_row_breaks(data: &[u8], sheet_count: usize) -> Result<Vec<Vec<u32>>, ExcelrsError> {
    use std::io::{Cursor, Read};
    let cursor = Cursor::new(data);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|e| ExcelrsError::Zip(e.to_string()))?;
    let mut all: Vec<Vec<u32>> = Vec::with_capacity(sheet_count);
    for i in 0..sheet_count {
        let path = format!("xl/worksheets/sheet{}.xml", i + 1);
        let breaks = match archive.by_name(&path) {
            Ok(entry) => {
                let mut xml = String::new();
                entry.take(MAX_ENTRY_BYTES).read_to_string(&mut xml)?;
                parse_breaks_from_xml(&xml, b"rowBreaks")
            }
            Err(_) => Vec::new(),
        };
        all.push(breaks);
    }
    Ok(all)
}

/// Parse column page breaks from every worksheet part.
fn parse_sheet_col_breaks(data: &[u8], sheet_count: usize) -> Result<Vec<Vec<u32>>, ExcelrsError> {
    use std::io::{Cursor, Read};
    let cursor = Cursor::new(data);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|e| ExcelrsError::Zip(e.to_string()))?;
    let mut all: Vec<Vec<u32>> = Vec::with_capacity(sheet_count);
    for i in 0..sheet_count {
        let path = format!("xl/worksheets/sheet{}.xml", i + 1);
        let breaks = match archive.by_name(&path) {
            Ok(entry) => {
                let mut xml = String::new();
                entry.take(MAX_ENTRY_BYTES).read_to_string(&mut xml)?;
                parse_breaks_from_xml(&xml, b"colBreaks")
            }
            Err(_) => Vec::new(),
        };
        all.push(breaks);
    }
    Ok(all)
}

/// Extract `<brk id="N"/>` ids from within the `<tag>` block of a worksheet XML blob.
fn parse_breaks_from_xml(xml: &str, tag: &[u8]) -> Vec<u32> {
    use quick_xml::events::Event;
    use quick_xml::Reader;
    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();
    let mut result: Vec<u32> = Vec::new();
    let mut in_block = false;
    let mut events: u64 = 0;
    loop {
        buf.clear();
        events += 1;
        if events > MAX_EVENTS as u64 {
            break;
        }
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == tag => {
                in_block = true;
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == tag => {
                in_block = false;
            }
            Ok(Event::Empty(ref e)) | Ok(Event::Start(ref e)) if in_block && e.name().as_ref() == b"brk" => {
                for attr in e.attributes().flatten() {
                    if attr.key.as_ref() == b"id" {
                        if let Ok(id) = String::from_utf8_lossy(&attr.value).trim().parse::<u32>() {
                            result.push(id);
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }
    result
}

// ---------------------------------------------------------------------------
// Page setup / print reader (v1.0.0)
// ---------------------------------------------------------------------------

fn parse_sheet_page_setups(data: &[u8], sheet_count: usize) -> Result<Vec<Option<PageSetup>>, ExcelrsError> {
    use std::io::Cursor;
    let cursor = Cursor::new(data);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|e| ExcelrsError::Zip(e.to_string()))?;
    let mut per_sheet = Vec::with_capacity(sheet_count);

    for i in 0..sheet_count {
        let path = format!("xl/worksheets/sheet{}.xml", i + 1);
        let ps = match archive.by_name(&path) {
            Ok(entry) => {
                let mut xml = String::new();
                entry.take(MAX_ENTRY_BYTES).read_to_string(&mut xml)?;
                parse_page_setup_from_xml(&xml)
            }
            Err(_) => None,
        };
        per_sheet.push(ps);
    }

    Ok(per_sheet)
}

fn num_attr<T: std::str::FromStr>(value: &[u8]) -> Option<T> {
    std::str::from_utf8(value).ok().and_then(|s| s.parse().ok())
}

fn parse_page_setup_from_xml(xml: &str) -> Option<PageSetup> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();
    let mut ps = PageSetup::default();
    let mut found = false;

    let mut events: u64 = 0;
    loop {
        buf.clear();
        events += 1;
        if events > MAX_EVENTS as u64 {
            break;
        }
        match reader.read_event_into(&mut buf) {
            Ok(Event::Empty(ref e)) | Ok(Event::Start(ref e)) if e.name().as_ref() == b"pageMargins" => {
                found = true;
                let mut m = PageMargins::default();
                for attr in e.attributes().flatten() {
                    match attr.key.as_ref() {
                        b"left" => m.left = num_attr(&attr.value),
                        b"right" => m.right = num_attr(&attr.value),
                        b"top" => m.top = num_attr(&attr.value),
                        b"bottom" => m.bottom = num_attr(&attr.value),
                        b"header" => m.header = num_attr(&attr.value),
                        b"footer" => m.footer = num_attr(&attr.value),
                        _ => {}
                    }
                }
                ps.margins = Some(m);
            }
            Ok(Event::Empty(ref e)) | Ok(Event::Start(ref e)) if e.name().as_ref() == b"pageSetup" => {
                found = true;
                for attr in e.attributes().flatten() {
                    match attr.key.as_ref() {
                        b"orientation" => ps.orientation = Some(String::from_utf8_lossy(&attr.value).into_owned()),
                        b"paperSize" => ps.paper_size = num_attr(&attr.value),
                        b"fitToPage" => ps.fit_to_page = parse_boolean_flag(&attr.value),
                        b"fitToWidth" => ps.fit_to_width = num_attr(&attr.value),
                        b"fitToHeight" => ps.fit_to_height = num_attr(&attr.value),
                        b"horizontalDpi" => ps.horizontal_dpi = num_attr(&attr.value),
                        b"verticalDpi" => ps.vertical_dpi = num_attr(&attr.value),
                        b"blackAndWhite" => ps.black_and_white = parse_boolean_flag(&attr.value),
                        b"drawingPrinted" => ps.drawing_printed = parse_boolean_flag(&attr.value),
                        b"cellComments" => ps.cell_comments = Some(String::from_utf8_lossy(&attr.value).into_owned()),
                        b"copies" => ps.copies = num_attr(&attr.value),
                        _ => {}
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }

    if found {
        Some(ps)
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Workbook views & calc properties reader (v1.0.0)
// ---------------------------------------------------------------------------

fn parse_workbook_views_calc(
    data: &[u8],
) -> Result<
    (
        Vec<crate::model::workbook_view::WorkbookView>,
        Option<crate::model::workbook_view::CalcProperties>,
    ),
    ExcelrsError,
> {
    use std::io::Cursor;
    let cursor = Cursor::new(data);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|e| ExcelrsError::Zip(e.to_string()))?;
    let mut xml = String::new();
    match archive.by_name("xl/workbook.xml") {
        Ok(entry) => {
            entry.take(MAX_ENTRY_BYTES).read_to_string(&mut xml)?;
        }
        Err(_) => return Ok((Vec::new(), None)),
    }
    Ok((parse_book_views_from_xml(&xml), parse_calc_pr_from_xml(&xml)))
}

fn parse_book_views_from_xml(xml: &str) -> Vec<crate::model::workbook_view::WorkbookView> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();
    let mut views = Vec::new();
    let mut events: u64 = 0;
    loop {
        buf.clear();
        events += 1;
        if events > MAX_EVENTS as u64 {
            break;
        }
        match reader.read_event_into(&mut buf) {
            Ok(Event::Empty(ref e)) | Ok(Event::Start(ref e)) if e.name().as_ref() == b"workbookView" => {
                let mut v = crate::model::workbook_view::WorkbookView::default();
                for attr in e.attributes().flatten() {
                    match attr.key.as_ref() {
                        b"xWindow" => v.x_window = num_attr(&attr.value),
                        b"yWindow" => v.y_window = num_attr(&attr.value),
                        b"windowWidth" => v.window_width = num_attr(&attr.value),
                        b"windowHeight" => v.window_height = num_attr(&attr.value),
                        b"activeTab" => v.active_tab = num_attr(&attr.value),
                        b"firstSheet" => v.first_sheet = num_attr(&attr.value),
                        b"minimized" => v.minimized = parse_boolean_flag(&attr.value),
                        b"showHorizontalScroll" => v.show_horizontal_scroll = parse_boolean_flag(&attr.value),
                        b"showVerticalScroll" => v.show_vertical_scroll = parse_boolean_flag(&attr.value),
                        b"tabRatio" => v.tab_ratio = num_attr(&attr.value),
                        b"visibility" => v.visibility = Some(String::from_utf8_lossy(&attr.value).into_owned()),
                        _ => {}
                    }
                }
                views.push(v);
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }
    views
}

fn parse_calc_pr_from_xml(xml: &str) -> Option<crate::model::workbook_view::CalcProperties> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();
    let mut found = false;
    let mut calc = crate::model::workbook_view::CalcProperties::default();
    let mut events: u64 = 0;
    loop {
        buf.clear();
        events += 1;
        if events > MAX_EVENTS as u64 {
            break;
        }
        match reader.read_event_into(&mut buf) {
            Ok(Event::Empty(ref e)) | Ok(Event::Start(ref e)) if e.name().as_ref() == b"calcPr" => {
                found = true;
                for attr in e.attributes().flatten() {
                    match attr.key.as_ref() {
                        b"fullCalcOnLoad" => calc.full_calc_on_load = parse_boolean_flag(&attr.value),
                        b"calcId" => calc.calc_id = num_attr(&attr.value),
                        b"calcMode" => calc.calc_mode = Some(String::from_utf8_lossy(&attr.value).into_owned()),
                        b"refFullCalc" => calc.ref_full_calc = parse_boolean_flag(&attr.value),
                        b"iterate" => calc.iterate = parse_boolean_flag(&attr.value),
                        b"iterateCount" => calc.iterate_count = num_attr(&attr.value),
                        b"iterateDelta" => calc.iterate_delta = num_attr(&attr.value),
                        _ => {}
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }
    if found {
        Some(calc)
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Hyperlinks reader (v0.11.0)
// ---------------------------------------------------------------------------

fn parse_sheet_hyperlinks(data: &[u8], sheet_count: usize) -> Result<Vec<Vec<(String, String)>>, ExcelrsError> {
    use std::io::Cursor;
    let cursor = Cursor::new(data);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|e| ExcelrsError::Zip(e.to_string()))?;
    let mut per_sheet = Vec::with_capacity(sheet_count);

    for i in 0..sheet_count {
        let sheet_num = i + 1;
        let path = format!("xl/worksheets/sheet{}.xml", sheet_num);
        let rels_path = format!("xl/worksheets/_rels/sheet{}.xml.rels", sheet_num);

        let rels = parse_sheet_rels(&mut archive, &rels_path);
        let links = match archive.by_name(&path) {
            Ok(entry) => {
                let mut xml = String::new();
                entry.take(MAX_ENTRY_BYTES).read_to_string(&mut xml)?;
                parse_hyperlinks_from_xml(&xml, &rels)
            }
            Err(_) => Vec::new(),
        };
        per_sheet.push(links);
    }

    Ok(per_sheet)
}

fn parse_sheet_rels(
    archive: &mut zip::ZipArchive<Cursor<&[u8]>>,
    path: &str,
) -> std::collections::HashMap<String, String> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut rels = std::collections::HashMap::new();
    let xml = match archive.by_name(path) {
        Ok(entry) => {
            let mut s = String::new();
            let _ = entry.take(MAX_ENTRY_BYTES).read_to_string(&mut s);
            s
        }
        Err(_) => return rels,
    };

    let mut reader = Reader::from_str(&xml);
    let mut buf = Vec::new();
    let mut events: u64 = 0;
    loop {
        buf.clear();
        events += 1;
        if events > MAX_EVENTS as u64 {
            break;
        }
        match reader.read_event_into(&mut buf) {
            Ok(Event::Empty(ref e)) | Ok(Event::Start(ref e)) if e.name().as_ref() == b"Relationship" => {
                let mut rid = String::new();
                let mut target = String::new();
                for attr in e.attributes().flatten() {
                    match attr.key.as_ref() {
                        b"Id" => rid = String::from_utf8_lossy(&attr.value).into_owned(),
                        b"Target" => target = String::from_utf8_lossy(&attr.value).into_owned(),
                        _ => {}
                    }
                }
                if !rid.is_empty() && !target.is_empty() {
                    rels.insert(rid, target);
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }
    rels
}

fn parse_hyperlinks_from_xml(xml: &str, rels: &std::collections::HashMap<String, String>) -> Vec<(String, String)> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut links = Vec::new();
    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();

    let mut events: u64 = 0;
    loop {
        buf.clear();
        events += 1;
        if events > MAX_EVENTS as u64 {
            break;
        }
        match reader.read_event_into(&mut buf) {
            Ok(Event::Empty(ref e)) | Ok(Event::Start(ref e)) if e.name().as_ref() == b"hyperlink" => {
                let mut cell_ref = String::new();
                let mut rid = String::new();
                for attr in e.attributes().flatten() {
                    match attr.key.as_ref() {
                        b"ref" => cell_ref = String::from_utf8_lossy(&attr.value).into_owned(),
                        b"r:id" | b"id" => rid = String::from_utf8_lossy(&attr.value).into_owned(),
                        _ => {}
                    }
                }
                if !cell_ref.is_empty() {
                    let url = rels.get(&rid).cloned();
                    if let Some(url) = url {
                        links.push((cell_ref, url));
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }
    links
}

/// Parse a cell reference like "A1" or "AB123" into (row, col) (1-based).
// ---------------------------------------------------------------------------
// (v1.0.0) Comments + Images parsing
// ---------------------------------------------------------------------------
/// Parse a sheet's `.rels` returning `(Id, Type, Target)` tuples so callers can
/// distinguish comments / drawing / hyperlink relationships by Type.
fn parse_sheet_rels_full(archive: &mut zip::ZipArchive<Cursor<&[u8]>>, path: &str) -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    let xml = match archive.by_name(path) {
        Ok(entry) => {
            let mut s = String::new();
            let _ = entry.take(MAX_ENTRY_BYTES).read_to_string(&mut s);
            s
        }
        Err(_) => return out,
    };

    let mut i = 0;
    while let Some(pos) = xml[i..].find("<Relationship ") {
        let start = i + pos;
        let tag_end = xml[start..]
            .find("/>")
            .map(|p| start + p + 2)
            .or_else(|| xml[start..].find('>').map(|p| start + p + 1));
        let tag = match tag_end {
            Some(e) => &xml[start..e],
            None => &xml[start..],
        };
        let rid = rel_attr(tag, "Id");
        let rtype = rel_attr(tag, "Type").unwrap_or_default();
        let target = rel_attr(tag, "Target");
        if let (Some(rid), Some(target)) = (rid, target) {
            out.push((rid, rtype, target));
        }
        i = tag_end.unwrap_or(xml.len());
    }
    out
}

/// Extract a double-quoted attribute value from a single XML tag string.
fn rel_attr(tag: &str, key: &str) -> Option<String> {
    let prefix = format!("{}=", key);
    let idx = tag.find(prefix.as_str())?;
    let rest = &tag[idx + prefix.len()..];
    let q1 = rest.find('"')?;
    let q2 = rest[q1 + 1..].find('"')?;
    // ponytail: writer escapes attribute values; invert so round-trips stay faithful
    let raw = &rest[q1 + 1..q1 + 1 + q2];
    let unescaped = quick_xml::escape::unescape(raw).unwrap_or(std::borrow::Cow::Borrowed(raw));
    Some(unescaped.into_owned())
}

/// Parse `xl/commentsN.xml` for every sheet, returning `(cellRef, CellComment)`
/// pairs per sheet.
fn parse_sheet_comments(data: &[u8], sheet_count: usize) -> Result<Vec<Vec<(String, CellComment)>>, ExcelrsError> {
    use std::io::Cursor;

    let cursor = Cursor::new(data);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|e| ExcelrsError::Zip(e.to_string()))?;
    let mut per_sheet = Vec::with_capacity(sheet_count);
    for i in 0..sheet_count {
        let sheet_num = i + 1;
        let rels_path = format!("xl/worksheets/_rels/sheet{}.xml.rels", sheet_num);
        let rels = parse_sheet_rels_full(&mut archive, &rels_path);
        let comments_target = rels
            .iter()
            .find(|(_, t, _)| t.ends_with("/comments"))
            .map(|(_, _, target)| target.clone());
        let mut comments: Vec<(String, CellComment)> = Vec::new();
        if let Some(target) = comments_target {
            let cpath = format!("xl/{}", target.trim_start_matches("../"));
            if let Ok(entry) = archive.by_name(&cpath) {
                let mut xml = String::new();
                entry.take(MAX_ENTRY_BYTES).read_to_string(&mut xml)?;
                comments = parse_comments_from_xml(&xml);
            }
        }
        per_sheet.push(comments);
    }
    Ok(per_sheet)
}

/// Extract the substring between the first `open` and the next `close` (exclusive).
fn between<'a>(s: &'a str, open: &str, close: &str) -> Option<&'a str> {
    let i = s.find(open)?;
    let j = s[i + open.len()..].find(close)?;
    Some(&s[i + open.len()..i + open.len() + j])
}

fn parse_comments_from_xml(xml: &str) -> Vec<(String, CellComment)> {
    use quick_xml::escape::unescape;

    let mut out: Vec<(String, CellComment)> = Vec::new();
    // Authors: <authors><author>Name</author>...</authors>
    let mut authors: Vec<String> = Vec::new();
    if let Some(body) = between(xml, "<authors>", "</authors>") {
        let mut rest = body;
        while let Some(a) = between(rest, "<author>", "</author>") {
            let author_name = unescape(a).unwrap_or_else(|_| a.into()).trim().to_string();
            authors.push(author_name);
            if let Some(p) = rest.find("</author>") {
                rest = &rest[p + 9..];
            } else {
                break;
            }
        }
    }
    // Comments: <comment ref="A1" authorId="0"><text><t>text</t></text></comment>
    let mut rest = xml;
    while let Some(start) = rest.find("<comment ") {
        let close = match rest[start..].find('>') {
            Some(p) => start + p,
            None => break,
        };
        let tag = &rest[start..=close];
        let r = rel_attr(tag, "ref");
        let aid = rel_attr(tag, "authorId").and_then(|v| v.parse::<u32>().ok());
        let after = &rest[close + 1..];
        let text = unescape(between(after, "<t>", "</t>").unwrap_or(""))
            .map(|c| c.into_owned())
            .unwrap_or_default();
        if let Some(r) = r {
            let author = aid
                .and_then(|a| authors.get(a as usize).cloned())
                .filter(|a| !a.is_empty());
            out.push((r, CellComment { text, author }));
        }
        if let Some(p) = rest[close..].find("</comment>") {
            rest = &rest[close + p + 9..];
        } else {
            break;
        }
    }
    out
}

/// Parse `xl/tables/tableN.xml` for every sheet, returning `Table` records per sheet (v1.1.0).
fn parse_sheet_tables(data: &[u8], sheet_count: usize) -> Result<Vec<Vec<Table>>, ExcelrsError> {
    use std::io::Cursor;
    let cursor = Cursor::new(data);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|e| ExcelrsError::Zip(e.to_string()))?;
    let mut per_sheet: Vec<Vec<Table>> = Vec::with_capacity(sheet_count);
    for i in 0..sheet_count {
        let sheet_num = i + 1;
        let rels_path = format!("xl/worksheets/_rels/sheet{}.xml.rels", sheet_num);
        let rels = parse_sheet_rels_full(&mut archive, &rels_path);
        let table_targets: Vec<String> = rels
            .iter()
            .filter(|(_, t, _)| t.ends_with("/table"))
            .map(|(_, _, target)| target.clone())
            .collect();
        let mut tables: Vec<Table> = Vec::new();
        for target in table_targets {
            let tpath = format!("xl/{}", target.trim_start_matches("../"));
            if let Ok(entry) = archive.by_name(&tpath) {
                let mut xml = String::new();
                entry.take(MAX_ENTRY_BYTES).read_to_string(&mut xml)?;
                if let Some(t) = parse_tables_from_xml(&xml) {
                    tables.push(t);
                }
            }
        }
        per_sheet.push(tables);
    }
    Ok(per_sheet)
}

/// Parse a single `xl/tables/tableN.xml` into a `Table` model (v1.1.0).
/// `rows` are reconstructed later from the worksheet cells (see `reconstruct_table_rows`).
fn parse_tables_from_xml(xml: &str) -> Option<Table> {
    let table_open = between(xml, "<table ", ">")?;
    let name = rel_attr(table_open, "name").unwrap_or_default();
    let display_name = rel_attr(table_open, "displayName").unwrap_or_else(|| name.clone());
    let ref_range = rel_attr(table_open, "ref").unwrap_or_default();
    let totals_row = rel_attr(table_open, "totalsRowShown").unwrap_or_else(|| "0".to_string()) == "1";
    let header_row = rel_attr(table_open, "headerRowCount").unwrap_or_else(|| "1".to_string()) != "0";
    let autofilter_ref = between(xml, "<autoFilter", "/>").and_then(|tag| rel_attr(tag, "ref"));
    let mut columns: Vec<TableColumn> = Vec::new();
    if let Some(body) = between(xml, "<tableColumns", "</tableColumns>") {
        let mut rest = body;
        while let Some(start) = rest.find("<tableColumn ") {
            let close = match rest[start..].find('>') {
                Some(p) => start + p,
                None => break,
            };
            let tag = &rest[start..=close];
            columns.push(TableColumn {
                name: rel_attr(tag, "name").unwrap_or_default(),
                totals_row_function: rel_attr(tag, "totalsRowFunction"),
                totals_row_label: rel_attr(tag, "totalsRowLabel"),
            });
            rest = &rest[close + 1..];
        }
    }
    let style = between(xml, "<tableStyleInfo ", "/>").map(|s| TableStyle {
        theme: rel_attr(s, "name"),
        show_first_column: Some(rel_attr(s, "showFirstColumn").unwrap_or_else(|| "0".to_string()) == "1"),
        show_last_column: Some(rel_attr(s, "showLastColumn").unwrap_or_else(|| "0".to_string()) == "1"),
        show_row_stripes: Some(rel_attr(s, "showRowStripes").unwrap_or_else(|| "0".to_string()) == "1"),
        show_column_stripes: Some(rel_attr(s, "showColumnStripes").unwrap_or_else(|| "0".to_string()) == "1"),
    });
    Some(Table {
        name,
        display_name,
        ref_range,
        header_row,
        totals_row,
        columns,
        rows: Vec::new(),
        style,
        autofilter_ref,
    })
}

/// Reconstruct a table's data rows from the worksheet's own cells (v1.1.0).
fn reconstruct_table_rows(ws: &Worksheet, table: &Table) -> Vec<TableRow> {
    let (a, b) = match table.ref_range.split_once(':') {
        Some(p) => p,
        None => return Vec::new(),
    };
    let (sr, sc) = ref_to_rowcol(a).unwrap_or((1, 1));
    let (er, ec) = ref_to_rowcol(b).unwrap_or((1, 1));
    let data_start = if table.header_row { sr + 1 } else { sr };
    let data_end = if table.totals_row { er - 1 } else { er };
    let mut rows = Vec::new();
    if data_end >= data_start {
        for r in data_start..=data_end {
            let mut values = Vec::new();
            for c in sc..=ec {
                values.push(ws.get_cell_by_rc(r, c).value_raw().clone());
            }
            rows.push(TableRow { values });
        }
    }
    rows
}

/// Parse `xl/drawings/drawingN.xml` + media for every sheet, returning
/// `WorksheetImage` records per sheet.
fn parse_sheet_images(data: &[u8], sheet_count: usize) -> Result<Vec<Vec<WorksheetImage>>, ExcelrsError> {
    use std::io::Cursor;

    let cursor = Cursor::new(data);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|e| ExcelrsError::Zip(e.to_string()))?;
    let mut per_sheet = Vec::with_capacity(sheet_count);
    for i in 0..sheet_count {
        let sheet_num = i + 1;
        let rels_path = format!("xl/worksheets/_rels/sheet{}.xml.rels", sheet_num);
        let rels = parse_sheet_rels_full(&mut archive, &rels_path);
        let drawing_target = rels
            .iter()
            .find(|(_, t, _)| t.ends_with("/drawing"))
            .map(|(_, _, target)| target.clone());
        let mut imgs: Vec<WorksheetImage> = Vec::new();
        if let Some(dtarget) = drawing_target {
            let dpath = format!("xl/{}", dtarget.trim_start_matches("../"));
            let xml = match archive.by_name(&dpath) {
                Ok(entry) => {
                    let mut s = String::new();
                    let _ = entry.take(MAX_ENTRY_BYTES).read_to_string(&mut s);
                    s
                }
                Err(_) => String::new(),
            };
            if !xml.is_empty() {
                let drel_path = {
                    let file = std::path::Path::new(&dpath).file_name().unwrap_or_default();
                    format!("xl/drawings/_rels/{}.rels", file.to_string_lossy())
                };
                let drels = parse_sheet_rels_full(&mut archive, &drel_path);
                let media_map: std::collections::HashMap<String, String> = drels
                    .iter()
                    .map(|(id, _, target)| (id.clone(), target.clone()))
                    .collect();
                for (rid, anchor) in parse_drawing_xml(&xml) {
                    if let Some(target) = media_map.get(&rid) {
                        let mpath = format!("xl/{}", target.trim_start_matches("../"));
                        if let Ok(mut me) = archive.by_name(&mpath) {
                            let mut buf = Vec::new();
                            me.read_to_end(&mut buf)?;
                            let ext = Path::new(&mpath)
                                .extension()
                                .and_then(|e| e.to_str())
                                .unwrap_or("bin")
                                .to_string();
                            imgs.push(WorksheetImage {
                                extension: ext,
                                buffer: buf,
                                positioning: "oneCell".to_string(),
                                anchor,
                                media_index: 0,
                            });
                        }
                    }
                }
            }
        }
        per_sheet.push(imgs);
    }
    Ok(per_sheet)
}

fn parse_drawing_xml(xml: &str) -> Vec<(String, ImageAnchor)> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();
    let mut out: Vec<(String, ImageAnchor)> = Vec::new();
    let mut cur = ImageAnchor {
        anchor_type: "oneCell".to_string(),
        col: 0,
        row: 0,
        x: 0,
        y: 0,
        col2: 0,
        row2: 0,
        x2: 0,
        y2: 0,
    };
    let mut in_from = false;
    let mut in_to = false;
    let mut embed_rid: Option<String> = None;
    let mut events: u64 = 0;
    loop {
        buf.clear();
        events += 1;
        if events > MAX_EVENTS as u64 {
            break;
        }
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => match e.name().as_ref() {
                b"xdr:oneCellAnchor" => {
                    cur = ImageAnchor {
                        anchor_type: "oneCell".to_string(),
                        col: 0,
                        row: 0,
                        x: 0,
                        y: 0,
                        col2: 0,
                        row2: 0,
                        x2: 0,
                        y2: 0,
                    };
                    embed_rid = None;
                }
                b"xdr:twoCellAnchor" => {
                    cur = ImageAnchor {
                        anchor_type: "twoCell".to_string(),
                        col: 0,
                        row: 0,
                        x: 0,
                        y: 0,
                        col2: 0,
                        row2: 0,
                        x2: 0,
                        y2: 0,
                    };
                    embed_rid = None;
                }
                b"xdr:from" => in_from = true,
                b"xdr:to" => in_to = true,
                b"xdr:col" => {
                    let v = read_next_text_u32(&mut reader);
                    if in_from {
                        cur.col = v;
                    } else if in_to {
                        cur.col2 = v;
                    }
                }
                b"xdr:row" => {
                    let v = read_next_text_u32(&mut reader);
                    if in_from {
                        cur.row = v;
                    } else if in_to {
                        cur.row2 = v;
                    }
                }
                b"xdr:colOff" => {
                    let v = read_next_text_u32(&mut reader);
                    if in_from {
                        cur.x = v;
                    } else if in_to {
                        cur.x2 = v;
                    }
                }
                b"xdr:rowOff" => {
                    let v = read_next_text_u32(&mut reader);
                    if in_from {
                        cur.y = v;
                    } else if in_to {
                        cur.y2 = v;
                    }
                }
                b"a:blip" => {
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"r:embed" {
                            embed_rid = Some(String::from_utf8_lossy(&attr.value).into_owned());
                        }
                    }
                }
                _ => {}
            },
            Ok(Event::End(ref e)) => match e.name().as_ref() {
                b"xdr:from" => in_from = false,
                b"xdr:to" => in_to = false,
                b"xdr:pic" => {
                    if let Some(rid) = embed_rid.take() {
                        out.push((rid, cur.clone()));
                    }
                }
                _ => {}
            },
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
    }
    out
}

fn read_next_text_u32(reader: &mut quick_xml::Reader<&[u8]>) -> u32 {
    use quick_xml::events::Event;
    let mut b = Vec::new();
    match reader.read_event_into(&mut b) {
        Ok(Event::Text(t)) => t
            .unescape()
            .ok()
            .and_then(|s| s.trim().parse::<u32>().ok())
            .unwrap_or(0),
        _ => 0,
    }
}

fn ref_to_rowcol(ref_: &str) -> Option<(u32, u32)> {
    let ref_ = ref_.trim();
    if ref_.is_empty() {
        return None;
    }
    let col_str: String = ref_.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
    let row_str: String = ref_.chars().skip_while(|c| c.is_ascii_alphabetic()).collect();
    if col_str.is_empty() || row_str.is_empty() {
        return None;
    }
    let row: u32 = row_str.parse().ok()?;
    let col: u32 = col_str
        .chars()
        .fold(0, |acc, c| acc * 26 + (c.to_ascii_uppercase() as u32 - 'A' as u32 + 1));
    Some((row, col))
}

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
            // v0.13.0: preserve as a `Date` (Excel serial), not an ISO string, so the
            // date survives read→write as a JS `Date`.
            CellValue::date(dt.as_f64())
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
// Rich-text inline string reader (v0.12.0)
// ---------------------------------------------------------------------------

/// Parse rich-text inline strings (`<c t="inlineStr"><is><r>...</r></is></c>`)
/// from each sheet's XML. Returns per-sheet lists of (row, col, runs).
// Not behind a Result: inline-str parsing is best-effort; failure on individual
// cells degrades to plain string (the already-parsed calamine string value).
fn parse_sheet_rich_text(data: &[u8], sheet_count: usize) -> Vec<Vec<(u32, u32, Vec<RichTextRun>)>> {
    use std::io::Cursor;
    let cursor = Cursor::new(data);
    let mut archive = match zip::ZipArchive::new(cursor) {
        Ok(a) => a,
        Err(_) => return vec![Vec::new(); sheet_count],
    };
    let mut per_sheet = Vec::with_capacity(sheet_count);
    for i in 0..sheet_count {
        let path = format!("xl/worksheets/sheet{}.xml", i + 1);
        let cells = match archive.by_name(&path) {
            Ok(entry) => {
                let mut xml = String::new();
                if entry.take(MAX_ENTRY_BYTES).read_to_string(&mut xml).is_ok() {
                    parse_inline_str_rich_text(&xml)
                } else {
                    Vec::new()
                }
            }
            Err(_) => Vec::new(),
        };
        per_sheet.push(cells);
    }
    per_sheet
}

/// Parse `<c t="inlineStr"><is><r>...</r></is></c>` elements from a sheet XML string.
fn parse_inline_str_rich_text(xml: &str) -> Vec<(u32, u32, Vec<RichTextRun>)> {
    parse_inline_str_rich_text_with(xml, MAX_EVENTS)
}

fn parse_inline_str_rich_text_with(xml: &str, max_events: usize) -> Vec<(u32, u32, Vec<RichTextRun>)> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();

    let mut result = Vec::new();
    // State machine
    let mut in_c = false;
    let mut in_is = false;
    let mut in_r = false;
    let mut in_rpr = false;
    let mut in_t = false;
    let mut cell_ref = String::new();
    let mut runs: Vec<RichTextRun> = Vec::new();
    let mut current_text = String::new();
    let mut current_font = Font::default();
    let mut has_rpr = false;
    let mut events: u64 = 0;

    loop {
        buf.clear();
        events += 1;
        if events > max_events as u64 {
            break;
        }
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => match e.name().as_ref() {
                b"c" => {
                    cell_ref.clear();
                    let mut is_inline_str = false;
                    for attr in e.attributes().flatten() {
                        match attr.key.as_ref() {
                            b"r" => {
                                cell_ref = String::from_utf8_lossy(&attr.value).into_owned();
                            }
                            b"t" if attr.value.as_ref() == b"inlineStr" => {
                                is_inline_str = true;
                            }
                            _ => {}
                        }
                    }
                    if is_inline_str {
                        in_c = true;
                        runs.clear();
                    }
                }
                b"is" if in_c => in_is = true,
                b"r" if in_is => {
                    in_r = true;
                    current_font = Font::default();
                    current_text.clear();
                    has_rpr = false;
                }
                b"rPr" if in_r => in_rpr = true,
                b"b" if in_rpr => {
                    current_font.bold = Some(true);
                    has_rpr = true;
                }
                b"i" if in_rpr => {
                    current_font.italic = Some(true);
                    has_rpr = true;
                }
                b"u" if in_rpr => {
                    current_font.underline = Some(true);
                    has_rpr = true;
                }
                b"sz" if in_rpr => {
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"val" {
                            current_font.size = String::from_utf8_lossy(&attr.value).parse::<f64>().ok();
                            has_rpr = true;
                        }
                    }
                }
                b"color" if in_rpr => {
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"rgb" {
                            current_font.color = Some(String::from_utf8_lossy(&attr.value).into_owned());
                            has_rpr = true;
                        }
                    }
                }
                b"rFont" if in_rpr => {
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"val" {
                            current_font.name = Some(String::from_utf8_lossy(&attr.value).into_owned());
                            has_rpr = true;
                        }
                    }
                }
                b"t" if in_r => in_t = true,
                _ => {}
            },
            Ok(Event::End(ref e)) => match e.name().as_ref() {
                b"t" if in_t => in_t = false,
                b"r" if in_r => {
                    if !current_text.is_empty() {
                        let font = if has_rpr { Some(current_font.clone()) } else { None };
                        runs.push(RichTextRun {
                            text: std::mem::take(&mut current_text),
                            font,
                        });
                    }
                    in_r = false;
                }
                b"rPr" if in_rpr => in_rpr = false,
                b"is" if in_is => in_is = false,
                b"c" if in_c => {
                    if !runs.is_empty() {
                        if let Some((row, col)) = ref_to_rowcol(&cell_ref) {
                            result.push((row, col, runs.clone()));
                        }
                    }
                    in_c = false;
                }
                _ => {}
            },
            Ok(Event::Text(ref e)) if in_t => {
                let text = e.unescape().unwrap_or_default().to_string();
                current_text.push_str(&text);
            }
            Ok(Event::Eof) => break,
            Err(_) => break, // best-effort: degrade to plain string
            _ => {}
        }
    }
    result
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
        assert_eq!(result.value_type, "Date");
        let serial = result.date_serial.expect("date serial");
        assert!((serial - 45943.541).abs() < 1e-9, "expected ~45943.541, got {serial}");
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

    // -- worksheet-structure (v1.3.0): Excel-authored grouping + breaks --

    #[test]
    fn test_read_excel_authored_grouping_and_breaks() {
        use std::io::Write;

        // Build a minimal but valid xlsx whose sheet XML is shaped the way
        // Excel emits it: <cols> grouping, <row outlineLevel>, <rowBreaks>,
        // <colBreaks>. This exercises the reader parse passes (Steps 3.17-3.20)
        // against real Excel-shaped markup rather than our own round-trip.
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
  <cols>
    <col min="1" max="2" outlineLevel="1"/>
  </cols>
  <sheetData>
    <row r="1" outlineLevel="2">
      <c r="A1" t="inlineStr"><is><t>hello</t></is></c>
    </row>
    <row r="2">
      <c r="A2" t="inlineStr"><is><t>world</t></is></c>
    </row>
  </sheetData>
  <rowBreaks count="1">
    <brk id="3" max="16383" man="0"/>
  </rowBreaks>
  <colBreaks count="1">
    <brk id="2" max="1048575" man="0"/>
  </colBreaks>
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

        let read = workbook_inner_from_bytes(&buf).unwrap();
        let ws = &read.worksheets()[0];

        // grouping preserved
        assert_eq!(ws.get_row(1).outline_level(), 2, "row 1 outline level");
        let cols = ws.columns();
        let col1 = cols.iter().find(|c| c.col_num() == 1);
        assert!(
            col1.is_some_and(|c| c.outline_level() == 1),
            "column 1 outline level should round-trip"
        );

        // breaks preserved
        assert_eq!(ws.row_breaks(), vec![3], "row breaks");
        assert_eq!(ws.col_breaks(), vec![2], "col breaks");

        // write-back preserves the structure (read -> write -> read)
        let bytes2 = crate::writer::xlsx::workbook_to_bytes(&read).unwrap();
        let read2 = workbook_inner_from_bytes(&bytes2).unwrap();
        let ws2 = &read2.worksheets()[0];
        assert_eq!(ws2.get_row(1).outline_level(), 2, "row 1 outline level (write-back)");
        assert_eq!(ws2.row_breaks(), vec![3], "row breaks (write-back)");
        assert_eq!(ws2.col_breaks(), vec![2], "col breaks (write-back)");
    }

    // -- data validation parse tests --

    #[test]
    fn test_parse_datavalidation_boolean_values() {
        fn dv_xml(attrs: &str) -> String {
            format!(
                r##"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><dataValidations count="1"><dataValidation sqref="A1" type="whole" {}><formula1>1</formula1></dataValidation></dataValidations></worksheet>"##,
                attrs
            )
        }

        let xml = dv_xml(r##"allowBlank="true" showInputMessage="false""##);
        let dvs = parse_datavalidations_from_xml(&xml).unwrap();
        assert_eq!(dvs.len(), 1);
        assert_eq!(dvs[0].allow_blank, Some(true));
        assert_eq!(dvs[0].show_input_message, Some(false));

        let xml = dv_xml(r##"allowBlank="false""##);
        let dvs = parse_datavalidations_from_xml(&xml).unwrap();
        assert_eq!(dvs[0].allow_blank, Some(false));

        let xml = dv_xml(r##"allowBlank="1""##);
        let dvs = parse_datavalidations_from_xml(&xml).unwrap();
        assert_eq!(dvs[0].allow_blank, Some(true));

        let xml = dv_xml(r##"allowBlank="0""##);
        let dvs = parse_datavalidations_from_xml(&xml).unwrap();
        assert_eq!(dvs[0].allow_blank, Some(false));

        let xml = dv_xml(r##"allowBlank="TRUE""##);
        let dvs = parse_datavalidations_from_xml(&xml).unwrap();
        assert_eq!(dvs[0].allow_blank, Some(true));

        let xml = dv_xml(r##"showInputMessage="true""##);
        let dvs = parse_datavalidations_from_xml(&xml).unwrap();
        assert_eq!(dvs[0].show_input_message, Some(true));
    }

    #[test]
    fn test_parse_datavalidation_cdata_formula() {
        let xml = concat!(
            r##"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">"##,
            r##"<dataValidations count="1"><dataValidation sqref="A1" type="custom">"##,
            r##"<formula1><![CDATA[=A1>B1]]></formula1>"##,
            r##"</dataValidation></dataValidations></worksheet>"##,
        );
        let dvs = parse_datavalidations_from_xml(xml).unwrap();
        assert_eq!(dvs.len(), 1);
        assert_eq!(dvs[0].formula1, "=A1>B1");

        let xml2 = concat!(
            r##"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">"##,
            r##"<dataValidations count="1"><dataValidation sqref="A1" type="whole">"##,
            r##"<formula1>SUM(A1)</formula1>"##,
            r##"</dataValidation></dataValidations></worksheet>"##,
        );
        let dvs = parse_datavalidations_from_xml(xml2).unwrap();
        assert_eq!(dvs[0].formula1, "SUM(A1)");
    }

    #[test]
    fn test_parse_datavalidation_malformed_type() {
        let xml = concat!(
            r##"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">"##,
            r##"<dataValidations count="1"><dataValidation sqref="A1" type="bogus">"##,
            r##"<formula1>1</formula1>"##,
            r##"</dataValidation></dataValidations></worksheet>"##,
        );
        let dvs = parse_datavalidations_from_xml(xml).unwrap();
        assert!(dvs.is_empty(), "bogus type should be skipped");

        let xml2 = concat!(
            r##"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">"##,
            r##"<dataValidations count="1"><dataValidation sqref="A1" type="whole">"##,
            r##"<formula1>1</formula1>"##,
            r##"</dataValidation></dataValidations></worksheet>"##,
        );
        let dvs = parse_datavalidations_from_xml(xml2).unwrap();
        assert_eq!(dvs.len(), 1);
    }

    // -----------------------------------------------------------------------
    // autoFilter parser tests (v0.11.0)
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_autofilter_found() {
        let xml = r##"<worksheet><autoFilter ref="A1:C10"/></worksheet>"##;
        let result = parse_autofilter_from_xml(xml);
        assert_eq!(result.as_deref(), Some("A1:C10"));
    }

    #[test]
    fn test_parse_autofilter_absent() {
        let xml = r##"<worksheet><sheetData></sheetData></worksheet>"##;
        let result = parse_autofilter_from_xml(xml);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_autofilter_start_with_children() {
        let xml = r##"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <autoFilter ref="A1:C10"><filterColumn colId="0"><filters><filter val="x"/></filters></filterColumn></autoFilter>
</worksheet>"##;
        let result = parse_autofilter_from_xml(xml);
        assert_eq!(result, Some("A1:C10".to_string()));
    }

    // -----------------------------------------------------------------------
    // SheetView parser tests (v0.11.0)
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_views_frozen_pane() {
        let xml = r##"<worksheet><sheetViews><sheetView state="frozen"><pane xSplit="2" ySplit="1" topLeftCell="C2" activePane="bottomRight"/></sheetView></sheetViews></worksheet>"##;
        let views = parse_views_from_xml(xml);
        assert_eq!(views.len(), 1);
        assert_eq!(views[0].state.as_deref(), Some("frozen"));
        assert_eq!(views[0].x_split, Some(2));
        assert_eq!(views[0].y_split, Some(1));
        assert_eq!(views[0].top_left_cell.as_deref(), Some("C2"));
        assert_eq!(views[0].active_pane.as_deref(), Some("bottomRight"));
    }

    #[test]
    fn test_parse_views_absent() {
        let xml = r##"<worksheet><sheetData></sheetData></worksheet>"##;
        let views = parse_views_from_xml(xml);
        assert!(views.is_empty());
    }

    // -----------------------------------------------------------------------
    // SheetProtection parser tests (v0.11.0)
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_sheet_protection_some_flags() {
        let xml = r##"<worksheet><sheetProtection selectLockedCells="1" formatCells="0"/></worksheet>"##;
        let sp = parse_sheet_protection_from_xml(xml);
        assert!(sp.is_some());
        let sp = sp.unwrap();
        assert_eq!(sp.select_locked_cells, Some(true));
        assert_eq!(sp.format_cells, Some(false));
        assert_eq!(sp.locked, None);
    }

    #[test]
    fn test_parse_sheet_protection_absent() {
        let xml = r##"<worksheet><sheetData></sheetData></worksheet>"##;
        let sp = parse_sheet_protection_from_xml(xml);
        assert!(sp.is_none());
    }

    #[test]
    fn test_parse_boolean_flag_true() {
        assert_eq!(parse_boolean_flag(b"1"), Some(true));
        assert_eq!(parse_boolean_flag(b"true"), Some(true));
        assert_eq!(parse_boolean_flag(b"TRUE"), Some(true));
    }

    #[test]
    fn test_parse_boolean_flag_false() {
        assert_eq!(parse_boolean_flag(b"0"), Some(false));
        assert_eq!(parse_boolean_flag(b"false"), Some(false));
    }

    #[test]
    fn test_parse_boolean_flag_absent() {
        assert_eq!(parse_boolean_flag(b""), None);
        assert_eq!(parse_boolean_flag(b"yes"), None);
    }

    // -----------------------------------------------------------------------
    // Hyperlinks parser tests (v0.11.0)
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_hyperlinks_with_rels() {
        let xml = r##"<worksheet><hyperlinks><hyperlink ref="B2" r:id="rId1"/></hyperlinks></worksheet>"##;
        let mut rels = std::collections::HashMap::new();
        rels.insert("rId1".into(), "https://example.com".into());
        let links = parse_hyperlinks_from_xml(xml, &rels);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0], ("B2".to_string(), "https://example.com".to_string()));
    }

    #[test]
    fn test_parse_hyperlinks_no_rels() {
        let xml = r##"<worksheet><hyperlinks><hyperlink ref="B2" r:id="rId1"/></hyperlinks></worksheet>"##;
        let rels = std::collections::HashMap::new();
        let links = parse_hyperlinks_from_xml(xml, &rels);
        assert!(links.is_empty(), "no rels → no match");
    }

    #[test]
    fn test_parse_hyperlinks_absent() {
        let xml = r##"<worksheet><sheetData></sheetData></worksheet>"##;
        let rels = std::collections::HashMap::new();
        let links = parse_hyperlinks_from_xml(xml, &rels);
        assert!(links.is_empty());
    }

    // -----------------------------------------------------------------------
    // ref_to_rowcol tests (v0.11.0)
    // -----------------------------------------------------------------------

    #[test]
    fn test_ref_to_rowcol_a1() {
        assert_eq!(ref_to_rowcol("A1"), Some((1, 1)));
    }

    #[test]
    fn test_ref_to_rowcol_aa42() {
        assert_eq!(ref_to_rowcol("AA42"), Some((42, 27)));
    }

    #[test]
    fn test_ref_to_rowcol_empty() {
        assert_eq!(ref_to_rowcol(""), None);
    }

    // -----------------------------------------------------------------------
    // Rich-text inline string tests (v0.12.0)
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_inline_str_rich_text() {
        let xml = r##"<worksheet>
        <sheetData>
          <row r="1">
            <c r="A1" t="inlineStr"><is><r><t>Hello</t></r></is></c>
            <c r="B1" t="inlineStr"><is><r><rPr><b/><sz val="14"/><color rgb="FFFF0000"/></rPr><t>Red Bold</t></r></is></c>
            <c r="C1"><v>123</v></c>
            <c r="D1" t="inlineStr"><is><r><rPr><i/></rPr><t>Italic</t></r><r><t> Normal</t></r></is></c>
          </row>
        </sheetData>
        </worksheet>"##;
        let cells = parse_inline_str_rich_text(xml);
        assert_eq!(cells.len(), 3, "expected 3 rich-text cells");

        // A1: plain rich text, no rPr
        assert_eq!(cells[0].0, 1); // row
        assert_eq!(cells[0].1, 1); // col A
        assert_eq!(cells[0].2.len(), 1);
        assert_eq!(cells[0].2[0].text, "Hello");
        assert!(cells[0].2[0].font.is_none(), "no rPr → no font");

        // B1: bold + size 14 + red
        assert_eq!(cells[1].0, 1);
        assert_eq!(cells[1].1, 2); // col B
        assert_eq!(cells[1].2.len(), 1);
        assert_eq!(cells[1].2[0].text, "Red Bold");
        let f = cells[1].2[0].font.as_ref().unwrap();
        assert_eq!(f.bold, Some(true));
        assert_eq!(f.size, Some(14.0));
        assert_eq!(f.color.as_deref(), Some("FFFF0000"));

        // D1: two runs, first italic, second plain
        assert_eq!(cells[2].0, 1);
        assert_eq!(cells[2].1, 4); // col D
        assert_eq!(cells[2].2.len(), 2);
        assert_eq!(cells[2].2[0].text, "Italic");
        assert!(cells[2].2[0].font.as_ref().unwrap().italic == Some(true));
        assert_eq!(cells[2].2[1].text, " Normal");
        assert!(cells[2].2[1].font.is_none());
    }

    #[test]
    fn test_parse_inline_str_rich_text_run_font_name() {
        // rFont val must be captured as font name.
        let xml = r##"<worksheet>
        <sheetData>
          <row r="1">
            <c r="A1" t="inlineStr"><is><r><rPr><rFont val="Arial"/><sz val="12"/></rPr><t>Hi</t></r></is></c>
          </row>
        </sheetData>
        </worksheet>"##;
        let cells = parse_inline_str_rich_text(xml);
        assert_eq!(cells.len(), 1);
        assert_eq!(cells[0].2.len(), 1);
        let f = cells[0].2[0].font.as_ref().unwrap();
        assert_eq!(f.name, Some("Arial".into()));
    }

    #[test]
    fn test_parse_inline_str_rich_text_event_cap() {
        // Finding #4: parser must stop after max_events, not loop unbounded.
        // 3 cells, each 1 run (~10 events per cell; commit happens at </c>).
        let mut cells_xml = String::new();
        for c in 1..=3 {
            cells_xml.push_str(&format!(
                "<row r='{}'><c r='A{}' t='inlineStr'><is><r><t>r{}</t></r></is></c></row>",
                c, c, c
            ));
        }
        let xml = format!("<worksheet><sheetData>{}</sheetData></worksheet>", cells_xml);
        // cap below total → only the first cell is committed, rest truncated
        let cells = parse_inline_str_rich_text_with(&xml, 15);
        assert!(cells.len() < 3, "event cap not enforced: got {} cells", cells.len());
        assert_eq!(cells.len(), 1);
        // cap above total → all cells parsed
        let cells2 = parse_inline_str_rich_text_with(&xml, 1000);
        assert_eq!(cells2.len(), 3);
    }

    #[test]
    fn test_parse_inline_str_rich_text_plain_cell_not_affected() {
        // A regular string cell (not inlineStr) must not produce rich text.
        let xml = r##"<worksheet><sheetData><row r="1">
          <c r="A1" t="s"><v>0</v></c>
          <c r="B1"><v>123</v></c>
        </row></sheetData></worksheet>"##;
        let cells = parse_inline_str_rich_text(xml);
        assert!(cells.is_empty(), "no inlineStr → no rich text rows");
    }

    #[test]
    fn test_gradient_fill_roundtrip() {
        use crate::model::style::{Fill, GradientStop, Style};
        use crate::model::workbook_inner::WorkbookInner;
        use crate::writer::xlsx::workbook_to_bytes;

        let mut inner = WorkbookInner::new();
        let ws = inner.add_worksheet("G".into());
        ws.insert_cell_value(1, 1, CellValue::string("grad"));
        let mut cell = ws.get_cell_by_rc(1, 1);
        let fill = Fill {
            kind: "gradient".into(),
            gradient_type: Some("linear".into()),
            gradient_degree: Some(45.0),
            gradient_stops: Some(vec![
                GradientStop {
                    color: "FFFF0000".into(),
                    position: 0.0,
                },
                GradientStop {
                    color: "FF0000FF".into(),
                    position: 1.0,
                },
            ]),
            ..Default::default()
        };
        cell.set_style_raw(Some(Style {
            fill: Some(fill),
            ..Default::default()
        }));

        let bytes = workbook_to_bytes(&inner).unwrap();
        let read = crate::reader::xlsx::workbook_inner_from_bytes(&bytes).unwrap();
        let cell = read.worksheets[0].get_cell_by_rc(1, 1);
        let s = cell.style();
        let f = s.unwrap().fill.unwrap();
        assert_eq!(f.kind, "gradient");
        assert_eq!(f.gradient_type.as_deref(), Some("linear"));
        assert_eq!(f.gradient_degree, Some(45.0));
        let stops = f.gradient_stops.unwrap();
        assert_eq!(stops.len(), 2);
        assert_eq!(stops[0].color, "FFFF0000");
        assert_eq!(stops[1].color, "FF0000FF");
    }

    #[test]
    fn test_diagonal_border_roundtrip() {
        use crate::model::style::{Border, BorderStyle, Style};
        use crate::model::workbook_inner::WorkbookInner;
        use crate::writer::xlsx::workbook_to_bytes;

        let mut inner = WorkbookInner::new();
        let ws = inner.add_worksheet("D".into());
        ws.insert_cell_value(1, 1, CellValue::string("diag"));
        let mut cell = ws.get_cell_by_rc(1, 1);
        cell.set_style_raw(Some(Style {
            border: Some(Border {
                diagonal: Some(BorderStyle {
                    style: "thin".into(),
                    color: Some("FF000000".into()),
                    ..Default::default()
                }),
                diagonal_up: Some(true),
                diagonal_down: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        }));

        let bytes = workbook_to_bytes(&inner).unwrap();
        let read = crate::reader::xlsx::workbook_inner_from_bytes(&bytes).unwrap();
        let cell = read.worksheets[0].get_cell_by_rc(1, 1);
        let s = cell.style();
        let b = s.unwrap().border.unwrap();
        assert!(b.diagonal.is_some());
        assert_eq!(b.diagonal.as_ref().unwrap().style, "thin");
        assert_eq!(b.diagonal.as_ref().unwrap().color.as_deref(), Some("FF000000"));
        assert_eq!(b.diagonal_up, Some(true));
        assert_eq!(b.diagonal_down, Some(true));
    }

    #[test]
    fn test_rich_text_roundtrip() {
        use crate::model::workbook_inner::WorkbookInner;
        use crate::writer::xlsx::workbook_to_bytes;

        let mut inner = WorkbookInner::new();
        let ws = inner.add_worksheet("S".into());
        ws.insert_cell_value(
            1,
            1,
            CellValue::rich_text(vec![
                RichTextRun {
                    text: "Hello ".into(),
                    font: Some(Font {
                        bold: Some(true),
                        size: Some(14.0),
                        name: Some("Arial".into()),
                        ..Default::default()
                    }),
                },
                RichTextRun {
                    text: "World".into(),
                    font: None,
                },
            ]),
        );

        let bytes = workbook_to_bytes(&inner).unwrap();
        let read = crate::reader::xlsx::workbook_inner_from_bytes(&bytes).unwrap();
        let cell = read.worksheets[0].get_cell_by_rc(1, 1);
        let cv = cell.value_raw();
        assert_eq!(cv.value_type, "RichText");
        let runs = cv.rich_text.as_ref().unwrap();
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].text, "Hello ");
        assert_eq!(runs[0].font.as_ref().unwrap().bold, Some(true));
        assert_eq!(runs[0].font.as_ref().unwrap().size, Some(14.0));
        assert_eq!(runs[0].font.as_ref().unwrap().name.as_deref(), Some("Arial"));
        assert_eq!(runs[1].text, "World");
        assert!(runs[1].font.is_none());
    }

    #[test]
    fn test_conditional_formatting_roundtrip() {
        use crate::model::conditional_formatting::{CfRule, ConditionalFormat};
        use crate::model::style::{Fill, Font, Style};
        use crate::writer::xlsx::workbook_to_bytes;

        let mut inner = WorkbookInner::new();
        let ws = inner.add_worksheet("Sheet1".into());
        let cf = ConditionalFormat {
            sqref: "A1:A10".into(),
            rules: vec![CfRule {
                r#type: "cellIs".into(),
                priority: 0,
                dxf_id: None,
                operator: Some("lessThan".into()),
                formula: Some(vec!["10".into()]),
                text: None,
                time_period: None,
                rank: None,
                percent: None,
                bottom: None,
                style: Some(Style {
                    font: Some(Font {
                        bold: Some(true),
                        ..Default::default()
                    }),
                    fill: Some(Fill {
                        kind: "solid".into(),
                        foreground: Some("FFFF0000".into()),
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
                cfvo: None,
                color: None,
                data_bar_color: None,
                icon_set: None,
                reverse: None,
                show_value: None,
            }],
        };
        ws.add_conditional_formatting(cf).unwrap();

        let bytes = workbook_to_bytes(&inner).unwrap();
        let read = crate::reader::xlsx::workbook_inner_from_bytes(&bytes).unwrap();
        let cfs = read.worksheets[0].get_conditional_formatting();
        assert_eq!(cfs.len(), 1, "conditional format count mismatch");
        assert_eq!(cfs[0].sqref, "A1:A10");
        assert_eq!(cfs[0].rules.len(), 1);
        let r = &cfs[0].rules[0];
        assert_eq!(r.r#type, "cellIs");
        assert_eq!(r.operator.as_deref(), Some("lessThan"));
        assert_eq!(r.formula.as_ref().unwrap(), &vec!["10".to_string()]);
        assert!(r.dxf_id.is_some(), "dxfId should be assigned on write");
        let style = r.style.as_ref().expect("style should round-trip");
        assert_eq!(style.font.as_ref().unwrap().bold, Some(true));
        assert_eq!(style.fill.as_ref().unwrap().foreground.as_deref(), Some("FFFF0000"));
    }

    #[test]
    fn test_foreign_dxfs_preserved() {
        use crate::model::style::{Dxf, Fill, Font};
        use crate::writer::xlsx::workbook_to_bytes;

        let mut inner = WorkbookInner::new();
        inner.add_worksheet("Sheet1".into());
        // Inject dxfs that no conditional-format rule references (e.g. pivot tables).
        inner.dxfs.push(Dxf {
            font: Some(Font {
                bold: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        });
        inner.dxfs.push(Dxf {
            fill: Some(Fill {
                kind: "solid".into(),
                foreground: Some("FF00FF00".into()),
                ..Default::default()
            }),
            ..Default::default()
        });

        let bytes = workbook_to_bytes(&inner).unwrap();
        let read = crate::reader::xlsx::workbook_inner_from_bytes(&bytes).unwrap();
        assert_eq!(read.dxfs.len(), 2, "foreign dxfs dropped on round-trip");
    }

    #[test]
    fn test_conditional_formatting_preserves_explicit_priority() {
        use crate::model::conditional_formatting::{CfRule, ConditionalFormat};
        use crate::writer::xlsx::workbook_to_bytes;

        let mut inner = WorkbookInner::new();
        let ws = inner.add_worksheet("Sheet1".into());

        // Explicit priorities out of document order; A3 uses 0 = auto-assign.
        ws.add_conditional_formatting(ConditionalFormat {
            sqref: "A1".into(),
            rules: vec![CfRule {
                r#type: "expression".into(),
                priority: 5,
                formula: Some(vec!["A1>1".into()]),
                ..Default::default()
            }],
        })
        .unwrap();
        ws.add_conditional_formatting(ConditionalFormat {
            sqref: "A2".into(),
            rules: vec![CfRule {
                r#type: "expression".into(),
                priority: 2,
                formula: Some(vec!["A2>1".into()]),
                ..Default::default()
            }],
        })
        .unwrap();
        ws.add_conditional_formatting(ConditionalFormat {
            sqref: "A3".into(),
            rules: vec![CfRule {
                r#type: "expression".into(),
                priority: 0,
                formula: Some(vec!["A3>1".into()]),
                ..Default::default()
            }],
        })
        .unwrap();

        let bytes = workbook_to_bytes(&inner).unwrap();
        let read = crate::reader::xlsx::workbook_inner_from_bytes(&bytes).unwrap();
        let cfs = read.worksheets[0].get_conditional_formatting();

        let a1 = cfs.iter().find(|c| c.sqref == "A1").expect("A1 missing");
        let a2 = cfs.iter().find(|c| c.sqref == "A2").expect("A2 missing");
        let a3 = cfs.iter().find(|c| c.sqref == "A3").expect("A3 missing");

        assert_eq!(a1.rules[0].priority, 5, "explicit priority 5 must be preserved");
        assert_eq!(a2.rules[0].priority, 2, "explicit priority 2 must be preserved");
        assert_ne!(a3.rules[0].priority, 0, "auto-assigned priority must be non-zero");
        assert_eq!(
            a3.rules[0].priority, 1,
            "auto-assigned priority must be the first free slot (1)"
        );
        let mut ps: Vec<u32> = cfs.iter().flat_map(|c| c.rules.iter().map(|r| r.priority)).collect();
        ps.sort_unstable();
        ps.dedup();
        assert_eq!(ps.len(), 3, "priorities must stay unique across the sheet");
    }

    #[test]
    fn test_cf_auto_priority_never_collides_with_explicit() {
        use crate::model::conditional_formatting::{CfRule, ConditionalFormat};
        use crate::writer::xlsx::workbook_to_bytes;
        let mut inner = WorkbookInner::new();
        let ws = inner.add_worksheet("Sheet1".into());
        // Auto rule added FIRST (priority 0), then an explicit rule wanting 1.
        ws.add_conditional_formatting(ConditionalFormat {
            sqref: "A1".into(),
            rules: vec![CfRule {
                r#type: "expression".into(),
                priority: 0,
                formula: Some(vec!["A1>1".into()]),
                ..Default::default()
            }],
        })
        .unwrap();
        ws.add_conditional_formatting(ConditionalFormat {
            sqref: "A2".into(),
            rules: vec![CfRule {
                r#type: "expression".into(),
                priority: 1,
                formula: Some(vec!["A2>1".into()]),
                ..Default::default()
            }],
        })
        .unwrap();
        let bytes = workbook_to_bytes(&inner).unwrap();
        let read = crate::reader::xlsx::workbook_inner_from_bytes(&bytes).unwrap();
        let cfs = read.worksheets[0].get_conditional_formatting();
        let a1 = cfs.iter().find(|c| c.sqref == "A1").unwrap();
        let a2 = cfs.iter().find(|c| c.sqref == "A2").unwrap();
        assert_eq!(a2.rules[0].priority, 1, "explicit priority 1 must be honored");
        assert_ne!(a1.rules[0].priority, 1, "auto rule must not steal explicit priority 1");
        assert_eq!(a1.rules[0].priority, 2, "auto rule should take next free slot");
    }

    #[test]
    fn test_cf_duplicate_explicit_priority_rejected() {
        use crate::model::conditional_formatting::{CfRule, ConditionalFormat};
        let mut inner = WorkbookInner::new();
        let ws = inner.add_worksheet("Sheet1".into());
        ws.add_conditional_formatting(ConditionalFormat {
            sqref: "A1".into(),
            rules: vec![CfRule {
                r#type: "expression".into(),
                priority: 5,
                formula: Some(vec!["A1>1".into()]),
                ..Default::default()
            }],
        })
        .unwrap();
        let dup = ws.add_conditional_formatting(ConditionalFormat {
            sqref: "A2".into(),
            rules: vec![CfRule {
                r#type: "expression".into(),
                priority: 5,
                formula: Some(vec!["A2>1".into()]),
                ..Default::default()
            }],
        });
        assert!(dup.is_err(), "duplicate explicit priority must be rejected");
    }

    #[test]
    fn test_cf_quickxml_attrs_isolated() {
        use quick_xml::events::Event;
        use quick_xml::Reader;
        let xml = r##"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData/><conditionalFormatting sqref="A1:A10"><cfRule type="cellIs"/></conditionalFormatting></worksheet>"##;
        let mut reader = Reader::from_str(xml);
        let mut found = std::collections::HashMap::new();
        loop {
            let mut buf = Vec::new();
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                    let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                    let mut attrs = String::new();
                    for attr in e.attributes().flatten() {
                        attrs.push_str(&format!(
                            "{}={} ",
                            String::from_utf8_lossy(attr.key.as_ref()),
                            String::from_utf8_lossy(&attr.value)
                        ));
                    }
                    found.insert(name, attrs);
                }
                Ok(Event::Eof) => break,
                _ => {}
            }
        }
        eprintln!("ISOLATED: {:?}", found);
        assert!(
            !found.get("conditionalFormatting").unwrap_or(&String::new()).is_empty(),
            "conditionalFormatting attrs empty: {:?}",
            found
        );
    }

    #[test]
    fn test_parse_merge_cells_explicit_open_close() {
        // Some producers serialize empty elements as explicit open/close
        // (<mergeCell ref="…"></mergeCell>) instead of self-closing. The reader
        // must accept both forms (mirrors every other matcher in this file).
        let xml = r#"<mergeCells count="1"><mergeCell ref="A1:C3"></mergeCell></mergeCells>"#;
        let ranges = parse_merge_cells_from_xml(xml);
        assert!(
            ranges.contains(&"A1:C3".to_string()),
            "explicit <mergeCell></mergeCell> must be parsed, got {ranges:?}"
        );
    }

    #[test]
    fn test_parse_merge_cells_namespaced() {
        // A producer that emits a namespace-prefixed <x:mergeCell> must still
        // be parsed — the matcher must compare local name, not qualified name.
        let xml = r#"<mergeCells count="1"><x:mergeCell ref="A1:C3"></x:mergeCell></mergeCells>"#;
        let ranges = parse_merge_cells_from_xml(xml);
        assert!(
            ranges.contains(&"A1:C3".to_string()),
            "namespaced <mergeCell> must parse, got {ranges:?}"
        );
    }

    // -- Security regression guards --

    #[test]
    fn test_xml_col_range_bounded() {
        // Bug 1: XML range bomb — unbounded lo..=hi loop with u32 max.
        // A single crafted <col> with min=1, max=4294967295 would iterate
        // 4 billion times. This test verifies the fix caps hi at 16384.
        let xml = r#"<cols><col min="1" max="4294967295" outlineLevel="1"/></cols>"#;

        let start = std::time::Instant::now();
        let result = parse_col_outline_levels_from_xml(xml);
        let elapsed = start.elapsed();

        // With fix (cap at 16384): ~16k entries, well under 100ms.
        // Without fix: would either hang forever or OOM.
        assert!(
            elapsed.as_millis() < 5000,
            "Bug 1: XML range bomb took {elapsed:?} — unbounded loop!"
        );

        assert!(!result.is_empty(), "should have parsed outline levels");

        // Verify the max is bounded at Excel's column limit (16384)
        let max_col = result.iter().map(|(c, _)| *c).max().unwrap_or(0);
        assert!(
            max_col <= 16384,
            "Bug 1: max col {max_col} exceeds 16384 — unbounded range bomb"
        );
    }
}
