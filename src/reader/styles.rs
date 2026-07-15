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
//! # Limitations (v0.3.0, partially addressed)
//! - **Theme colors** (`<color theme="N"/>`)   → resolved via ThemeColorScheme.
//! - **Gradient fills** (`<gradientFill>`)      → resolved in v0.12.0.
//! - **Diagonal borders** (diagonal/diagonalUp/diagonalDown) → resolved in v0.12.0.
//! - **cellStyleXfs inheritance** (xfId)        → ignored; cellXf is used
//!   directly.  The `applyX` flags *are* parsed and honored: when `applyFont="0"`
//!   (or any `applyX="0"`), the corresponding sub-field resolves to `None`
//!   (the caller inherits the Normal default).  cellStyleXfs parent values
//!   are *not* inherited — `applyX="0"` means "Normal for this field".
//! - **Built-in numFmt IDs** (0-49) resolve via a static table
//!   ([`BUILTIN_NUMFMTS`]); custom IDs (≥164) via the `<numFmts>` element.
//! - **cellStyleXfs**, **dxfs**, **tableStyles**, **extLst** → skipped.

use std::collections::HashMap;
use std::io::Read;

use quick_xml::events::{attributes::Attribute, Event};

/// Maximum decompressed bytes per zip entry (16 MiB). Used to prevent zip-bomb OOM.
const MAX_ENTRY_BYTES: u64 = 16 * 1024 * 1024;
const MAX_EVENTS: usize = 5_000_000;
use quick_xml::Reader as XmlReader;

use crate::error::ExcelrsError;
use crate::model::color::{Color, ThemeColorScheme};
use crate::model::style::{Alignment, Border, BorderStyle, Fill, Font, GradientStop, Style};
use crate::types;

/// Per-sheet cell-to-style-index map: (1-indexed row, col) → cellXfs index.
pub type SheetStyleMap = HashMap<(u32, u32), u32>;

/// Built-in numFmt codes 0-49 from ECMA-376 §18.8.30.
/// A common subset covering the most-used Excel format codes.
/// Custom format codes (≥164) are parsed from `<numFmts>` at runtime.
const BUILTIN_NUMFMTS: &[(u32, &str)] = &[
    (0, "General"),
    (1, "0"),
    (2, "0.00"),
    (3, "#,##0"),
    (4, "#,##0.00"),
    (9, "0%"),
    (10, "0.00%"),
    (11, "0.00E+00"),
    (12, "# ?/?"),
    (13, "# ??/??"),
    (14, "m/d/yyyy"),
    (15, "d-mmm-yy"),
    (16, "d-mmm"),
    (17, "mmm-yy"),
    (18, "h:mm AM/PM"),
    (19, "h:mm:ss AM/PM"),
    (20, "h:mm"),
    (21, "h:mm:ss"),
    (22, "m/d/yyyy h:mm"),
];

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
///
/// `apply_*` fields are `Option<bool>` because the attribute may be absent from
/// the XML (OOXML default is "1" when omitted).  `resolve_style` uses
/// `unwrap_or(true)` to implement the OOXML default.
#[derive(Debug, Clone, Default)]
pub struct ParsedCellXf {
    pub num_fmt_id: u32,
    pub font_id: u32,
    pub fill_id: u32,
    pub border_id: u32,
    pub apply_number_format: Option<bool>,
    pub apply_font: Option<bool>,
    pub apply_fill: Option<bool>,
    pub apply_border: Option<bool>,
    pub apply_alignment: Option<bool>,
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
    ///
    /// Built-in numFmt IDs (0-49) are resolved from a static table
    /// ([`BUILTIN_NUMFMTS`]); custom IDs (≥164) are looked up from
    /// `self.num_fmts`.  Each sub-field is gated by its `applyX` flag:
    /// when `applyX` is `0` (or missing, defaulting to true) the field
    /// resolves to `None` — the caller inherits the Normal default.
    pub fn resolve_style(&self, xf_index: u32) -> Option<Style> {
        if xf_index == 0 {
            return None;
        }
        let xf = self.cell_xfs.get(xf_index as usize)?;

        // numFmt: applyNumberFormat gate, then built-in table or custom lookup
        let num_fmt = if xf.apply_number_format.unwrap_or(true) && xf.num_fmt_id != 0 {
            if xf.num_fmt_id < 50 {
                BUILTIN_NUMFMTS
                    .iter()
                    .find(|(id, _)| *id == xf.num_fmt_id)
                    .map(|(_, code)| code.to_string())
            } else {
                self.num_fmts
                    .iter()
                    .find(|(id, _)| *id == xf.num_fmt_id)
                    .map(|(_, code)| code.clone())
            }
        } else {
            None
        };

        let font = if xf.apply_font.unwrap_or(true) && xf.font_id != 0 {
            self.fonts.get(xf.font_id as usize).cloned()
        } else {
            None
        };

        let fill = if xf.apply_fill.unwrap_or(true) && xf.fill_id != 0 {
            self.fills.get(xf.fill_id as usize).cloned()
        } else {
            None
        };

        let border = if xf.apply_border.unwrap_or(true) && xf.border_id != 0 {
            self.borders.get(xf.border_id as usize).cloned()
        } else {
            None
        };

        let alignment = if xf.apply_alignment.unwrap_or(true) {
            xf.alignment.clone()
        } else {
            None
        };

        let style = Style {
            font,
            fill,
            border,
            alignment,
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
    let entry = archive
        .by_name(path)
        .map_err(|_| ExcelrsError::Parse(format!("Missing zip entry: '{path}'")))?;
    let mut buf = Vec::new();
    entry.take(MAX_ENTRY_BYTES).read_to_end(&mut buf)?;
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

/// Resolve a theme, indexed, or rgb color attribute to a `Color` (ARGB plus the
/// originating theme reference when present).
///
/// Priority: `theme` → `indexed` → `rgb`. Returns `None` when none are present.
fn parse_color(attrs: &[Attribute], scheme: &ThemeColorScheme) -> Option<Color> {
    // Prefer theme, then indexed, then rgb
    if let Some(theme_str) = str_attr(attrs, b"theme") {
        if let Ok(index) = theme_str.parse::<usize>() {
            let tint = f64_attr(attrs, b"tint");
            if let Some(rgb) = scheme.resolve_theme(index, tint) {
                return Some(Color {
                    rgb,
                    theme: Some(index as u8),
                    tint,
                });
            }
        }
    }
    if let Some(idx_str) = str_attr(attrs, b"indexed") {
        if let Ok(index) = idx_str.parse::<usize>() {
            if let Some(rgb) = scheme.resolve_indexed(index) {
                return Some(Color { rgb, theme: None, tint: None });
            }
        }
    }
    // rgb attr — fallback
    if let Some(rgb) = str_attr(attrs, b"rgb") {
        if !rgb.is_empty() && rgb.chars().all(|c| c.is_ascii_hexdigit()) {
            return Some(Color {
                rgb: rgb.to_uppercase(),
                theme: None,
                tint: None,
            });
        }
        return None;
    }
    None
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
pub fn parse_style_table(data: &[u8], scheme: &ThemeColorScheme) -> Result<StyleTableRead, ExcelrsError> {
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
    let mut in_gradient_fill = false;
    let mut current_gradient_stop: Option<GradientStop> = None;
    let mut border: Option<Border> = None;
    let mut border_side: Option<(String, Option<BorderStyle>)> = None; // (side_name, style)
                                                                       // border building: after parsing all sides, reconstruct
    let mut border_top: Option<BorderStyle> = None;
    let mut border_right: Option<BorderStyle> = None;
    let mut border_bottom: Option<BorderStyle> = None;
    let mut border_left: Option<BorderStyle> = None;
    let mut border_diagonal: Option<BorderStyle> = None;

    // Track the index of the current xf in cell_xfs (for alignment children).
    // The ParsedCellXf is pushed immediately even before alignment is parsed.
    let mut xf_idx: Option<usize> = None;
    let mut events: u64 = 0;
    loop {
        events += 1;
        if events > MAX_EVENTS as u64 {
            break;
        }
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
                        if let Some(c) = parse_color(&attrs, scheme) {
                            if let Some(ref mut f) = font {
                                f.color = Some(c.rgb);
                                f.color_theme = c.theme;
                                f.color_tint = c.tint;
                            }
                        }
                    }

                    // color inside border side
                    b"color" if border_side.is_some() => {
                        let attrs: Vec<_> = e.attributes().filter_map(|a| a.ok()).collect();
                        if let Some(c) = parse_color(&attrs, scheme) {
                            if let Some((_, Some(ref mut style))) = border_side {
                                style.color = Some(c.rgb);
                                style.color_theme = c.theme;
                                style.color_tint = c.tint;
                            }
                        }
                    }

                    // color inside fill
                    b"fgColor" if fill.is_some() || in_pattern_fill => {
                        let attrs: Vec<_> = e.attributes().filter_map(|a| a.ok()).collect();
                        if let Some(c) = parse_color(&attrs, scheme) {
                            if let Some(ref mut f) = fill {
                                f.foreground = Some(c.rgb);
                                f.foreground_theme = c.theme;
                                f.foreground_tint = c.tint;
                            }
                        }
                    }
                    b"bgColor" if fill.is_some() || in_pattern_fill => {
                        let attrs: Vec<_> = e.attributes().filter_map(|a| a.ok()).collect();
                        if let Some(c) = parse_color(&attrs, scheme) {
                            if let Some(ref mut f) = fill {
                                f.background = Some(c.rgb);
                                f.background_theme = c.theme;
                                f.background_tint = c.tint;
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
                    b"gradientFill" if matches!(section, StyleSection::Fills) => {
                        in_gradient_fill = true;
                        let attrs: Vec<_> = e.attributes().filter_map(|a| a.ok()).collect();
                        if let Some(ref mut f) = fill {
                            f.kind = "gradient".to_owned();
                            f.gradient_type = str_attr(&attrs, b"type").map(|s| s.to_owned());
                            f.gradient_degree = str_attr(&attrs, b"degree").and_then(|v| v.parse::<f64>().ok());
                            f.gradient_left = str_attr(&attrs, b"left").and_then(|v| v.parse::<f64>().ok());
                            f.gradient_right = str_attr(&attrs, b"right").and_then(|v| v.parse::<f64>().ok());
                            f.gradient_top = str_attr(&attrs, b"top").and_then(|v| v.parse::<f64>().ok());
                            f.gradient_bottom = str_attr(&attrs, b"bottom").and_then(|v| v.parse::<f64>().ok());
                        }
                    }
                    b"stop" if in_gradient_fill => {
                        let attrs: Vec<_> = e.attributes().filter_map(|a| a.ok()).collect();
                        let position = str_attr(&attrs, b"position")
                            .and_then(|v| v.parse::<f64>().ok())
                            .unwrap_or(0.0);
                        if let Some(c) = str_attr(&attrs, b"color") {
                            // Attribute form: <stop position="0" color="FFFF0000"/>
                            if let Some(ref mut f) = fill {
                                f.gradient_stops.get_or_insert_with(Vec::new).push(GradientStop {
                                    color: c.to_owned(),
                                    position,
                                });
                            }
                            current_gradient_stop = None;
                        } else {
                            // Child-element form: <stop position="0"><color rgb="..."/></stop>
                            current_gradient_stop = Some(GradientStop {
                                color: String::new(),
                                position,
                            });
                        }
                    }
                    b"color" if in_gradient_fill && current_gradient_stop.is_some() => {
                        let attrs: Vec<_> = e.attributes().filter_map(|a| a.ok()).collect();
                        if let Some(c) = parse_color(&attrs, scheme) {
                            if let Some(ref mut stop) = current_gradient_stop {
                                stop.color = c.rgb;
                            }
                        }
                    }

                    // -- border --
                    b"border" if matches!(section, StyleSection::Borders) => {
                        let attrs: Vec<_> = e.attributes().filter_map(|a| a.ok()).collect();
                        let diagonal_up = bool_attr(&attrs, b"diagonalUp");
                        let diagonal_down = bool_attr(&attrs, b"diagonalDown");
                        border = Some(Border {
                            diagonal_up,
                            diagonal_down,
                            ..Default::default()
                        });
                        border_top = None;
                        border_right = None;
                        border_bottom = None;
                        border_left = None;
                        border_diagonal = None;
                    }
                    b"left" | b"right" | b"top" | b"bottom" | b"diagonal" if border.is_some() => {
                        let side_name = std::str::from_utf8(&tag).unwrap_or("");
                        let attrs: Vec<_> = e.attributes().filter_map(|a| a.ok()).collect();
                        let style_val = str_attr(&attrs, b"style");
                        let bs = style_val.map(|s| BorderStyle {
                            style: s.to_owned(),
                            color: None,
                            ..Default::default()
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
                            apply_number_format: bool_attr(&attrs, b"applyNumberFormat"),
                            apply_font: bool_attr(&attrs, b"applyFont"),
                            apply_fill: bool_attr(&attrs, b"applyFill"),
                            apply_border: bool_attr(&attrs, b"applyBorder"),
                            apply_alignment: bool_attr(&attrs, b"applyAlignment"),
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
                        in_gradient_fill = false;
                    }
                    b"gradientFill" => {
                        in_gradient_fill = false;
                        current_gradient_stop = None;
                    }
                    b"stop" if in_gradient_fill => {
                        if let Some(stop) = current_gradient_stop.take() {
                            if let Some(ref mut f) = fill {
                                f.gradient_stops.get_or_insert_with(Vec::new).push(stop);
                            }
                        }
                    }

                    b"border" => {
                        if let Some(mut b) = border.take() {
                            b.top = border_top.take();
                            b.right = border_right.take();
                            b.bottom = border_bottom.take();
                            b.left = border_left.take();
                            b.diagonal = border_diagonal.take();
                            borders.push(b);
                        }
                        border_side = None;
                    }
                    b"left" | b"right" | b"top" | b"bottom" | b"diagonal" => {
                        if let Some((side_name, style)) = border_side.take() {
                            match side_name.as_str() {
                                "top" => border_top = style,
                                "right" => border_right = style,
                                "bottom" => border_bottom = style,
                                "left" => border_left = style,
                                "diagonal" => border_diagonal = style,
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

    let mut events: u64 = 0;
    loop {
        events += 1;
        if events > MAX_EVENTS as u64 {
            break;
        }
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
) -> Result<(StyleTableRead, Vec<SheetStyleMap>, ThemeColorScheme), ExcelrsError> {
    use std::io::Cursor;
    let cursor = Cursor::new(data);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|e| ExcelrsError::Zip(e.to_string()))?;

    // Resolve theme color scheme from xl/theme/theme1.xml (optional)
    let scheme = match archive.by_name("xl/theme/theme1.xml") {
        Ok(entry) => {
            let mut data = String::new();
            entry.take(MAX_ENTRY_BYTES).read_to_string(&mut data)?;
            ThemeColorScheme::from_xml(&data).unwrap_or_default()
        }
        Err(_) => ThemeColorScheme::default(),
    };

    // Parse xl/styles.xml (optional — some files lack it)
    let style_table = if let Ok(styles_bytes) = read_entry(&mut archive, "xl/styles.xml") {
        parse_style_table(&styles_bytes, &scheme)?
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

    Ok((style_table, sheet_style_maps, scheme))
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
        let table = parse_style_table(xml, &ThemeColorScheme::default()).unwrap();
        assert!(table.num_fmts.is_empty());
        assert!(table.fonts.is_empty());
        assert!(table.fills.is_empty());
        assert!(table.borders.is_empty());
        assert!(table.cell_xfs.is_empty());
    }

    #[test]
    fn test_parse_gradient_fill_linear() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
        <styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
          <fills count="3">
            <fill><patternFill patternType="none"/></fill>
            <fill><patternFill patternType="gray125"/></fill>
            <fill>
              <gradientFill type="linear" degree="45">
                <stop position="0" color="FFFF0000"/>
                <stop position="1" color="FF0000FF"/>
              </gradientFill>
            </fill>
          </fills>
          <fonts count="1"><font><sz val="11"/><name val="Calibri"/></font></fonts>
          <borders count="1"><border><left/><right/><top/><bottom/><diagonal/></border></borders>
          <cellXfs count="2">
            <xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/>
            <xf numFmtId="0" fontId="0" fillId="2" borderId="0" xfId="0"/>
          </cellXfs>
        </styleSheet>"#;
        let table = parse_style_table(xml, &ThemeColorScheme::default()).unwrap();
        let fill = &table.fills[2];
        assert_eq!(fill.kind, "gradient");
        assert_eq!(fill.gradient_type.as_deref(), Some("linear"));
        assert_eq!(fill.gradient_degree, Some(45.0));
        assert!(fill.gradient_left.is_none());
        assert!(fill.gradient_right.is_none());
        let stops = fill.gradient_stops.as_ref().unwrap();
        assert_eq!(stops.len(), 2);
        assert_eq!(stops[0].color, "FFFF0000");
        assert_eq!(stops[0].position, 0.0);
        assert_eq!(stops[1].color, "FF0000FF");
        assert_eq!(stops[1].position, 1.0);
    }

    #[test]
    fn test_parse_gradient_fill_path() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
        <styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
          <fills count="3">
            <fill><patternFill patternType="none"/></fill>
            <fill><patternFill patternType="gray125"/></fill>
            <fill>
              <gradientFill type="path" left="0.0" right="1.0" top="0.5" bottom="1.0">
                <stop position="0" color="FF00FF00"/>
              </gradientFill>
            </fill>
          </fills>
          <fonts count="1"><font><sz val="11"/><name val="Calibri"/></font></fonts>
          <borders count="1"><border><left/><right/><top/><bottom/><diagonal/></border></borders>
          <cellXfs count="2">
            <xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/>
            <xf numFmtId="0" fontId="0" fillId="2" borderId="0" xfId="0"/>
          </cellXfs>
        </styleSheet>"#;
        let table = parse_style_table(xml, &ThemeColorScheme::default()).unwrap();
        let fill = &table.fills[2];
        assert_eq!(fill.kind, "gradient");
        assert_eq!(fill.gradient_type.as_deref(), Some("path"));
        assert_eq!(fill.gradient_left, Some(0.0));
        assert_eq!(fill.gradient_right, Some(1.0));
        assert_eq!(fill.gradient_top, Some(0.5));
        assert_eq!(fill.gradient_bottom, Some(1.0));
        let stops = fill.gradient_stops.as_ref().unwrap();
        assert_eq!(stops.len(), 1);
        assert_eq!(stops[0].color, "FF00FF00");
        assert_eq!(stops[0].position, 0.0);
    }

    #[test]
    fn test_parse_color_returns_theme_index() {
        use quick_xml::events::Event;
        use quick_xml::Reader;
        let mut r = Reader::from_str(r#"<color theme="4" tint="-0.5"/>"#);
        let e = match r.read_event() {
            Ok(Event::Empty(e)) => e,
            Ok(_) => panic!("expected empty element"),
            Err(err) => panic!("xml error: {err}"),
        };
        let attrs: Vec<_> = e.attributes().collect::<Result<_, _>>().unwrap();
        let scheme = ThemeColorScheme::default();
        let c = parse_color(&attrs, &scheme).unwrap();
        assert_eq!(c.theme, Some(4));
        assert_eq!(c.tint, Some(-0.5));
        assert!(!c.rgb.is_empty());
    }

    #[test]
    fn test_parse_gradient_fill_theme_stop() {
        // Regression for finding #2: theme/indexed gradient stops must resolve,
        // not parse to an empty string.
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
        <styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
          <fills count="3">
            <fill><patternFill patternType="none"/></fill>
            <fill><patternFill patternType="gray125"/></fill>
            <fill>
              <gradientFill type="linear" degree="45">
                <stop position="0"><color theme="4"/></stop>
                <stop position="1" color="FFFF0000"/>
              </gradientFill>
            </fill>
          </fills>
          <fonts count="1"><font><sz val="11"/><name val="Calibri"/></font></fonts>
          <borders count="1"><border><left/><right/><top/><bottom/><diagonal/></border></borders>
          <cellXfs count="2">
            <xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/>
            <xf numFmtId="0" fontId="0" fillId="2" borderId="0" xfId="0"/>
          </cellXfs>
        </styleSheet>"#;
        let table = parse_style_table(xml, &ThemeColorScheme::default()).unwrap();
        let fill = &table.fills[2];
        assert_eq!(fill.kind, "gradient");
        let stops = fill.gradient_stops.as_ref().unwrap();
        assert_eq!(stops.len(), 2);
        // Child-element theme color resolved (was "" before fix)
        assert_eq!(stops[0].color, "FF4F81BD");
        assert_eq!(stops[0].position, 0.0);
        // Attribute-form color still works
        assert_eq!(stops[1].color, "FFFF0000");
        assert_eq!(stops[1].position, 1.0);
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
        let table = parse_style_table(xml, &ThemeColorScheme::default()).unwrap();
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
        let table = parse_style_table(xml, &ThemeColorScheme::default()).unwrap();
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
        let table = parse_style_table(xml, &ThemeColorScheme::default()).unwrap();
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
    fn test_parse_border_diagonal() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
        <styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
          <borders count="2">
            <border><left/><right/><top/><bottom/><diagonal/></border>
            <border diagonalUp="1" diagonalDown="1">
              <left/>
              <right/>
              <top/>
              <bottom/>
              <diagonal style="thin"><color rgb="FF000000"/></diagonal>
            </border>
          </borders>
        </styleSheet>"#;
        let table = parse_style_table(xml, &ThemeColorScheme::default()).unwrap();
        assert_eq!(table.borders.len(), 2);
        // First border: no diagonal attrs, no diagonal style
        assert!(table.borders[0].diagonal.is_none());
        assert!(table.borders[0].diagonal_up.is_none());
        assert!(table.borders[0].diagonal_down.is_none());
        // Second border: diagonal side + diagonalUp/down
        assert!(table.borders[1].diagonal.is_some());
        assert_eq!(table.borders[1].diagonal.as_ref().unwrap().style, "thin");
        assert_eq!(
            table.borders[1].diagonal.as_ref().unwrap().color.as_deref(),
            Some("FF000000")
        );
        assert_eq!(table.borders[1].diagonal_up, Some(true));
        assert_eq!(table.borders[1].diagonal_down, Some(true));
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
        let table = parse_style_table(xml, &ThemeColorScheme::default()).unwrap();
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
        let table = parse_style_table(xml, &ThemeColorScheme::default()).unwrap();
        assert_eq!(table.cell_xfs.len(), 3);
        assert!(table.cell_xfs[0].alignment.is_none());
        assert!(table.cell_xfs[1].alignment.is_none());
        let a = table.cell_xfs[2].alignment.as_ref().unwrap();
        assert_eq!(a.horizontal.as_deref(), Some("center"));
        assert_eq!(a.vertical.as_deref(), Some("middle")); // OOXML "center" → "middle"
        assert_eq!(a.wrap_text, Some(true));
        assert_eq!(a.indent, Some(2));
    }

    // -- Theme/indexed color resolution tests (v0.6.0) --

    /// B1: font `<color theme="4"/>` resolves via accent1.
    #[test]
    fn test_parse_font_theme_color() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
        <styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
          <fonts count="2">
            <font><sz val="11"/><name val="Calibri"/></font>
            <font><sz val="14"/><name val="Arial"/><color theme="4"/></font>
          </fonts>
        </styleSheet>"#;
        let table = parse_style_table(xml, &ThemeColorScheme::default()).unwrap();
        assert_eq!(table.fonts[0].color, None);
        assert_eq!(table.fonts[1].color.as_deref(), Some("FF4F81BD"));
    }

    /// B2: border `<left><color theme="1"/>` resolves via lt1.
    #[test]
    fn test_parse_border_theme_color() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
        <styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
          <borders count="2">
            <border><left/><right/><top/><bottom/><diagonal/></border>
            <border>
              <left style="thin"><color theme="1"/></left>
              <right/><top/><bottom/><diagonal/>
            </border>
          </borders>
        </styleSheet>"#;
        let table = parse_style_table(xml, &ThemeColorScheme::default()).unwrap();
        assert_eq!(table.borders.len(), 2);
        assert!(table.borders[0].left.is_none());
        let left = table.borders[1].left.as_ref().unwrap();
        assert_eq!(left.style, "thin");
        assert_eq!(left.color.as_deref(), Some("FFFFFFFF"));
    }

    /// B3: fill `<fgColor theme="6"/>` resolves via accent3.
    #[test]
    fn test_parse_fill_fg_theme() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
        <styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
          <fills count="2">
            <fill><patternFill patternType="none"/></fill>
            <fill><patternFill patternType="solid">
              <fgColor theme="6"/>
            </patternFill></fill>
          </fills>
        </styleSheet>"#;
        let table = parse_style_table(xml, &ThemeColorScheme::default()).unwrap();
        assert_eq!(table.fills.len(), 2);
        assert_eq!(table.fills[1].foreground.as_deref(), Some("FF9BBB59"));
    }

    /// B4: fill `<bgColor theme="3"/>` resolves via lt2.
    #[test]
    fn test_parse_fill_bg_theme() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
        <styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
          <fills count="2">
            <fill><patternFill patternType="none"/></fill>
            <fill><patternFill patternType="solid">
              <fgColor theme="4"/>
              <bgColor theme="3"/>
            </patternFill></fill>
          </fills>
        </styleSheet>"#;
        let table = parse_style_table(xml, &ThemeColorScheme::default()).unwrap();
        assert_eq!(table.fills[1].foreground.as_deref(), Some("FF4F81BD"));
        assert_eq!(table.fills[1].background.as_deref(), Some("FFEEECE1"));
    }

    /// B5: font `<color theme="4" tint="-0.5"/>` resolves with tint darken.
    #[test]
    fn test_parse_font_theme_with_tint() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
        <styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
          <fonts count="2">
            <font><sz val="11"/><name val="Calibri"/></font>
            <font><sz val="14"/><name val="Arial"/><color theme="4" tint="-0.5"/></font>
          </fonts>
        </styleSheet>"#;
        let table = parse_style_table(xml, &ThemeColorScheme::default()).unwrap();
        assert_eq!(table.fonts[1].color.as_deref(), Some("FF28415F"));
    }

    /// B6: `<color indexed="8"/>` resolves to system palette entry 8 (black).
    #[test]
    fn test_parse_color_indexed() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
        <styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
          <fonts count="2">
            <font><sz val="11"/><name val="Calibri"/></font>
            <font><sz val="14"/><name val="Arial"/><color indexed="8"/></font>
          </fonts>
        </styleSheet>"#;
        let table = parse_style_table(xml, &ThemeColorScheme::default()).unwrap();
        assert_eq!(table.fonts[1].color.as_deref(), Some("FF000000"));
    }

    /// B7: `<color rgb="..."/>` path unchanged; absent color stays None.
    #[test]
    fn test_parse_no_color_attr_still_none() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
        <styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
          <fonts count="3">
            <font><sz val="11"/><name val="Calibri"/></font>
            <font><sz val="14"/><name val="Arial"/><color rgb="FFFF0000"/></font>
            <font><sz val="10"/><name val="Arial"/><color/></font>
          </fonts>
        </styleSheet>"#;
        let table = parse_style_table(xml, &ThemeColorScheme::default()).unwrap();
        assert_eq!(table.fonts[0].color, None);
        assert_eq!(table.fonts[1].color.as_deref(), Some("FFFF0000"));
        assert_eq!(table.fonts[2].color, None);
    }

    /// B8: REPLACES old test_skip_theme_color — theme="1" NOW resolves.
    #[test]
    fn test_resolve_theme_not_skipped() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
        <styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
          <fonts count="1">
            <font>
              <sz val="11"/><name val="Calibri"/>
              <color theme="1"/>
            </font>
          </fonts>
        </styleSheet>"#;
        let table = parse_style_table(xml, &ThemeColorScheme::default()).unwrap();
        assert_eq!(table.fonts.len(), 1);
        assert!(table.fonts[0].color.is_some());
        assert_eq!(table.fonts[0].color.as_deref(), Some("FFFFFFFF"));
    }

    /// B9: default scheme resolves theme="4" even without real theme1.xml.
    #[test]
    fn test_default_scheme_when_theme1_absent() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
        <styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
          <fonts count="2">
            <font><sz val="11"/><name val="Calibri"/></font>
            <font><sz val="14"/><name val="Arial"/><color theme="4"/></font>
          </fonts>
        </styleSheet>"#;
        let table = parse_style_table(xml, &ThemeColorScheme::default()).unwrap();
        assert_eq!(table.fonts[1].color.as_deref(), Some("FF4F81BD"));
    }

    /// B10: `<color rgb=""/>` must return None, not Some("").
    #[test]
    fn test_parse_color_rgb_empty_returns_none() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
        <styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
          <fonts count="2">
            <font><sz val="11"/><name val="Calibri"/></font>
            <font><sz val="14"/><name val="Arial"/><color rgb=""/></font>
          </fonts>
        </styleSheet>"#;
        let table = parse_style_table(xml, &ThemeColorScheme::default()).unwrap();
        assert_eq!(table.fonts[1].color, None);
    }

    /// B11: `<color rgb="ZZZZZZ"/>` non-hex → None.
    #[test]
    fn test_parse_color_rgb_non_hex_returns_none() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
        <styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
          <fonts count="2">
            <font><sz val="11"/><name val="Calibri"/></font>
            <font><sz val="14"/><name val="Arial"/><color rgb="ZZZZZZ"/></font>
          </fonts>
        </styleSheet>"#;
        let table = parse_style_table(xml, &ThemeColorScheme::default()).unwrap();
        assert_eq!(table.fonts[1].color, None);
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
        assert!(!map.contains_key(&(1, 1))); // A1 = s=0, excluded
        assert!(!map.contains_key(&(2, 2))); // B2 = no s attr, excluded
    }

    // -- Built-in numFmt resolution (Bug 1) --

    fn make_builtin_numfmt_table(numfmt_id: u32) -> StyleTableRead {
        let xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <fonts count="1"><font><sz val="11"/><name val="Calibri"/></font></fonts>
  <fills count="2"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill></fills>
  <borders count="1"><border><left/><right/><top/><bottom/><diagonal/></border></borders>
  <cellXfs count="2">
    <xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/>
    <xf numFmtId="{numfmt_id}" fontId="0" fillId="0" borderId="0" xfId="0"/>
  </cellXfs>
</styleSheet>"#
        );
        parse_style_table(xml.as_bytes(), &ThemeColorScheme::default()).unwrap()
    }

    #[test]
    fn test_resolve_builtin_numfmt_14() {
        let table = make_builtin_numfmt_table(14);
        let style = table.resolve_style(1).unwrap();
        assert_eq!(style.num_fmt.as_deref(), Some("m/d/yyyy"));
    }

    #[test]
    fn test_resolve_builtin_numfmt_9() {
        let table = make_builtin_numfmt_table(9);
        let style = table.resolve_style(1).unwrap();
        assert_eq!(style.num_fmt.as_deref(), Some("0%"));
    }

    #[test]
    fn test_resolve_builtin_numfmt_22() {
        let table = make_builtin_numfmt_table(22);
        let style = table.resolve_style(1).unwrap();
        assert_eq!(style.num_fmt.as_deref(), Some("m/d/yyyy h:mm"));
    }

    #[test]
    fn test_resolve_builtin_numfmt_10() {
        let table = make_builtin_numfmt_table(10);
        let style = table.resolve_style(1).unwrap();
        assert_eq!(style.num_fmt.as_deref(), Some("0.00%"));
    }

    #[test]
    fn test_resolve_numfmt_zero_is_none() {
        let table = make_builtin_numfmt_table(0);
        let style = table.resolve_style(1);
        assert!(style.is_none()); // numFmtId=0 → whole style empty → None
    }

    #[test]
    fn test_resolve_unknown_custom_numfmt_is_none() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
        <styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
          <fonts count="1"><font><sz val="11"/><name val="Calibri"/></font></fonts>
          <fills count="2"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill></fills>
          <borders count="1"><border><left/><right/><top/><bottom/><diagonal/></border></borders>
          <cellXfs count="2">
            <xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/>
            <xf numFmtId="999" fontId="0" fillId="0" borderId="0" xfId="0"/>
          </cellXfs>
        </styleSheet>"#;
        let table = parse_style_table(xml, &ThemeColorScheme::default()).unwrap();
        let style = table.resolve_style(1);
        assert!(style.is_none()); // 999 doesn't exist → whole style empty → None
    }

    // -- applyX flag tests (Bug 2) --

    #[test]
    fn test_apply_number_format_zero_ignores_numfmt() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
        <styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
          <fonts count="1"><font><sz val="11"/><name val="Calibri"/></font></fonts>
          <fills count="2"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill></fills>
          <borders count="1"><border><left/><right/><top/><bottom/><diagonal/></border></borders>
          <numFmts count="1"><numFmt numFmtId="164" formatCode="0.00%"/></numFmts>
          <cellXfs count="2">
            <xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/>
            <xf numFmtId="164" fontId="0" fillId="0" borderId="0" xfId="0" applyNumberFormat="0"/>
          </cellXfs>
        </styleSheet>"#;
        let table = parse_style_table(xml, &ThemeColorScheme::default()).unwrap();
        // numFmt was the only non-default field; suppressing it leaves
        // an empty style → resolve_style returns None (Normal).
        assert!(
            table.resolve_style(1).is_none(),
            "applyNumberFormat=0 makes the whole style Normal"
        );
    }

    #[test]
    fn test_apply_font_zero_ignores_font() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
        <styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
          <fonts count="2">
            <font><sz val="11"/><name val="Calibri"/></font>
            <font><b/><sz val="14"/></font>
          </fonts>
          <fills count="2"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill></fills>
          <borders count="1"><border><left/><right/><top/><bottom/><diagonal/></border></borders>
          <cellXfs count="2">
            <xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/>
            <xf numFmtId="0" fontId="1" fillId="0" borderId="0" xfId="0" applyFont="0"/>
          </cellXfs>
        </styleSheet>"#;
        let table = parse_style_table(xml, &ThemeColorScheme::default()).unwrap();
        // Font was the only non-default field; suppressing it → Normal.
        assert!(
            table.resolve_style(1).is_none(),
            "applyFont=0 makes the whole style Normal"
        );
    }

    #[test]
    fn test_apply_fill_zero_ignores_fill() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
        <styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
          <fonts count="1"><font><sz val="11"/><name val="Calibri"/></font></fonts>
          <fills count="2">
            <fill><patternFill patternType="none"/></fill>
            <fill><patternFill patternType="solid"><fgColor rgb="FFFF0000"/></patternFill></fill>
          </fills>
          <borders count="1"><border><left/><right/><top/><bottom/><diagonal/></border></borders>
          <cellXfs count="2">
            <xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/>
            <xf numFmtId="0" fontId="0" fillId="1" borderId="0" xfId="0" applyFill="0"/>
          </cellXfs>
        </styleSheet>"#;
        let table = parse_style_table(xml, &ThemeColorScheme::default()).unwrap();
        // Fill was the only non-default field; suppressing it → Normal.
        assert!(
            table.resolve_style(1).is_none(),
            "applyFill=0 makes the whole style Normal"
        );
    }

    #[test]
    fn test_apply_border_zero_ignores_border() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
        <styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
          <fonts count="1"><font><sz val="11"/><name val="Calibri"/></font></fonts>
          <fills count="2"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill></fills>
          <borders count="2">
            <border><left/><right/><top/><bottom/><diagonal/></border>
            <border><top style="thin"><color rgb="FF000000"/></top></border>
          </borders>
          <cellXfs count="2">
            <xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/>
            <xf numFmtId="0" fontId="0" fillId="0" borderId="1" xfId="0" applyBorder="0"/>
          </cellXfs>
        </styleSheet>"#;
        let table = parse_style_table(xml, &ThemeColorScheme::default()).unwrap();
        // Border was the only non-default field; suppressing it leaves an
        // empty style → resolve_style returns None (Normal).
        assert!(
            table.resolve_style(1).is_none(),
            "applyBorder=0 makes the whole style Normal"
        );
    }

    #[test]
    fn test_apply_alignment_zero_ignores_alignment() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
        <styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
          <fonts count="1"><font><sz val="11"/><name val="Calibri"/></font></fonts>
          <fills count="2"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill></fills>
          <borders count="1"><border><left/><right/><top/><bottom/><diagonal/></border></borders>
          <cellXfs count="2">
            <xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/>
            <xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0" applyAlignment="0">
              <alignment horizontal="center" vertical="center"/>
            </xf>
          </cellXfs>
        </styleSheet>"#;
        let table = parse_style_table(xml, &ThemeColorScheme::default()).unwrap();
        // Alignment was the only non-default field; suppressing it leaves an
        // empty style → resolve_style returns None (Normal).
        assert!(
            table.resolve_style(1).is_none(),
            "applyAlignment=0 makes the whole style Normal"
        );
    }

    /// applyFont=0 with a persistent fill: font is suppressed but fill is kept.
    /// This proves the applyX mechanism works without collapsing the whole style.
    #[test]
    fn test_apply_font_zero_keeps_other_fields() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
        <styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
          <fonts count="2">
            <font><sz val="11"/><name val="Calibri"/></font>
            <font><b/><sz val="14"/></font>
          </fonts>
          <fills count="2">
            <fill><patternFill patternType="none"/></fill>
            <fill><patternFill patternType="solid"><fgColor rgb="FF0000FF"/></patternFill></fill>
          </fills>
          <borders count="1"><border><left/><right/><top/><bottom/><diagonal/></border></borders>
          <cellXfs count="2">
            <xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/>
            <xf numFmtId="0" fontId="1" fillId="1" borderId="0" xfId="0" applyFont="0"/>
          </cellXfs>
        </styleSheet>"#;
        let table = parse_style_table(xml, &ThemeColorScheme::default()).unwrap();
        let style = table.resolve_style(1).unwrap();
        // applyFont=0 → font suppressed
        assert!(style.font.is_none(), "applyFont=0 should suppress font");
        // Fill is still applied independently
        assert!(style.fill.is_some(), "fill should survive applyFont=0");
        assert_eq!(style.fill.as_ref().unwrap().foreground.as_deref(), Some("FF0000FF"));
    }

    /// No applyX attributes: defaults to true for all fields.
    #[test]
    fn test_apply_x_default_is_true() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
        <styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
          <fonts count="2">
            <font><sz val="11"/><name val="Calibri"/></font>
            <font><b/><sz val="14"/></font>
          </fonts>
          <fills count="2"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill></fills>
          <borders count="1"><border><left/><right/><top/><bottom/><diagonal/></border></borders>
          <cellXfs count="2">
            <xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/>
            <xf numFmtId="14" fontId="1" fillId="0" borderId="0" xfId="0"/>
          </cellXfs>
        </styleSheet>"#;
        let table = parse_style_table(xml, &ThemeColorScheme::default()).unwrap();
        let style = table.resolve_style(1).unwrap();
        // No applyX attrs → defaults are true → both fields present
        assert_eq!(
            style.num_fmt.as_deref(),
            Some("m/d/yyyy"),
            "built-in 14 resolves with no applyNumberFormat attr"
        );
        assert!(style.font.is_some(), "fontId=1 resolves with no applyFont attr");
        assert_eq!(style.font.as_ref().unwrap().bold, Some(true));
    }

    // -- C-tests: parse_styles_and_sheet_maps with real zip archives --

    /// C1: theme1.xml present with custom accent1.
    /// Font using <color theme="4"/> resolves via the custom scheme.
    #[test]
    fn test_parse_styles_reads_theme1() {
        use std::io::Write;

        let mut buf = Vec::new();
        {
            let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            let options: zip::write::FileOptions<'_, ()> =
                zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

            zip.start_file("xl/styles.xml", options).unwrap();
            write!(
                zip,
                r##"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <fonts count="1">
    <font><sz val="11"/><name val="Calibri"/><color theme="4"/></font>
  </fonts>
</styleSheet>"##
            )
            .unwrap();

            zip.start_file("xl/theme/theme1.xml", options).unwrap();
            write!(
                zip,
                r##"<a:clrScheme name="Custom">
  <a:dk1><a:srgbClr val="000000"/></a:dk1>
  <a:lt1><a:srgbClr val="FFFFFF"/></a:lt1>
  <a:dk2><a:srgbClr val="1F497D"/></a:dk2>
  <a:lt2><a:srgbClr val="EEECE1"/></a:lt2>
  <a:accent1><a:srgbClr val="123456"/></a:accent1>
  <a:accent2><a:srgbClr val="C0504D"/></a:accent2>
  <a:accent3><a:srgbClr val="9BBB59"/></a:accent3>
  <a:accent4><a:srgbClr val="F79646"/></a:accent4>
  <a:accent5><a:srgbClr val="8064A2"/></a:accent5>
  <a:accent6><a:srgbClr val="4BACC6"/></a:accent6>
  <a:hlink><a:srgbClr val="0000FF"/></a:hlink>
  <a:folHlink><a:srgbClr val="800080"/></a:folHlink>
</a:clrScheme>"##
            )
            .unwrap();

            zip.finish().unwrap();
        }

        let (table, _, _) = parse_styles_and_sheet_maps(&buf, 0).unwrap();
        assert_eq!(table.fonts.len(), 1);
        // Custom accent1="123456" → resolved with FF prefix
        assert_eq!(
            table.fonts[0].color.as_deref(),
            Some("FF123456"),
            "theme=\"4\" should resolve via custom theme1.xml accent1"
        );
    }

    /// C2: theme1.xml absent → default accent1 is used.
    #[test]
    fn test_parse_styles_theme1_absent_falls_back() {
        use std::io::Write;

        let mut buf = Vec::new();
        {
            let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            let options: zip::write::FileOptions<'_, ()> =
                zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

            // Only styles.xml — no theme1.xml
            zip.start_file("xl/styles.xml", options).unwrap();
            write!(
                zip,
                r##"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <fonts count="1">
    <font><sz val="11"/><name val="Calibri"/><color theme="4"/></font>
  </fonts>
</styleSheet>"##
            )
            .unwrap();

            zip.finish().unwrap();
        }

        let (table, _, _) = parse_styles_and_sheet_maps(&buf, 0).unwrap();
        assert_eq!(table.fonts.len(), 1);
        // Default accent1 = "4F81BD" → "FF4F81BD"
        assert_eq!(
            table.fonts[0].color.as_deref(),
            Some("FF4F81BD"),
            "missing theme1.xml should fall back to default accent1"
        );
    }
}
