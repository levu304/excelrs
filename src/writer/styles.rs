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
use crate::model::style::{Border, Fill, Font, Style};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Deduplicated sub-tables plus the cell-level style-index table.
///
/// Every sub-table has index 0 as the "Normal" entry (empty/font Calibri 11).
/// The `cell_xfs` vector mirrors the input cell order; each entry records the
/// sub-table indices for that cell's style.
pub struct StyleTable {
    pub fonts: Vec<Font>,
    pub fills: Vec<Fill>,
    pub borders: Vec<Border>,
    pub num_fmts: Vec<(u32, String)>,
    pub cell_xfs: Vec<CellXf>,
}

/// One cell level format (XF) record — indices into the sub-tables above.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellXf {
    /// Custom numFmt ID (≥164) or 0 for General / no custom format.
    pub num_fmt_id: u32,
    /// Index into [`StyleTable::fonts`].
    pub font_id: u32,
    /// Index into [`StyleTable::fills`].
    pub fill_id: u32,
    /// Index into [`StyleTable::borders`].
    pub border_id: u32,
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
/// - Every input entry produces one [`CellXf`] in the output.
/// - `None` / empty style → Normal (all indices 0).
/// - Matching sub-values (same font, same fill, etc.) share one sub-table slot.
pub fn build_style_table(styles: &[Option<Style>]) -> StyleTable {
    // Sub-table dedup maps: canonical key → index
    let mut font_map: BTreeMap<String, u32> = BTreeMap::new();
    let mut fill_map: BTreeMap<String, u32> = BTreeMap::new();
    let mut border_map: BTreeMap<String, u32> = BTreeMap::new();
    let mut numfmt_map: BTreeMap<String, u32> = BTreeMap::new();

    let mut fonts: Vec<Font> = Vec::new();
    let mut fills: Vec<Fill> = Vec::new();
    let mut borders: Vec<Border> = Vec::new();
    let mut num_fmts: Vec<(u32, String)> = Vec::new();
    let mut cell_xfs: Vec<CellXf> = Vec::with_capacity(styles.len());

    // Always seed index 0 with Normal defaults
    let normal_font = Font::default();
    let normal_fill = Fill::default();
    let normal_border = Border::default();

    font_map.insert(canonical_key(&normal_font), 0);
    fill_map.insert(canonical_key(&normal_fill), 0);
    border_map.insert(canonical_key(&normal_border), 0);

    fonts.push(normal_font);
    fills.push(normal_fill);
    borders.push(normal_border);

    let mut next_numfmt_id = 164u32;

    for opt in styles {
        let (num_fmt_id, font_id, fill_id, border_id) = if is_normal(opt) {
            (0u32, 0, 0, 0)
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

            (num_fmt_id, font_id, fill_id, border_id)
        };

        cell_xfs.push(CellXf {
            num_fmt_id,
            font_id,
            fill_id,
            border_id,
        });
    }

    StyleTable {
        fonts,
        fills,
        borders,
        num_fmts,
        cell_xfs,
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
    emit_cell_xfs(w, &table.cell_xfs, &table.num_fmts)?;

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
            write_str(w, &format!(r#"<color argb="{}"/>"#, color))?;
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
        let pattern = if f.kind == "none" { "none" } else { "solid" };
        write_str(w, &format!(r#"<patternFill patternType="{}"/>"#, pattern))?;
        if let Some(ref fg) = f.foreground {
            write_str(w, &format!(r#"<fgColor argb="{}"/>"#, fg))?;
        }
        if let Some(ref bg) = f.background {
            write_str(w, &format!(r#"<bgColor argb="{}"/>"#, bg))?;
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
        write_str(w, "<border>")?;
        emit_border_side(w, "left", &b.left)?;
        emit_border_side(w, "right", &b.right)?;
        emit_border_side(w, "top", &b.top)?;
        emit_border_side(w, "bottom", &b.bottom)?;
        write_str(w, r#"<diagonal/>"#)?;
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
                Some(c) => format!(r#"<color argb="{}"/>"#, c),
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
) -> Result<(), ExcelrsError> {
    write_str(w, &format!(r#"<cellXfs count="{}">"#, cell_xfs.len()))?;

    // Build a set of which numFmt IDs are custom (so we can emit applyNumberFormat)
    let custom_numfmt: std::collections::HashSet<u32> =
        num_fmts.iter().map(|(id, _)| *id).collect();

    for xf in cell_xfs {
        let apply_number_fmt = xf.num_fmt_id != 0 && custom_numfmt.contains(&xf.num_fmt_id);
        write_str(
            w,
            &format!(
                r#"<xf numFmtId="{}" fontId="{}" fillId="{}" borderId="{}" xfId="0"{}/>"#,
                xf.num_fmt_id,
                xf.font_id,
                xf.fill_id,
                xf.border_id,
                if apply_number_fmt { r#" applyNumberFormat="1""# } else { "" },
            ),
        )?;
    }
    write_str(w, "</cellXfs>")?;
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
    use crate::model::style::{BorderStyle, Fill};

    // -- 4 dedup tests (per §9.2 budget) --

    /// Empty input → only Normal at cellXfs[0]; sub-tables have 1 entry each.
    #[test]
    fn dedup_empty() {
        let table = build_style_table(&[]);
        assert_eq!(table.cell_xfs.len(), 0);
        assert_eq!(table.fonts.len(), 1); // Normal
        assert_eq!(table.fills.len(), 1); // Normal (will be 1 + gray125 at emit)
        assert_eq!(table.borders.len(), 1); // Normal
        assert!(table.num_fmts.is_empty());
    }

    /// Single style → Normal + 1 cellXfs entry. Sub-table grows as needed.
    #[test]
    fn dedup_normal_only() {
        let styles = vec![None, None];
        let table = build_style_table(&styles);
        assert_eq!(table.cell_xfs.len(), 2);
        // Both map to Normal
        for xf in &table.cell_xfs {
            assert_eq!(xf.num_fmt_id, 0);
            assert_eq!(xf.font_id, 0);
            assert_eq!(xf.fill_id, 0);
            assert_eq!(xf.border_id, 0);
        }
    }

    /// Two distinct styles produce two distinct cellXfs entries.
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
        assert_eq!(table.cell_xfs.len(), 2);

        // First: custom numFmt, Normal font
        let xf0 = &table.cell_xfs[0];
        assert!(xf0.num_fmt_id >= 164);
        assert_eq!(xf0.font_id, 0); // Normal
        assert_eq!(xf0.fill_id, 0);
        assert_eq!(xf0.border_id, 0);

        // Second: custom font, Normal numFmt
        let xf1 = &table.cell_xfs[1];
        assert_eq!(xf1.num_fmt_id, 0); // Normal
        assert!(xf1.font_id > 0); // custom font
        assert_eq!(xf1.fill_id, 0);
        assert_eq!(xf1.border_id, 0);

        // Distinct font entries
        assert_eq!(table.fonts.len(), 2);
    }

    /// Duplicate styles dedup to the same index.
    #[test]
    fn dedup_duplicates() {
        let s = Style {
            num_fmt: Some("0.00%".into()),
            ..Default::default()
        };
        let styles = vec![Some(s.clone()), Some(s.clone())];
        let table = build_style_table(&styles);
        assert_eq!(table.cell_xfs.len(), 2);
        assert_eq!(table.cell_xfs[0], table.cell_xfs[1]);
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
        assert!(xml.contains(r#"<cellXfs count="0">"#)); // no cells
        assert!(xml.contains("</styleSheet>"));

        // Should not contain numFmts (no custom formats)
        assert!(!xml.contains("<numFmts"));
    }

    /// Populated StyleTable with all sub-types.
    #[test]
    fn emit_full() {
        let styles = vec![
            Some(Style {
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
            }),
        ];
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
        assert!(xml.contains(r#"<cellXfs count="1">"#));

        // Verify font content
        assert!(xml.contains("<b val=\"1\"/>"));
        assert!(xml.contains(r#"color argb="FFFF0000""#));

        // Verify fill content
        assert!(xml.contains(r#"fgColor argb="FFFFFF00""#));

        // Verify border content
        assert!(xml.contains(r#"style="thin""#));
        assert!(xml.contains(r#"color argb="FF000000""#));
    }

    /// XML output parses as valid well-formed XML with quick_xml.
    #[test]
    fn emit_parses() {
        let styles = vec![
            Some(Style {
                num_fmt: Some("0.00%".into()),
                ..Default::default()
            }),
        ];
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
        assert_eq!(table.cell_xfs.len(), 2);
        assert_eq!(table.cell_xfs[0], table.cell_xfs[1]);
        assert_eq!(table.cell_xfs[0].font_id, 0);
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
        assert_eq!(table.cell_xfs[0], table.cell_xfs[1]);
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
        // Duplicate matches
        assert_eq!(table.cell_xfs[0].num_fmt_id, table.cell_xfs[2].num_fmt_id);
    }
}
