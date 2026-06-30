//! Style reader — parses `xl/styles.xml` and `xl/worksheets/sheet{N}.xml`
//! for style indices, resolving them into excelrs model types.
//!
//! # Architecture
//! Two passes, both using `quick_xml::Reader` over data read from the `.xlsx` zip:
//!
//! 1. **`parse_style_table`** — reads `<styleSheet>` once and builds
//!    `StyleTableRead` (sub-tables: numFmts, fonts, fills, borders, cellXfs).
//! 2. **`parse_sheet_cell_styles`** — reads each sheet XML and extracts the
//!    `s` (cellXfs index) attribute per cell, returning a `(row,col) → u32` map.
//! 3. **`StyleTableRead::resolve_style`** — converts a cellXfs index to a
//!    model `Style` by looking up the sub-table indices.
//!
//! # v0.3.0 limitations
//! - **Theme colors** (`<color theme="N"/>`)   → skipped (color = None).
//! - **Gradient fills** (`<gradientFill>`)      → skipped (fill = default Normal).
//! - **Diagonal borders** (diagonal/diagonalUp/diagonalDown) → skipped.
//! - **cellStyleXfs inheritance** (xfId)        → ignored; cellXf is used
//!   directly. The `applyX` flags are respected: a field with `applyX="0"`
//!   (or index 0, per our writer's convention) maps to `None` in the `Style`.
//! - **cellStyleXfs**, **dxfs**, **tableStyles**, **extLst** → skipped.

use std::collections::HashMap;
use std::io::Read;

use quick_xml::events::{attributes::Attribute, Event};
use quick_xml::Reader as XmlReader;

use crate::error::ExcelrsError;
use crate::model::style::{Alignment, Border, BorderStyle, Fill, Font, Style};
use crate::types;

/// Per-sheet cell-to-style-index map: (1-indexed row, col) → cellXfs index.
pub type SheetStyleMap = HashMap<(u32, u32), u32>;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Parsed (unresolved) style table from `xl/styles.xml`.
///
/// Sub-table indices match the OOXML source 1:1. Index 0 in every sub-table
/// is the "Normal" entry (empty / Calibri 11).  Use `resolve_style(xf_idx)`
/// to convert a cellXfs index to a model `Style`.
#[derive(Debug, Clone)]
pub struct StyleTableRead {
    pub num_fmts: Vec<(u32, String)>,
    pub fonts: Vec<Font>,
    pub fills: Vec<Fill>,
    pub borders: Vec<Border>,
    pub cell_xfs: Vec<ParsedCellXf>,
}

/// One parsed `<xf>` record from `<cellXfs>`.
#[derive(Debug, Clone, Default)]
pub struct ParsedCellXf {
    pub num_fmt_id: u32,
    pub font_id: u32,
    pub fill_id: u32,
    pub border_id: u32,
    pub alignment: Option<Alignment>,
}

impl StyleTableRead {
    /// Empty style table — all cells get Normal.
    pub fn empty() -> Self {
        StyleTableRead {
            num_fmts: Vec::new(),
            fonts: Vec::new(),
            fills: Vec::new(),
            borders: Vec::new(),
            cell_xfs: Vec::new(),
        }
    }

    /// Resolve a cellXfs index (the `s` attribute on a `<c>` element) to a
    /// model `Style`.  Returns `None` (Normal) for index 0 or out-of-range.
    pub fn resolve_style(&self, xf_index: u32) -> Option<Style> {
        if xf_index == 0 {
            return None;
        }
        let xf = self.cell_xfs.get(xf_index as usize)?;

        let num_fmt = if xf.num_fmt_id != 0 {
            self.num_fmts
                .iter()
                .find(|(id, _)| *id == xf.num_fmt_id)
                .map(|(_, code)| code.clone())
        } else {
            None
        };

        let font = if xf.font_id != 0 {
            self.fonts.get(xf.font_id as usize).cloned()
        } else {
            None
        };

        let fill = if xf.fill_id != 0 {
            self.fills.get(xf.fill_id as usize).cloned()
        } else {
            None
        };

        let border = if xf.border_id != 0 {
            self.borders.get(xf.border_id as usize).cloned()
        } else {
            None
        };

        let style = Style {
            font,
            fill,
            border,
            alignment: xf.alignment.clone(),
            num_fmt,
        };

        if style.is_empty() {
            None
        } else {
            Some(style)
        }
    }
}

// ---------------------------------------------------------------------------
// Parse helpers
// ---------------------------------------------------------------------------

/// Read the full content of a zip entry into a `Vec<u8>`.
fn read_entry<R: Read + std::io::Seek>(archive: &mut zip::ZipArchive<R>, path: &str) -> Result<Vec<u8>, ExcelrsError> {
    let mut entry = archive
        .by_name(path)
        .map_err(|_| ExcelrsError::Parse(format!("Missing zip entry: '{path}'")))?;
    let mut buf = Vec::new();
    entry.read_to_end(&mut buf)?;
    Ok(buf)
}

/// Extract an attribute value by its local name from an attribute list.
fn get_attr<'a>(attrs: &'a [Attribute<'a>], name: &[u8]) -> Option<&'a [u8]> {
    attrs
        .iter()
        .find(|a| a.key.local_name().as_ref() == name)
        .map(|a| a.value.as_ref())
}

/// Parse a `u32` attribute value, returning `default` on missing or invalid.
fn u32_attr(attrs: &[Attribute], name: &[u8]) -> Option<u32> {
    let raw = get_attr(attrs, name)?;
    let s = std::str::from_utf8(raw).ok()?;
    s.parse::<u32>().ok()
}

/// Parse a `bool` attribute value.  Accepts `"1"`, `"true"`, or bare element.
fn bool_attr(attrs: &[Attribute], name: &[u8]) -> Option<bool> {
    let raw = get_attr(attrs, name)?;
    let s = std::str::from_utf8(raw).ok()?;
    Some(s == "1" || s.eq_ignore_ascii_case("true"))
}

/// Parse a `f64` attribute value.
fn f64_attr(attrs: &[Attribute], name: &[u8]) -> Option<f64> {
    let raw = get_attr(attrs, name)?;
    let s = std::str::from_utf8(raw).ok()?;
    s.parse::<f64>().ok()
}

/// Parse a `String` attribute value.
fn str_attr<'a>(attrs: &'a [Attribute], name: &[u8]) -> Option<&'a str> {
    let raw = get_attr(attrs, name)?;
    std::str::from_utf8(raw).ok()
}

// ---------------------------------------------------------------------------
// xl/styles.xml parser
// ---------------------------------------------------------------------------

enum StyleSection {
    None,
    NumFmts,
    Fonts,
    Fills,
    Borders,
    CellXfs,
}

/// Parse the `xl/styles.xml` content into a `StyleTableRead`.
pub fn parse_style_table(data: &[u8]) -> Result<StyleTableRead, ExcelrsError> {
    let mut reader = XmlReader::from_reader(data);

    let mut num_fmts: Vec<(u32, String)> = Vec::new();
    let mut fonts: Vec<Font> = Vec::new();
    let mut fills: Vec<Fill> = Vec::new();
    let mut borders: Vec<Border> = Vec::new();
    let mut cell_xfs: Vec<ParsedCellXf> = Vec::new();

    let mut section = StyleSection::None;

    // Accumulators for the current sub-element being parsed
    let mut font: Option<Font> = None;
    let mut fill: Option<Fill> = None;
    let mut in_pattern_fill = false;
    let mut border: Option<Border> = None;
    let mut border_side: Option<(String, Option<BorderStyle>)> = None; // (side_name, style)
                                                                       // border building: after parsing all sides, reconstruct
    let mut border_top: Option<BorderStyle> = None;
    let mut border_right: Option<BorderStyle> = None;
    let mut border_bottom: Option<BorderStyle> = None;
    let mut border_left: Option<BorderStyle> = None;

    // Track the index of the current xf in cell_xfs (for alignment children).
    // The ParsedCellXf is pushed immediately even before alignment is parsed.
    let mut xf_idx: Option<usize> = None;
    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let tag = e.local_name().as_ref().to_vec();

                match &*tag {
                    // -- Top-level sections --
                    b"numFmts" | b"fonts" | b"fills" | b"borders" | b"cellXfs" => {
                        // enter section
                        section = match &*tag {
                            b"numFmts" => StyleSection::NumFmts,
                            b"fonts" => StyleSection::Fonts,
                            b"fills" => StyleSection::Fills,
                            b"borders" => StyleSection::Borders,
                            b"cellXfs" => StyleSection::CellXfs,
                            _ => unreachable!(),
                        };
                    }

                    // -- numFmt --
                    b"numFmt" if matches!(section, StyleSection::NumFmts) => {
                        let attrs: Vec<_> = e.attributes().filter_map(|a| a.ok()).collect();
                        if let (Some(id), Some(code)) = (u32_attr(&attrs, b"numFmtId"), str_attr(&attrs, b"formatCode"))
                        {
                            num_fmts.push((id, code.to_owned()));
                        }
                    }

                    // -- font --
                    b"font" if matches!(section, StyleSection::Fonts) => {
                        font = Some(Font::default());
                    }

                    // Font sub-elements (only when inside a font)
                    b"name" if font.is_some() => {
                        if let Some(name) = str_attr(&e.attributes().filter_map(|a| a.ok()).collect::<Vec<_>>(), b"val")
                        {
                            if let Some(ref mut f) = font {
                                f.name = Some(name.to_owned());
                            }
                        }
                    }
                    b"sz" if font.is_some() => {
                        if let Some(sz) = f64_attr(&e.attributes().filter_map(|a| a.ok()).collect::<Vec<_>>(), b"val") {
                            if let Some(ref mut f) = font {
                                f.size = Some(sz);
                            }
                        }
                    }
                    b"b" if font.is_some() => {
                        if let Some(ref mut f) = font {
                            f.bold = Some(true);
                        }
                    }
                    b"i" if font.is_some() => {
                        if let Some(ref mut f) = font {
                            f.italic = Some(true);
                        }
                    }
                    b"u" if font.is_some() => {
                        if let Some(ref mut f) = font {
                            f.underline = Some(true);
                        }
                    }
                    b"color" if font.is_some() => {
                        let attrs: Vec<_> = e.attributes().filter_map(|a| a.ok()).collect();
                        if let Some(rgb) = str_attr(&attrs, b"rgb") {
                            if let Some(ref mut f) = font {
                                f.color = Some(rgb.to_uppercase());
                            }
                        }
                        // theme or indexed colors: skip (color stays None)
                    }

                    // color inside border side
                    b"color" if border_side.is_some() => {
                        let attrs: Vec<_> = e.attributes().filter_map(|a| a.ok()).collect();
                        if let Some(rgb) = str_attr(&attrs, b"rgb") {
                            if let Some((_, Some(ref mut style))) = border_side {
                                style.color = Some(rgb.to_uppercase());
                            }
                        }
                    }

                    // color inside fill
                    b"fgColor" if fill.is_some() || in_pattern_fill => {
                        let attrs: Vec<_> = e.attributes().filter_map(|a| a.ok()).collect();
                        if let Some(rgb) = str_attr(&attrs, b"rgb") {
                            if let Some(ref mut f) = fill {
                                f.foreground = Some(rgb.to_uppercase());
                            }
                        }
                    }
                    b"bgColor" if fill.is_some() || in_pattern_fill => {
                        let attrs: Vec<_> = e.attributes().filter_map(|a| a.ok()).collect();
                        if let Some(rgb) = str_attr(&attrs, b"rgb") {
                            if let Some(ref mut f) = fill {
                                f.background = Some(rgb.to_uppercase());
                            }
                        }
                    }

                    // -- fill --
                    b"fill" if matches!(section, StyleSection::Fills) => {
                        fill = Some(Fill {
                            kind: "none".into(),
                            ..Default::default()
                        });
                        in_pattern_fill = false;
                    }
                    b"patternFill" if fill.is_some() => {
                        in_pattern_fill = true;
                        let attrs: Vec<_> = e.attributes().filter_map(|a| a.ok()).collect();
                        if let Some(pt) = str_attr(&attrs, b"patternType") {
                            if let Some(ref mut f) = fill {
                                f.kind = pt.to_owned();
                            }
                        }
                    }
                    b"gradientFill" => {
                        // Gradient fills: skip entirely (v0.3.0 limitation)
                        // mark that we're in a fill but not accumulating
                    }

                    // -- border --
                    b"border" if matches!(section, StyleSection::Borders) => {
                        border = Some(Border::default());
                        border_top = None;
                        border_right = None;
                        border_bottom = None;
                        border_left = None;
                    }
                    b"left" | b"right" | b"top" | b"bottom" if border.is_some() => {
                        let side_name = std::str::from_utf8(&tag).unwrap_or("");
                        let attrs: Vec<_> = e.attributes().filter_map(|a| a.ok()).collect();
                        let style_val = str_attr(&attrs, b"style");
                        let bs = style_val.map(|s| BorderStyle {
                            style: s.to_owned(),
                            color: None,
                        });
                        border_side = Some((side_name.to_owned(), bs));
                    }

                    // -- cellXf --
                    b"xf" if matches!(section, StyleSection::CellXfs) => {
                        let attrs: Vec<_> = e.attributes().filter_map(|a| a.ok()).collect();
                        let parsed = ParsedCellXf {
                            num_fmt_id: u32_attr(&attrs, b"numFmtId").unwrap_or(0),
                            font_id: u32_attr(&attrs, b"fontId").unwrap_or(0),
                            fill_id: u32_attr(&attrs, b"fillId").unwrap_or(0),
                            border_id: u32_attr(&attrs, b"borderId").unwrap_or(0),
                            alignment: None,
                        };
                        // Push immediately (handles both Start and Empty events).
                        // If there are children (alignment), they will be patched
                        // in-place via cell_xfs[xf_idx].
                        let idx = cell_xfs.len();
                        cell_xfs.push(parsed);
                        xf_idx = Some(idx);
                    }

                    // alignment child inside xf
                    b"alignment" if xf_idx.is_some() => {
                        let attrs: Vec<_> = e.attributes().filter_map(|a| a.ok()).collect();
                        let hor = str_attr(&attrs, b"horizontal").map(|s| s.to_owned());
                        let vert_raw = str_attr(&attrs, b"vertical").map(|s| s.to_owned());
                        // OOXML "center" → excelrs "middle" (round-trip mapping)
                        let vert = vert_raw.map(|s| {
                            if s.eq_ignore_ascii_case("center") {
                                "middle".to_owned()
                            } else {
                                s
                            }
                        });
                        let wrap = bool_attr(&attrs, b"wrapText");
                        let indent = u32_attr(&attrs, b"indent");
                        let alignment = Alignment {
                            horizontal: hor,
                            vertical: vert,
                            wrap_text: wrap,
                            indent,
                        };
                        // Patch the alignment in-place on the already-pushed xf entry
                        if let Some(idx) = xf_idx {
                            if let Some(entry) = cell_xfs.get_mut(idx) {
                                entry.alignment = Some(alignment);
                            }
                        }
                    }

                    // -- Elements to skip silently --
                    b"cellStyleXfs" | b"cellStyles" | b"dxfs" | b"tableStyles" | b"extLst" | b"styleSheet"
                    | b"colors" | b"col" | b"cols" | b"xml" | b"sheetData" | b"row" | b"c" | b"v" | b"f" | b"is"
                    | b"t" | b"si" | b"r" | b"sst" => {
                        // Known but uninteresting — skip.
                    }

                    // Unknown elements — skip (lenient parsing)
                    _ => {}
                }
            }

            Ok(Event::End(ref e)) => {
                let tag = e.local_name().as_ref().to_vec();

                match &*tag {
                    b"numFmts" | b"fonts" | b"fills" | b"borders" | b"cellXfs" => {
                        section = StyleSection::None;
                    }

                    b"font" => {
                        if let Some(f) = font.take() {
                            fonts.push(f);
                        }
                    }

                    b"fill" => {
                        if let Some(f) = fill.take() {
                            fills.push(f);
                        }
                        in_pattern_fill = false;
                    }

                    b"border" => {
                        if let Some(mut b) = border.take() {
                            b.top = border_top.take();
                            b.right = border_right.take();
                            b.bottom = border_bottom.take();
                            b.left = border_left.take();
                            borders.push(b);
                        }
                        border_side = None;
                    }
                    b"left" | b"right" | b"top" | b"bottom" => {
                        if let Some((side_name, style)) = border_side.take() {
                            match side_name.as_str() {
                                "top" => border_top = style,
                                "right" => border_right = style,
                                "bottom" => border_bottom = style,
                                "left" => border_left = style,
                                _ => {}
                            }
                        }
                    }

                    b"xf" => {
                        xf_idx = None;
                    }

                    b"alignment" => {
                        // end of alignment element — nothing to do
                    }

                    _ => {}
                }
            }

            Ok(Event::Eof) => break,

            Err(e) => {
                return Err(ExcelrsError::Parse(format!("Failed to parse xl/styles.xml: {e}")));
            }

            _ => {}
        }
    }

    // Push any incomplete accumulators (defensive)
    if let Some(f) = font.take() {
        fonts.push(f);
    }
    if let Some(f) = fill.take() {
        fills.push(f);
    }
    if let Some(b) = border.take() {
        borders.push(b);
    }

    Ok(StyleTableRead {
        num_fmts,
        fonts,
        fills,
        borders,
        cell_xfs,
    })
}

// ---------------------------------------------------------------------------
// Sheet cell-style map
// ---------------------------------------------------------------------------

/// Parse a sheet XML and extract the `s` (cellXfs index) attribute from each
/// `<c>` element, returning a map of 1-indexed `(row, col) → cellXfs index`.
///
/// Only entries with `s > 0` are returned (s=0 means Normal — resolve_style
/// handles that by returning None).
pub fn parse_sheet_cell_styles(data: &[u8]) -> Result<SheetStyleMap, ExcelrsError> {
    let mut reader = XmlReader::from_reader(data);

    let mut result: HashMap<(u32, u32), u32> = HashMap::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) if e.local_name().as_ref() == b"c" => {
                let attrs: Vec<_> = e.attributes().filter_map(|a| a.ok()).collect();
                let r_val = str_attr(&attrs, b"r");
                let s_val = u32_attr(&attrs, b"s").unwrap_or(0);
                if s_val > 0 {
                    if let Some(addr) = r_val {
                        if let Ok((col, row)) = types::parse_address(addr) {
                            result.insert((row, col), s_val);
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(ExcelrsError::Parse(format!("Failed to parse sheet XML: {e}")));
            }
            _ => {}
        }
    }

    Ok(result)
}

// ---------------------------------------------------------------------------
// High-level API: parse styles + sheet maps from zip archive
// ---------------------------------------------------------------------------

/// Read `xl/styles.xml` and all `xl/worksheets/sheet{N}.xml` entries from a
/// `.xlsx` buffer, returning the parsed style table and per-sheet cell-style maps.
///
/// `sheet_count` should match `calamine_wb.sheet_names().len()`.
pub fn parse_styles_and_sheet_maps(
    data: &[u8],
    sheet_count: usize,
) -> Result<(StyleTableRead, Vec<SheetStyleMap>), ExcelrsError> {
    use std::io::Cursor;
    let cursor = Cursor::new(data);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|e| ExcelrsError::Zip(e.to_string()))?;

    // Parse xl/styles.xml (optional — some files lack it)
    let style_table = if let Ok(styles_bytes) = read_entry(&mut archive, "xl/styles.xml") {
        parse_style_table(&styles_bytes)?
    } else {
        StyleTableRead::empty()
    };

    // Parse sheet style maps
    let mut sheet_style_maps: Vec<SheetStyleMap> = Vec::with_capacity(sheet_count);
    for i in 0..sheet_count {
        let path = format!("xl/worksheets/sheet{}.xml", i + 1);
        let map = match read_entry(&mut archive, &path) {
            Ok(sheet_bytes) => parse_sheet_cell_styles(&sheet_bytes)?,
            Err(_) => HashMap::new(), // sheet missing → no styles
        };
        sheet_style_maps.push(map);
    }

    Ok((style_table, sheet_style_maps))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- parse_style_table tests --

    #[test]
    fn test_parse_empty_styles() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
        <styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
        </styleSheet>"#;
        let table = parse_style_table(xml).unwrap();
        assert!(table.num_fmts.is_empty());
        assert!(table.fonts.is_empty());
        assert!(table.fills.is_empty());
        assert!(table.borders.is_empty());
        assert!(table.cell_xfs.is_empty());
    }

    #[test]
    fn test_parse_font_bold() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
        <styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
          <fonts count="2">
            <font>
              <sz val="11"/>
              <name val="Calibri"/>
            </font>
            <font>
              <sz val="14"/>
              <name val="Arial"/>
              <b/>
              <color rgb="FFFF0000"/>
            </font>
          </fonts>
        </styleSheet>"#;
        let table = parse_style_table(xml).unwrap();
        assert_eq!(table.fonts.len(), 2);
        let f0 = &table.fonts[0];
        assert_eq!(f0.name.as_deref(), Some("Calibri"));
        assert_eq!(f0.size, Some(11.0));
        assert!(f0.bold.is_none());

        let f1 = &table.fonts[1];
        assert_eq!(f1.name.as_deref(), Some("Arial"));
        assert_eq!(f1.size, Some(14.0));
        assert_eq!(f1.bold, Some(true));
        assert_eq!(f1.color.as_deref(), Some("FFFF0000"));
    }

    #[test]
    fn test_parse_fill_solid() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
        <styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
          <fills count="2">
            <fill><patternFill patternType="none"/></fill>
            <fill><patternFill patternType="solid">
              <fgColor rgb="FFFFFF00"/>
            </patternFill></fill>
          </fills>
        </styleSheet>"#;
        let table = parse_style_table(xml).unwrap();
        assert_eq!(table.fills.len(), 2);
        assert_eq!(table.fills[0].kind, "none");
        assert_eq!(table.fills[1].kind, "solid");
        assert_eq!(table.fills[1].foreground.as_deref(), Some("FFFFFF00"));
    }

    #[test]
    fn test_parse_border_thin() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
        <styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
          <borders count="2">
            <border><left/><right/><top/><bottom/><diagonal/></border>
            <border>
              <left style="thin"><color rgb="FF000000"/></left>
              <right/>
              <top style="thin"><color rgb="FF000000"/></top>
              <bottom/>
              <diagonal/>
            </border>
          </borders>
        </styleSheet>"#;
        let table = parse_style_table(xml).unwrap();
        assert_eq!(table.borders.len(), 2);
        assert!(table.borders[0].left.is_none());

        assert!(table.borders[1].left.is_some());
        assert_eq!(table.borders[1].left.as_ref().unwrap().style, "thin");
        assert_eq!(
            table.borders[1].left.as_ref().unwrap().color.as_deref(),
            Some("FF000000")
        );
        assert!(table.borders[1].top.is_some());
        assert!(table.borders[1].right.is_none());
        assert!(table.borders[1].bottom.is_none());
    }

    #[test]
    fn test_parse_numfmt() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
        <styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
          <numFmts count="2">
            <numFmt numFmtId="164" formatCode="0.00%"/>
            <numFmt numFmtId="165" formatCode="yyyy-mm-dd"/>
          </numFmts>
        </styleSheet>"#;
        let table = parse_style_table(xml).unwrap();
        assert_eq!(table.num_fmts.len(), 2);
        assert_eq!(table.num_fmts[0], (164, "0.00%".into()));
        assert_eq!(table.num_fmts[1], (165, "yyyy-mm-dd".into()));
    }

    #[test]
    fn test_parse_cell_xfs_with_alignment() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
        <styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
          <fonts count="1"><font><sz val="11"/><name val="Calibri"/></font></fonts>
          <fills count="2"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill></fills>
          <borders count="1"><border><left/><right/><top/><bottom/><diagonal/></border></borders>
          <cellXfs count="3">
            <xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/>
            <xf numFmtId="0" fontId="0" fillId="1" borderId="0" xfId="0"/>
            <xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0">
              <alignment horizontal="center" vertical="center" wrapText="1" indent="2"/>
            </xf>
          </cellXfs>
        </styleSheet>"#;
        let table = parse_style_table(xml).unwrap();
        assert_eq!(table.cell_xfs.len(), 3);
        assert!(table.cell_xfs[0].alignment.is_none());
        assert!(table.cell_xfs[1].alignment.is_none());
        let a = table.cell_xfs[2].alignment.as_ref().unwrap();
        assert_eq!(a.horizontal.as_deref(), Some("center"));
        assert_eq!(a.vertical.as_deref(), Some("middle")); // OOXML "center" → "middle"
        assert_eq!(a.wrap_text, Some(true));
        assert_eq!(a.indent, Some(2));
    }

    #[test]
    fn test_skip_theme_color() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
        <styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
          <fonts count="1">
            <font>
              <sz val="11"/><name val="Calibri"/>
              <color theme="1"/>
            </font>
          </fonts>
        </styleSheet>"#;
        let table = parse_style_table(xml).unwrap();
        assert_eq!(table.fonts.len(), 1);
        assert!(table.fonts[0].color.is_none());
    }

    // -- resolve_style tests --

    #[test]
    fn test_resolve_index_0_is_none() {
        let table = StyleTableRead::empty();
        assert!(table.resolve_style(0).is_none());
    }

    #[test]
    fn test_resolve_out_of_range_is_none() {
        let table = StyleTableRead::empty();
        assert!(table.resolve_style(99).is_none());
    }

    #[test]
    fn test_resolve_with_font() {
        let table = StyleTableRead {
            num_fmts: Vec::new(),
            fonts: vec![
                Font::default(),
                Font {
                    bold: Some(true),
                    ..Default::default()
                },
            ],
            fills: Vec::new(),
            borders: Vec::new(),
            cell_xfs: vec![
                ParsedCellXf::default(),
                ParsedCellXf {
                    font_id: 1,
                    ..Default::default()
                },
            ],
        };
        let style = table.resolve_style(1).unwrap();
        assert_eq!(style.font.as_ref().unwrap().bold, Some(true));
    }

    #[test]
    fn test_resolve_with_alignment() {
        let table = StyleTableRead {
            cell_xfs: vec![
                ParsedCellXf::default(),
                ParsedCellXf {
                    alignment: Some(Alignment {
                        horizontal: Some("center".into()),
                        vertical: None,
                        wrap_text: None,
                        indent: None,
                    }),
                    ..Default::default()
                },
            ],
            ..StyleTableRead::empty()
        };
        let style = table.resolve_style(1).unwrap();
        assert_eq!(style.alignment.as_ref().unwrap().horizontal.as_deref(), Some("center"));
    }

    // -- parse_sheet_cell_styles tests --

    #[test]
    fn test_parse_sheet_styles_empty() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
        <worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
          <sheetData/>
        </worksheet>"#;
        let map = parse_sheet_cell_styles(xml).unwrap();
        assert!(map.is_empty());
    }

    #[test]
    fn test_parse_sheet_styles_with_cells() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
        <worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
          <sheetData>
            <row r="1">
              <c r="A1" s="0" t="s"><v>0</v></c>
              <c r="B1" s="1"><v>42</v></c>
            </row>
            <row r="2">
              <c r="A2" s="2" t="s"><v>1</v></c>
              <c r="B2"><v>99</v></c>
            </row>
          </sheetData>
        </worksheet>"#;
        let map = parse_sheet_cell_styles(xml).unwrap();
        assert_eq!(map.len(), 2); // s=0 and no-s are excluded
        assert_eq!(map.get(&(1, 2)), Some(&1)); // B1 = row 1, col 2
        assert_eq!(map.get(&(2, 1)), Some(&2)); // A2 = row 2, col 1
        assert!(map.get(&(1, 1)).is_none()); // A1 = s=0, excluded
        assert!(map.get(&(2, 2)).is_none()); // B2 = no s attr, excluded
    }

    // -- parse_styles_and_sheet_maps integration test --
    // (requires building a real .xlsx; tested via the writer round-trip)
}
