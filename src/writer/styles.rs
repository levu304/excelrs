//! Style dedup + `xl/styles.xml` emission (spec v0.2.0, §9.2, ADR-27).
//!
//! Two responsibilities:
//! 1. **`build_style_table`** — collect unique `Font`/`Fill`/`Border`/`numFmt`
//!    values from a list of cell-level styles, dedup via `BTreeMap`-keyed
//!    canonical JSON strings, and return a [`StyleTable`] with stable indices.
//! 2. **`emit_styles_xml`** — write a `StyleTable` into the OOXML
//!    `xl/styles.xml` format using the existing `quick-xml` escaping.

use std::collections::BTreeMap;
use std::io::Write;

use quick_xml::escape::escape;

use crate::error::ExcelrsError;
use crate::model::style::{Alignment, Border, Fill, Font, Style};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Deduplicated sub-tables plus the cell-level style-index table.
///
/// Every sub-table has index 0 as the "Normal" entry (empty/font Calibri 11).
/// Alignment index 0 is the default (no explicit alignment).
pub struct StyleTable {
    pub fonts: Vec<Font>,
    pub fills: Vec<Fill>,
    pub borders: Vec<Border>,
    pub num_fmts: Vec<(u32, String)>,
    /// Deduplicated alignment entries. Index 0 is the default (None).
    pub alignments: Vec<Alignment>,
    /// Unique cell-level formats (cellXfs) — the OOXML `<cellXfs>` table.
    /// Index 0 is always Normal.
    pub cell_xfs: Vec<CellXf>,
    /// Per-cell mapping into `cell_xfs`. Length equals the input style count.
    /// `cell_indices[i]` = the unique cellXfs index for the i-th input cell.
    pub cell_indices: Vec<u32>,
}

/// One cell level format (XF) record — indices into the sub-tables above.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CellXf {
    /// Custom numFmt ID (≥164) or 0 for General / no custom format.
    pub num_fmt_id: u32,
    /// Index into [`StyleTable::fonts`].
    pub font_id: u32,
    /// Index into [`StyleTable::fills`].
    pub fill_id: u32,
    /// Index into [`StyleTable::borders`].
    pub border_id: u32,
    /// Index into [`StyleTable::alignments`]. 0 means default (no alignment).
    pub alignment_id: u32,
}

// ---------------------------------------------------------------------------
// Dedup
// ---------------------------------------------------------------------------

/// Check whether a style represents Normal (no styling at all).
pub fn is_normal(style: &Option<Style>) -> bool {
    match style {
        None => true,
        Some(s) => s.is_empty(),
    }
}

/// Build a deterministic canonical JSON key for `serde::Serialize` types.
/// Used as the `BTreeMap` key for dedup (ADR-27).
fn canonical_key<T: serde::Serialize>(value: &T) -> String {
    // serde_json::to_string preserves struct-field declaration order
    // (stable as long as the struct definition doesn't change).
    serde_json::to_string(value).unwrap_or_default()
}

/// Walk a list of cell-level styles and build a deduplicated [`StyleTable`].
///
/// - `None` / empty style → Normal (all indices in `cell_xfs` = 0).
/// - Matching sub-values (same font, same fill, etc.) share one sub-table slot.
/// - `cell_indices[i]` maps the i-th input cell to its unique `cellXfs` index.
pub fn build_style_table(styles: &[Option<Style>]) -> StyleTable {
    // Sub-table dedup maps: canonical key → index
    let mut font_map: BTreeMap<String, u32> = BTreeMap::new();
    let mut fill_map: BTreeMap<String, u32> = BTreeMap::new();
    let mut border_map: BTreeMap<String, u32> = BTreeMap::new();
    let mut numfmt_map: BTreeMap<String, u32> = BTreeMap::new();
    let mut alignment_map: BTreeMap<String, u32> = BTreeMap::new();

    let mut fonts: Vec<Font> = Vec::new();
    let mut fills: Vec<Fill> = Vec::new();
    let mut borders: Vec<Border> = Vec::new();
    let mut num_fmts: Vec<(u32, String)> = Vec::new();
    let mut alignments: Vec<Alignment> = Vec::new();

    // Always seed index 0 with Normal defaults
    let normal_font = Font::default();
    let normal_fill = Fill::default();
    let normal_border = Border::default();
    let normal_alignment = Alignment::default();

    font_map.insert(canonical_key(&normal_font), 0);
    fill_map.insert(canonical_key(&normal_fill), 0);
    border_map.insert(canonical_key(&normal_border), 0);
    alignment_map.insert(canonical_key(&normal_alignment), 0);

    fonts.push(normal_font);
    fills.push(normal_fill);
    borders.push(normal_border);
    alignments.push(normal_alignment);

    let mut next_numfmt_id = 164u32;

    // First pass: build (num_fmt_id, font_id, fill_id, border_id, alignment_id)
    // tuples for each input cell, stored in input order.
    let mut tuples: Vec<CellXf> = Vec::with_capacity(styles.len());

    for opt in styles {
        let (num_fmt_id, font_id, fill_id, border_id, alignment_id) = if is_normal(opt) {
            (0u32, 0, 0, 0, 0)
        } else {
            let style = opt.as_ref().unwrap();

            // Font
            let font_id = match &style.font {
                Some(f) => *font_map.entry(canonical_key(f)).or_insert_with(|| {
                    let id = fonts.len() as u32;
                    fonts.push(f.clone());
                    id
                }),
                None => 0,
            };

            // Fill
            let fill_id = match &style.fill {
                Some(f) => *fill_map.entry(canonical_key(f)).or_insert_with(|| {
                    let id = fills.len() as u32;
                    fills.push(f.clone());
                    id
                }),
                None => 0,
            };

            // Border
            let border_id = match &style.border {
                Some(b) => *border_map.entry(canonical_key(b)).or_insert_with(|| {
                    let id = borders.len() as u32;
                    borders.push(b.clone());
                    id
                }),
                None => 0,
            };

            // numFmt (custom format code)
            let num_fmt_id = match &style.num_fmt {
                Some(fmt) => *numfmt_map.entry(fmt.clone()).or_insert_with(|| {
                    let id = next_numfmt_id;
                    next_numfmt_id += 1;
                    num_fmts.push((id, fmt.clone()));
                    id
                }),
                None => 0,
            };

            // Alignment (v0.3.0)
            let alignment_id = match &style.alignment {
                Some(a) => *alignment_map.entry(canonical_key(a)).or_insert_with(|| {
                    let id = alignments.len() as u32;
                    alignments.push(a.clone());
                    id
                }),
                None => 0,
            };

            (num_fmt_id, font_id, fill_id, border_id, alignment_id)
        };

        tuples.push(CellXf {
            num_fmt_id,
            font_id,
            fill_id,
            border_id,
            alignment_id,
        });
    }

    // Second pass: dedup the tuple list into the unique cellXfs table.
    // Always seed Normal at index 0 (OOXML requires xfId="0" → Normal).
    let mut xf_set: BTreeMap<CellXf, u32> = BTreeMap::new();
    let mut cell_xfs: Vec<CellXf> = Vec::new();
    let mut cell_indices: Vec<u32> = Vec::with_capacity(tuples.len());

    let normal = CellXf {
        num_fmt_id: 0,
        font_id: 0,
        fill_id: 0,
        border_id: 0,
        alignment_id: 0,
    };
    xf_set.insert(normal, 0);
    cell_xfs.push(normal);

    for xf in &tuples {
        let idx = *xf_set.entry(*xf).or_insert_with(|| {
            let id = cell_xfs.len() as u32;
            cell_xfs.push(*xf);
            id
        });
        cell_indices.push(idx);
    }

    StyleTable {
        fonts,
        fills,
        borders,
        num_fmts,
        alignments,
        cell_xfs,
        cell_indices,
    }
}

// ---------------------------------------------------------------------------
// XML emission
// ---------------------------------------------------------------------------

/// Write `xl/styles.xml` from a [`StyleTable`].
pub fn emit_styles_xml<W: Write>(w: &mut W, table: &StyleTable) -> Result<(), ExcelrsError> {
    let xmlns = r#" xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main""#;

    xml_header(w)?;
    write_str(w, &format!("<styleSheet{xmlns}>"))?;

    // numFmts (only if there are custom formats)
    emit_num_fmts(w, &table.num_fmts)?;

    // fonts
    emit_fonts(w, &table.fonts)?;

    // fills (must include the required gray125 at index 1)
    emit_fills(w, &table.fills)?;

    // borders
    emit_borders(w, &table.borders)?;

    // cellStyleXfs (built-in Normal only)
    write_str(
        w,
        r#"<cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs>"#,
    )?;

    // cellXfs (the main cell-format table)
    emit_cell_xfs(w, &table.cell_xfs, &table.num_fmts, &table.alignments)?;

    // cellStyles (Normal only)
    write_str(
        w,
        r#"<cellStyles count="1"><cellStyle name="Normal" xfId="0" builtinId="0"/></cellStyles>"#,
    )?;

    write_str(w, "</styleSheet>")?;
    Ok(())
}

fn xml_header<W: Write>(w: &mut W) -> Result<(), ExcelrsError> {
    write_str(w, r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#)
}

fn emit_num_fmts<W: Write>(w: &mut W, num_fmts: &[(u32, String)]) -> Result<(), ExcelrsError> {
    if num_fmts.is_empty() {
        return Ok(());
    }
    write_str(w, &format!(r#"<numFmts count="{}">"#, num_fmts.len()))?;
    for (id, code) in num_fmts {
        write_str(
            w,
            &format!(r#"<numFmt numFmtId="{}" formatCode="{}"/>"#, id, escape(code)),
        )?;
    }
    write_str(w, "</numFmts>")?;
    Ok(())
}

fn emit_fonts<W: Write>(w: &mut W, fonts: &[Font]) -> Result<(), ExcelrsError> {
    write_str(w, &format!(r#"<fonts count="{}">"#, fonts.len()))?;
    for f in fonts {
        write_str(w, "<font>")?;
        if let Some(sz) = f.size {
            write_str(w, &format!(r#"<sz val="{}"/>"#, sz))?;
        }
        if let Some(ref name) = f.name {
            write_str(w, &format!(r#"<name val="{}"/>"#, escape(name)))?;
        }
        if let Some(true) = f.bold {
            write_str(w, r#"<b val="1"/>"#)?;
        }
        if let Some(true) = f.italic {
            write_str(w, r#"<i val="1"/>"#)?;
        }
        if let Some(true) = f.underline {
            write_str(w, r#"<u/>"#)?;
        }
        if let Some(ref color) = f.color {
            write_str(w, &format!(r#"<color rgb="{}"/>"#, color))?;
        }
        write_str(w, "</font>")?;
    }
    write_str(w, "</fonts>")?;
    Ok(())
}

fn emit_fills<W: Write>(w: &mut W, fills: &[Fill]) -> Result<(), ExcelrsError> {
    // OOXML requires at least 2 fills: [0] = none pattern, [1] = gray125.
    // We always inject gray125 as the last fill.
    let count = fills.len() + 1;

    write_str(w, &format!(r#"<fills count="{}">"#, count))?;

    for f in fills {
        write_str(w, "<fill>")?;
        if f.kind == "gradient" {
            let (tag, attrs) = if f.gradient_type.as_deref() == Some("path") {
                // Path gradient: left/right/top/bottom geometry
                let mut a = String::from(r#"type="path""#);
                if let Some(v) = f.gradient_left {
                    a.push_str(&format!(r#" left="{}""#, v));
                }
                if let Some(v) = f.gradient_right {
                    a.push_str(&format!(r#" right="{}""#, v));
                }
                if let Some(v) = f.gradient_top {
                    a.push_str(&format!(r#" top="{}""#, v));
                }
                if let Some(v) = f.gradient_bottom {
                    a.push_str(&format!(r#" bottom="{}""#, v));
                }
                ("gradientFill", a)
            } else {
                // Linear gradient (default): degree only
                let mut a = String::from(r#"type="linear""#);
                if let Some(deg) = f.gradient_degree {
                    a.push_str(&format!(r#" degree="{}""#, deg));
                }
                ("gradientFill", a)
            };
            write_str(w, &format!("<{} {}>", tag, attrs))?;
            if let Some(ref stops) = f.gradient_stops {
                for stop in stops {
                    write_str(
                        w,
                        &format!(
                            r#"<stop position="{}"><color rgb="{}"/></stop>"#,
                            stop.position, stop.color
                        ),
                    )?;
                }
            }
            write_str(w, "</gradientFill>")?;
        } else {
            let has_fg = f.foreground.is_some();
            let has_bg = f.background.is_some();
            if has_fg || has_bg {
                write_str(w, &format!(r#"<patternFill patternType="{}">"#, f.kind))?;
                if let Some(ref fg) = f.foreground {
                    write_str(w, &format!(r#"<fgColor rgb="{}"/>"#, fg))?;
                }
                if let Some(ref bg) = f.background {
                    write_str(w, &format!(r#"<bgColor rgb="{}"/>"#, bg))?;
                }
                write_str(w, "</patternFill>")?;
            } else {
                write_str(w, &format!(r#"<patternFill patternType="{}"/>"#, f.kind))?;
            }
        }
        write_str(w, "</fill>")?;
    }

    // Always inject the required gray125 fill
    write_str(w, r#"<fill><patternFill patternType="gray125"/></fill>"#)?;
    write_str(w, "</fills>")?;
    Ok(())
}

fn emit_borders<W: Write>(w: &mut W, borders: &[Border]) -> Result<(), ExcelrsError> {
    write_str(w, &format!(r#"<borders count="{}">"#, borders.len()))?;
    for b in borders {
        // Build diagonal attributes on <border> element
        let mut diag_attrs = String::new();
        if let Some(true) = b.diagonal_up {
            diag_attrs.push_str(r#" diagonalUp="1""#);
        }
        if let Some(true) = b.diagonal_down {
            diag_attrs.push_str(r#" diagonalDown="1""#);
        }
        write_str(w, &format!("<border{}>", diag_attrs))?;
        emit_border_side(w, "left", &b.left)?;
        emit_border_side(w, "right", &b.right)?;
        emit_border_side(w, "top", &b.top)?;
        emit_border_side(w, "bottom", &b.bottom)?;
        if b.diagonal.is_some() {
            emit_border_side(w, "diagonal", &b.diagonal)?;
        } else {
            write_str(w, r#"<diagonal/>"#)?;
        }
        write_str(w, "</border>")?;
    }
    write_str(w, "</borders>")?;
    Ok(())
}

fn emit_border_side<W: Write>(
    w: &mut W,
    side: &str,
    bs: &Option<crate::model::style::BorderStyle>,
) -> Result<(), ExcelrsError> {
    match bs {
        None => write_str(w, &format!("<{side}/>")),
        Some(b) => {
            let style_attr = &b.style;
            let color = match &b.color {
                Some(c) => format!(r#"<color rgb="{}"/>"#, c),
                None => String::new(),
            };
            write_str(w, &format!("<{side} style=\"{style_attr}\">{color}</{side}>"))
        }
    }
}

fn emit_cell_xfs<W: Write>(
    w: &mut W,
    cell_xfs: &[CellXf],
    num_fmts: &[(u32, String)],
    alignments: &[Alignment],
) -> Result<(), ExcelrsError> {
    write_str(w, &format!(r#"<cellXfs count="{}">"#, cell_xfs.len()))?;

    // Build a set of which numFmt IDs are custom (so we can emit applyNumberFormat)
    let custom_numfmt: std::collections::HashSet<u32> = num_fmts.iter().map(|(id, _)| *id).collect();

    for xf in cell_xfs {
        let apply_number_fmt = xf.num_fmt_id != 0 && custom_numfmt.contains(&xf.num_fmt_id);
        let apply_font = xf.font_id != 0;
        let apply_fill = xf.fill_id != 0;
        let apply_border = xf.border_id != 0;
        let apply_alignment = xf.alignment_id != 0;

        // Build comma-separated list of apply-X attributes (only when true)
        let mut apply_parts: Vec<&str> = Vec::new();
        if apply_number_fmt {
            apply_parts.push(r#"applyNumberFormat="1""#);
        }
        if apply_font {
            apply_parts.push(r#"applyFont="1""#);
        }
        if apply_fill {
            apply_parts.push(r#"applyFill="1""#);
        }
        if apply_border {
            apply_parts.push(r#"applyBorder="1""#);
        }
        if apply_alignment {
            apply_parts.push(r#"applyAlignment="1""#);
        }

        let apply_str = if apply_parts.is_empty() {
            String::new()
        } else {
            format!(" {}", apply_parts.join(" "))
        };

        let has_children = apply_alignment;

        if has_children {
            write_str(
                w,
                &format!(
                    r#"<xf numFmtId="{}" fontId="{}" fillId="{}" borderId="{}" xfId="0"{}>"#,
                    xf.num_fmt_id, xf.font_id, xf.fill_id, xf.border_id, apply_str,
                ),
            )?;
            // Emit alignment child (v0.3.0)
            emit_alignment_child(w, xf, alignments)?;
            write_str(w, "</xf>")?;
        } else {
            write_str(
                w,
                &format!(
                    r#"<xf numFmtId="{}" fontId="{}" fillId="{}" borderId="{}" xfId="0"{}/>"#,
                    xf.num_fmt_id, xf.font_id, xf.fill_id, xf.border_id, apply_str,
                ),
            )?;
        }
    }
    write_str(w, "</cellXfs>")?;
    Ok(())
}

/// Emit the `<alignment>` child element for a cellXf that has a non-default
/// alignment.  OOXML vertical value "center" is emitted for model "middle".
fn emit_alignment_child<W: Write>(w: &mut W, xf: &CellXf, alignments: &[Alignment]) -> Result<(), ExcelrsError> {
    let alignment = match alignments.get(xf.alignment_id as usize) {
        Some(a) => a,
        None => return Ok(()),
    };

    let mut parts: Vec<String> = Vec::new();
    if let Some(ref h) = alignment.horizontal {
        parts.push(format!(r##"horizontal="{}""##, h));
    }
    if let Some(ref v) = alignment.vertical {
        // OOXML uses "center"; excelrs API uses "middle"
        let ooxml = if v == "middle" { "center" } else { v.as_str() };
        parts.push(format!(r##"vertical="{}""##, ooxml));
    }
    if let Some(wt) = alignment.wrap_text {
        if wt {
            parts.push(r#"wrapText="1""#.to_string());
        }
    }
    if let Some(indent) = alignment.indent {
        if indent > 0 {
            parts.push(format!(r#"indent="{}""#, indent));
        }
    }

    if parts.is_empty() {
        // No meaningful alignment attributes — don't emit an empty element
        return Ok(());
    }

    write_str(w, &format!("<alignment {}/>", parts.join(" ")))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Utility
// ---------------------------------------------------------------------------

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
    use crate::model::style::{BorderStyle, Fill, GradientStop};

    // -- 4 dedup tests (per §9.2 budget) --

    /// Empty input → only Normal at cellXfs[0]; sub-tables have 1 entry each.
    #[test]
    fn dedup_empty() {
        let table = build_style_table(&[]);
        assert_eq!(table.cell_xfs.len(), 1); // Normal is always seeded
        assert_eq!(
            table.cell_xfs[0],
            CellXf {
                num_fmt_id: 0,
                font_id: 0,
                fill_id: 0,
                border_id: 0,
                alignment_id: 0,
            }
        );
        assert_eq!(table.fonts.len(), 1); // Normal
        assert_eq!(table.fills.len(), 1); // Normal (will be 1 + gray125 at emit)
        assert_eq!(table.borders.len(), 1); // Normal
        assert!(table.num_fmts.is_empty());
        assert!(table.cell_indices.is_empty());
    }

    /// Single style → Normal + 1 cellXfs entry. Sub-table grows as needed.
    #[test]
    fn dedup_normal_only() {
        let styles = vec![None, None];
        let table = build_style_table(&styles);
        // Both map to Normal → same unique cellXfs entry
        assert_eq!(table.cell_xfs.len(), 1);
        assert_eq!(table.cell_indices.len(), 2);
        assert_eq!(table.cell_indices[0], 0);
        assert_eq!(table.cell_indices[1], 0);

        // cell_xfs[0] is Normal
        let xf = &table.cell_xfs[0];
        assert_eq!(xf.num_fmt_id, 0);
        assert_eq!(xf.font_id, 0);
        assert_eq!(xf.fill_id, 0);
        assert_eq!(xf.border_id, 0);
    }

    /// Two distinct styles produce two distinct cellXfs entries (plus Normal).
    #[test]
    fn dedup_distinct() {
        let styles = vec![
            Some(Style {
                num_fmt: Some("0.00%".into()),
                ..Default::default()
            }),
            Some(Style {
                font: Some(crate::model::style::Font {
                    bold: Some(true),
                    ..Default::default()
                }),
                ..Default::default()
            }),
        ];
        let table = build_style_table(&styles);
        assert_eq!(table.cell_xfs.len(), 3); // Normal + 2 distinct CellXf entries

        // cell_xfs[0] is Normal
        assert_eq!(table.cell_xfs[0].num_fmt_id, 0);
        assert_eq!(table.cell_xfs[0].font_id, 0);

        // cell_xfs[1]: custom numFmt, Normal font
        let xf1 = &table.cell_xfs[1];
        assert!(xf1.num_fmt_id >= 164);
        assert_eq!(xf1.font_id, 0);
        assert_eq!(xf1.fill_id, 0);
        assert_eq!(xf1.border_id, 0);

        // cell_xfs[2]: custom font, Normal numFmt
        let xf2 = &table.cell_xfs[2];
        assert_eq!(xf2.num_fmt_id, 0);
        assert!(xf2.font_id > 0);
        assert_eq!(xf2.fill_id, 0);
        assert_eq!(xf2.border_id, 0);

        // Distinct font entries
        assert_eq!(table.fonts.len(), 2);
    }

    /// Duplicate styles dedup to the same cellXfs index.
    #[test]
    fn dedup_duplicates() {
        let s = Style {
            num_fmt: Some("0.00%".into()),
            ..Default::default()
        };
        let styles = vec![Some(s.clone()), Some(s.clone())];
        let table = build_style_table(&styles);
        // Two inputs, two unique cellXfs entries (Normal + the non-Normal style)
        assert_eq!(table.cell_xfs.len(), 2);
        assert_eq!(table.cell_indices.len(), 2);
        assert_eq!(table.cell_indices[0], table.cell_indices[1]);
        // Both map to index 1 (Normal is index 0)
        assert_eq!(table.cell_indices[0], 1);
    }

    // -- 4 emit tests --

    /// Minimal StyleTable → valid OOXML with just Normal.
    #[test]
    fn emit_minimal() {
        let table = build_style_table(&[]);
        let mut buf = Vec::new();
        emit_styles_xml(&mut buf, &table).unwrap();
        let xml = String::from_utf8(buf).unwrap();

        assert!(xml.starts_with("<?xml"));
        assert!(xml.contains(r#"<fonts count="1">"#));
        assert!(xml.contains(r#"<fills count="2">"#)); // Normal + gray125
        assert!(xml.contains(r#"<borders count="1">"#));
        assert!(xml.contains(r#"<cellXfs count="1">"#)); // Normal is always present
        assert!(xml.contains("</styleSheet>"));

        // Should not contain numFmts (no custom formats)
        assert!(!xml.contains("<numFmts"));
    }

    /// Populated StyleTable with all sub-types.
    #[test]
    fn emit_full() {
        let styles = vec![Some(Style {
            font: Some(crate::model::style::Font {
                bold: Some(true),
                color: Some("FFFF0000".into()),
                ..Default::default()
            }),
            fill: Some(Fill {
                kind: "solid".into(),
                foreground: Some("FFFFFF00".into()),
                ..Default::default()
            }),
            border: Some(crate::model::style::Border {
                top: Some(BorderStyle {
                    style: "thin".into(),
                    color: Some("FF000000".into()),
                }),
                ..Default::default()
            }),
            num_fmt: Some("0.00%".into()),
            ..Default::default()
        })];
        let table = build_style_table(&styles);
        let mut buf = Vec::new();
        emit_styles_xml(&mut buf, &table).unwrap();
        let xml = String::from_utf8(buf).unwrap();

        // All sub-tables present
        assert!(xml.contains(r#"<fonts count="2">"#)); // Normal + bold red
        assert!(xml.contains(r#"<fills count="3">"#)); // Normal + solid yellow + gray125
        assert!(xml.contains(r#"<borders count="2">"#)); // Normal + thin top
        assert!(xml.contains(r#"<numFmts count="1">"#));
        assert!(xml.contains(r#"formatCode="0.00%""#));
        assert!(xml.contains(r#"<cellXfs count="2">"#)); // Normal + 1 unique style

        // Verify font content
        assert!(xml.contains("<b val=\"1\"/>"));
        assert!(xml.contains(r#"color rgb="FFFF0000""#));

        // Verify fill content
        assert!(xml.contains(r#"fgColor rgb="FFFFFF00""#));

        // Verify border content
        assert!(xml.contains(r#"style="thin""#));
        assert!(xml.contains(r#"color rgb="FF000000""#));
    }

    /// XML output parses as valid well-formed XML with quick_xml.
    #[test]
    fn emit_parses() {
        let styles = vec![Some(Style {
            num_fmt: Some("0.00%".into()),
            ..Default::default()
        })];
        let table = build_style_table(&styles);
        let mut buf = Vec::new();
        emit_styles_xml(&mut buf, &table).unwrap();
        let xml = String::from_utf8(buf).unwrap();

        // quick_xml::Reader can parse it
        let mut reader = quick_xml::Reader::from_str(&xml);
        let mut count = 0usize;
        loop {
            match reader.read_event() {
                Ok(quick_xml::events::Event::Eof) => break,
                Ok(_) => count += 1,
                Err(e) => panic!("XML parse error at event {count}: {e}"),
            }
        }
        assert!(count > 0, "should have parsed at least one event");
    }

    /// Empty style (Some with all None) must map to Normal, not create
    /// a duplicate cellXfs entry.
    #[test]
    fn dedup_empty_style_is_normal() {
        let s = Style::default(); // is_empty() returns true
        let styles = vec![None, Some(s)];
        let table = build_style_table(&styles);
        // Two inputs but only one unique cellXfs entry (Normal)
        assert_eq!(table.cell_xfs.len(), 1);
        assert_eq!(table.cell_indices.len(), 2);
        assert_eq!(table.cell_indices[0], 0);
        assert_eq!(table.cell_indices[1], 0);
    }

    /// Font color uppercasing from A2 ensures dedup works case-insensitively.
    #[test]
    fn dedup_color_case_same() {
        let s1 = Style {
            font: Some(crate::model::style::Font {
                color: Some("FF0000".into()), // validated → uppercased
                ..Default::default()
            }),
            ..Default::default()
        };
        let s2 = Style {
            font: Some(crate::model::style::Font {
                color: Some("FF0000".into()),
                ..Default::default()
            }),
            ..Default::default()
        };
        let styles = vec![Some(s1), Some(s2)];
        let table = build_style_table(&styles);
        // Same cellXfs index for both inputs (at index 1, since Normal is index 0)
        assert_eq!(table.cell_indices[0], table.cell_indices[1]);
        assert_eq!(table.cell_xfs.len(), 2); // Normal + 1 dedup'd CellXf
    }

    /// cell_indices maps each input cell to its unique cellXfs index.
    #[test]
    fn dedup_cell_indices_mapping() {
        // 5 inputs: Normal, style A, style B, Normal, style A
        let s_a = Style {
            num_fmt: Some("0.00%".into()),
            ..Default::default()
        };
        let s_b = Style {
            font: Some(crate::model::style::Font {
                bold: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        };
        let styles = vec![None, Some(s_a.clone()), Some(s_b.clone()), None, Some(s_a)];
        let table = build_style_table(&styles);
        assert_eq!(table.cell_indices.len(), 5);

        // Normal → index 0
        assert_eq!(table.cell_indices[0], 0);
        assert_eq!(table.cell_indices[3], 0);

        // Style A appears at input 1 and 4 → same index
        assert_eq!(table.cell_indices[1], table.cell_indices[4]);
        // Style A and Style B are different
        assert_ne!(table.cell_indices[1], table.cell_indices[2]);
        assert_ne!(table.cell_indices[2], table.cell_indices[3]);
    }

    // -- Alignment dedup tests (v0.3.0) --

    /// Alignment dedup: same alignment → same alignment_id.
    #[test]
    fn dedup_alignment_same() {
        let s = Style {
            alignment: Some(crate::model::style::Alignment {
                horizontal: Some("center".into()),
                vertical: Some("middle".into()),
                ..Default::default()
            }),
            ..Default::default()
        };
        let styles = vec![Some(s.clone()), Some(s)];
        let table = build_style_table(&styles);
        // Both get same cellXfs (Normal + 1 unique = 2)
        assert_eq!(table.cell_xfs.len(), 2);
        assert_eq!(table.cell_indices[0], table.cell_indices[1]);
        assert_eq!(table.cell_indices[0], 1); // index 1 = the non-Normal style
    }

    /// Alignment emit: alignments section and applyAlignment="1" present.
    #[test]
    fn emit_alignment() {
        let styles = vec![Some(Style {
            alignment: Some(crate::model::style::Alignment {
                horizontal: Some("center".into()),
                vertical: Some("middle".into()),
                wrap_text: Some(true),
                indent: Some(2),
            }),
            ..Default::default()
        })];
        let table = build_style_table(&styles);
        let mut buf = Vec::new();
        emit_styles_xml(&mut buf, &table).unwrap();
        let xml = String::from_utf8(buf).unwrap();

        // alignment element present with correct attributes
        assert!(xml.contains(r#"applyAlignment="1""#));
        assert!(xml.contains(r##"horizontal="center""##));
        // OOXML "center" not API "middle"
        assert!(xml.contains(r##"vertical="center""##));
        assert!(xml.contains(r##"wrapText="1""##));
        assert!(xml.contains(r##"indent="2""##));
    }

    /// Alignment vertical: model "middle" → OOXML "center".
    #[test]
    fn emit_alignment_vertical_mapping() {
        let styles = vec![Some(Style {
            alignment: Some(crate::model::style::Alignment {
                vertical: Some("middle".into()),
                ..Default::default()
            }),
            ..Default::default()
        })];
        let table = build_style_table(&styles);
        let mut buf = Vec::new();
        emit_styles_xml(&mut buf, &table).unwrap();
        let xml = String::from_utf8(buf).unwrap();

        // Must NOT contain vertical="middle"
        assert!(!xml.contains(r##"vertical="middle""##));
        // Must contain vertical="center"
        assert!(xml.contains(r##"vertical="center""##));
    }

    /// Integration: alignment style set via `set_cell_style` produces
    /// a second cellXfs entry with alignment, and the cell references it.
    #[test]
    fn integration_alignment_emitted_via_set_cell_style() {
        use crate::model::workbook_inner::WorkbookInner;
        use std::io::{Cursor, Read};

        let mut inner = WorkbookInner::new();
        let ws = inner.add_worksheet("Debug".into());
        ws.add_row(vec![serde_json::json!("hello")]);
        ws.set_cell_style(
            1,
            1,
            serde_json::json!({
                "alignment": { "horizontal": "center", "vertical": "middle" }
            }),
        )
        .unwrap();

        let bytes = crate::writer::xlsx::workbook_to_bytes(&inner).unwrap();
        let mut archive = zip::read::ZipArchive::new(Cursor::new(&bytes)).unwrap();
        let mut styles_xml = String::new();
        archive
            .by_name("xl/styles.xml")
            .unwrap()
            .read_to_string(&mut styles_xml)
            .unwrap();
        let mut sheet_xml = String::new();
        archive
            .by_name("xl/worksheets/sheet1.xml")
            .unwrap()
            .read_to_string(&mut sheet_xml)
            .unwrap();

        assert!(styles_xml.contains(r#"cellXfs count="2""#));
        assert!(sheet_xml.contains(r#"s="1""#));
    }

    /// Multiple distinct numFmt codes each get their own ID starting at 164.
    #[test]
    fn dedup_num_fmt_ids() {
        let styles = vec![
            Some(Style {
                num_fmt: Some("0.00%".into()),
                ..Default::default()
            }),
            Some(Style {
                num_fmt: Some("yyyy-mm-dd".into()),
                ..Default::default()
            }),
            Some(Style {
                num_fmt: Some("0.00%".into()), // duplicate
                ..Default::default()
            }),
        ];
        let table = build_style_table(&styles);
        assert_eq!(table.num_fmts.len(), 2);
        assert_eq!(table.num_fmts[0].0, 164);
        assert_eq!(table.num_fmts[0].1, "0.00%");
        assert_eq!(table.num_fmts[1].0, 165);
        assert_eq!(table.num_fmts[1].1, "yyyy-mm-dd");

        // 3 inputs, 2 unique cellXfs entries + Normal = 3 total
        assert_eq!(table.cell_xfs.len(), 3);
        assert_eq!(table.cell_indices.len(), 3);
        // Input 0 and 2 share the same cellXfs index
        assert_eq!(table.cell_indices[0], table.cell_indices[2]);
        // Input 1 has a different index
        assert_ne!(table.cell_indices[0], table.cell_indices[1]);

        // Verify the unique cellXfs entries match
        assert_eq!(
            table.cell_xfs[table.cell_indices[0] as usize].num_fmt_id,
            table.cell_xfs[table.cell_indices[2] as usize].num_fmt_id
        );
    }

    /// Linear gradient must NOT emit `angle` (invalid CT_GradientFill attr).
    #[test]
    fn test_emit_gradient_linear_has_no_angle() {
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
                    color: "FF00FF00".into(),
                    position: 1.0,
                },
            ]),
            ..Default::default()
        };
        let table = build_style_table(&[Some(Style {
            fill: Some(fill),
            ..Default::default()
        })]);
        let mut buf = Vec::new();
        emit_styles_xml(&mut buf, &table).unwrap();
        let xml = String::from_utf8(buf).unwrap();

        assert!(
            xml.contains(r##"type="linear""##),
            "linear gradient missing type attr: {xml}"
        );
        assert!(
            xml.contains(r##"degree="45""##),
            "linear gradient missing degree attr: {xml}"
        );
        assert!(
            !xml.contains("angle="),
            "angle is not a valid CT_GradientFill attribute: {xml}"
        );
    }

    /// Path gradient must emit correct geometry attrs (left/right/top/bottom)
    /// and must NOT emit `angle`.
    #[test]
    fn test_emit_gradient_path_emits_geometry() {
        let fill = Fill {
            kind: "gradient".into(),
            gradient_type: Some("path".into()),
            gradient_left: Some(0.0),
            gradient_right: Some(1.0),
            gradient_top: Some(0.0),
            gradient_bottom: Some(1.0),
            gradient_stops: Some(vec![
                GradientStop {
                    color: "FFFF0000".into(),
                    position: 0.0,
                },
                GradientStop {
                    color: "FF00FF00".into(),
                    position: 1.0,
                },
            ]),
            ..Default::default()
        };
        let table = build_style_table(&[Some(Style {
            fill: Some(fill),
            ..Default::default()
        })]);
        let mut buf = Vec::new();
        emit_styles_xml(&mut buf, &table).unwrap();
        let xml = String::from_utf8(buf).unwrap();

        assert!(
            xml.contains(r##"type="path""##),
            "path gradient missing type attr: {xml}"
        );
        assert!(xml.contains(r##"left="0""##), "path gradient missing left: {xml}");
        assert!(xml.contains(r##"right="1""##), "path gradient missing right: {xml}");
        assert!(xml.contains(r##"top="0""##), "path gradient missing top: {xml}");
        assert!(xml.contains(r##"bottom="1""##), "path gradient missing bottom: {xml}");
        assert!(
            !xml.contains("angle="),
            "angle is not a valid CT_GradientFill attribute: {xml}"
        );
    }
}
