//! Theme color scheme resolver — read-side color resolution for XLSX styles.
//!
//! Resolves `<color theme="N">` and `<color indexed="N">` to concrete ARGB
//! hex strings using the ECMA-376 default scheme, optional `<a:clrScheme>`
//! overrides, and tint (darken/lighten) adjustments.
//!
//! # Internal module
//! This is a Rust-internal module with no napi exports. The `color` field on
//! style structs stays `Option<String>` ARGB — this resolver is what feeds into
//! it in a later pass (§B of v0.6.0).

use crate::error::ExcelrsError;
use quick_xml::events::Event;
use quick_xml::Reader as XmlReader;

// ---------------------------------------------------------------------------
// Color (theme-preserving carrier, v0.13.0)
// ---------------------------------------------------------------------------

/// A color resolved from OOXML, carrying the concrete ARGB plus the originating
/// theme reference (index + tint) when present, so the theme link can be
/// preserved on write (v0.13.0).
///
/// The public style API only exposes the ARGB (`color` / `foreground` /
/// `background` fields). `theme`/`tint` are internal — skipped from the napi
/// object and from serde — and only populated by the reader when a color came
/// from a `<color theme="N"/>` reference.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Color {
    /// Concrete ARGB hex (8 chars), always present when the color is resolved.
    pub rgb: String,
    /// Originating theme index (0–11) from `<color theme="N"/>`, if any.
    pub theme: Option<u8>,
    /// Tint from the `tint` attribute, if any.
    pub tint: Option<f64>,
}

// ---------------------------------------------------------------------------
// ECMA-376 §18.8.27 — default indexed color palette (56 entries, indices 0–55)
// ---------------------------------------------------------------------------

/// The default indexed color palette from ECMA-376 §18.8.27.
///
/// Each entry is a 6-char RGB string (no alpha prefix). Indices 0–7 are
/// redundant with 8–15 for backwards compatibility.
const SYSTEM_INDEXED_COLORS: [&str; 56] = [
    "000000", //  0 — Black
    "FFFFFF", //  1 — White
    "FF0000", //  2 — Red
    "00FF00", //  3 — Green
    "0000FF", //  4 — Blue
    "FFFF00", //  5 — Yellow
    "FF00FF", //  6 — Magenta
    "00FFFF", //  7 — Cyan
    "000000", //  8 — Black (redundant)
    "FFFFFF", //  9 — White (redundant)
    "FF0000", // 10 — Red (redundant)
    "00FF00", // 11 — Green (redundant)
    "0000FF", // 12 — Blue (redundant)
    "FFFF00", // 13 — Yellow (redundant)
    "FF00FF", // 14 — Magenta (redundant)
    "00FFFF", // 15 — Cyan (redundant)
    "800000", // 16 — Dark Red
    "008000", // 17 — Dark Green
    "000080", // 18 — Dark Blue
    "808000", // 19 — Olive
    "800080", // 20 — Purple
    "008080", // 21 — Teal
    "C0C0C0", // 22 — Silver
    "808080", // 23 — Gray
    "9999FF", // 24
    "993366", // 25
    "FFFFCC", // 26
    "CCFFFF", // 27
    "660066", // 28
    "FF8080", // 29
    "0066CC", // 30
    "CCCCFF", // 31
    "000080", // 32
    "FF00FF", // 33
    "FFFF00", // 34
    "00FFFF", // 35
    "800080", // 36
    "800000", // 37
    "008080", // 38
    "0000FF", // 39
    "00CCFF", // 40
    "CCFFFF", // 41
    "CCFFCC", // 42
    "FFFF99", // 43
    "99CCFF", // 44
    "FF99CC", // 45
    "CC99FF", // 46
    "FFCC99", // 47
    "3366FF", // 48
    "33CCCC", // 49
    "99CC00", // 50
    "FFCC00", // 51
    "FF9900", // 52
    "FF6600", // 53
    "666699", // 54
    "969696", // 55
];

// ---------------------------------------------------------------------------
// ThemeColorScheme
// ---------------------------------------------------------------------------

/// ECMA-376 theme color scheme — 12 named slots plus optional indexed palette override.
///
/// | Index | Name       | Default   |
/// |-------|------------|-----------|
/// | 0     | dk1        | 000000    |
/// | 1     | lt1        | FFFFFF    |
/// | 2     | dk2        | 1F497D    |
/// | 3     | lt2        | EEECE1    |
/// | 4     | accent1    | 4F81BD    |
/// | 5     | accent2    | C0504D    |
/// | 6     | accent3    | 9BBB59    |
/// | 7     | accent4    | F79646    |
/// | 8     | accent5    | 8064A2    |
/// | 9     | accent6    | 4BACC6    |
/// | 10    | hlink      | 0000FF    |
/// | 11    | folHlink   | 800080    |
#[derive(Clone, Debug, PartialEq)]
pub struct ThemeColorScheme {
    /// 12 theme color entries, each a 6-char RGB string (no alpha).
    entries: [String; 12],
    /// Optional override of the 56-entry indexed color palette.
    indexed: Option<[String; 56]>,
}

impl Default for ThemeColorScheme {
    fn default() -> Self {
        ThemeColorScheme {
            entries: [
                "000000".into(), //  0 — dk1
                "FFFFFF".into(), //  1 — lt1
                "1F497D".into(), //  2 — dk2
                "EEECE1".into(), //  3 — lt2
                "4F81BD".into(), //  4 — accent1
                "C0504D".into(), //  5 — accent2
                "9BBB59".into(), //  6 — accent3
                "F79646".into(), //  7 — accent4
                "8064A2".into(), //  8 — accent5
                "4BACC6".into(), //  9 — accent6
                "0000FF".into(), // 10 — hlink
                "800080".into(), // 11 — folHlink
            ],
            indexed: None,
        }
    }
}

impl ThemeColorScheme {
    /// Resolve a theme color by index (0–11), returning an 8-char ARGB hex string
    /// with `"FF"` alpha prefix. Applies tint when `tint` is `Some`.
    pub fn resolve_theme(&self, index: usize, tint: Option<f64>) -> Option<String> {
        if index >= 12 {
            return None;
        }
        let rgb = &self.entries[index];
        let result = match tint {
            Some(t) => apply_tint(rgb, t),
            None => rgb.clone(),
        };
        Some(format!("FF{result}"))
    }

    /// Resolve an indexed color (0–55), returning an 8-char ARGB hex string
    /// with `"FF"` alpha prefix.  Returns `None` for out-of-range indices.
    pub fn resolve_indexed(&self, index: usize) -> Option<String> {
        match &self.indexed {
            Some(palette) => palette.get(index).map(|rgb| format!("FF{rgb}")),
            None => SYSTEM_INDEXED_COLORS.get(index).map(|rgb| format!("FF{rgb}")),
        }
    }

    /// Serialize the 12 theme color entries into a minimal valid
    /// `<a:clrScheme>` OOXML fragment (v0.13.0).
    ///
    /// Mirrors the element names that `from_xml` parses: each of `dk1`,
    /// `lt1`, `dk2`, `lt2`, `accent1`..`accent6`, `hlink`, `folHlink` wraps
    /// an `<a:srgbClr val="RRGGBB"/>`.  ExcelJS (and `from_xml`) resolve
    /// theme indices 0–11 from these 12 values; no `<a:theme>` wrapper is
    /// required.
    pub fn to_xml(&self) -> String {
        let e = &self.entries;
        format!(
            r#"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<a:clrScheme xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" name=\"Office\">\n<a:dk1><a:srgbClr val=\"{}\"/></a:dk1>\n<a:lt1><a:srgbClr val=\"{}\"/></a:lt1>\n<a:dk2><a:srgbClr val=\"{}\"/></a:dk2>\n<a:lt2><a:srgbClr val=\"{}\"/></a:lt2>\n<a:accent1><a:srgbClr val=\"{}\"/></a:accent1>\n<a:accent2><a:srgbClr val=\"{}\"/></a:accent2>\n<a:accent3><a:srgbClr val=\"{}\"/></a:accent3>\n<a:accent4><a:srgbClr val=\"{}\"/></a:accent4>\n<a:accent5><a:srgbClr val=\"{}\"/></a:accent5>\n<a:accent6><a:srgbClr val=\"{}\"/></a:accent6>\n<a:hlink><a:srgbClr val=\"{}\"/></a:hlink>\n<a:folHlink><a:srgbClr val=\"{}\"/></a:folHlink>\n</a:clrScheme>"#,
            e[0], e[1], e[2], e[3], e[4], e[5], e[6], e[7], e[8], e[9], e[10], e[11]
        )
    }

    /// Parse a `<a:clrScheme>` XML fragment and return a `ThemeColorScheme`.
    ///
    /// Reads each child element (e.g. `<a:dk1><a:srgbClr val="XXXXXX"/>`) to
    /// populate the 12 theme slots.  If `<a:indexedColors>` is present, its 56
    /// `<a:rgbColor val="XXXXXXXX"/>` children override the default palette.
    pub fn from_xml(data: &str) -> Result<Self, ExcelrsError> {
        let mut reader = XmlReader::from_str(data);

        // Start with ECMA-376 defaults
        let defaults = ThemeColorScheme::default();

        // 12 named slots in OOXML order
        let mut entries = defaults.entries;
        let mut indexed_override: Option<[String; 56]> = None;

        let mut in_clr_scheme = false;
        let mut in_indexed_colors = false;
        let mut indexed_acc: Vec<String> = Vec::with_capacity(56);
        let mut current_entry_name: Option<String> = None;

        loop {
            match reader.read_event() {
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                    let tag = e.local_name().as_ref().to_vec();

                    if tag == b"clrScheme" {
                        in_clr_scheme = true;
                        continue;
                    }
                    if !in_clr_scheme {
                        continue;
                    }

                    if tag == b"indexedColors" {
                        in_indexed_colors = true;
                        indexed_acc.clear();
                        continue;
                    }

                    if in_indexed_colors {
                        if tag == b"rgbColor" {
                            let attrs: Vec<_> = e.attributes().filter_map(|a| a.ok()).collect();
                            if let Some(val) = attrs.iter().find(|a| a.key.local_name().as_ref() == b"val") {
                                let raw = std::str::from_utf8(val.value.as_ref()).unwrap_or("");
                                // rgbColor val is 8-char ARGB like "00FFFFFF" → strip alpha → "FFFFFF"
                                if raw.len() == 8 && raw[2..8].chars().all(|c| c.is_ascii_hexdigit()) {
                                    indexed_acc.push(raw[2..8].to_uppercase());
                                }
                            }
                        }
                        continue;
                    }

                    // Theme color child element — record its name so nested
                    // srgbClr/sysClr events can identify which slot to fill.
                    match &*tag {
                        b"dk1" | b"lt1" | b"dk2" | b"lt2" | b"accent1" | b"accent2" | b"accent3" | b"accent4"
                        | b"accent5" | b"accent6" | b"hlink" | b"folHlink" => {
                            current_entry_name = Some(String::from_utf8_lossy(&tag).to_string());
                            continue;
                        }
                        _ => {}
                    }

                    // srgbClr or sysClr inside a theme element
                    if tag == b"srgbClr" || tag == b"sysClr" {
                        if let Some(idx) = current_entry_name.as_ref().and_then(|name| match name.as_str() {
                            "dk1" => Some(0),
                            "lt1" => Some(1),
                            "dk2" => Some(2),
                            "lt2" => Some(3),
                            "accent1" => Some(4),
                            "accent2" => Some(5),
                            "accent3" => Some(6),
                            "accent4" => Some(7),
                            "accent5" => Some(8),
                            "accent6" => Some(9),
                            "hlink" => Some(10),
                            "folHlink" => Some(11),
                            _ => None,
                        }) {
                            if tag == b"srgbClr" {
                                let attrs: Vec<_> = e.attributes().filter_map(|a| a.ok()).collect();
                                if let Some(val) = attrs.iter().find(|a| a.key.local_name().as_ref() == b"val") {
                                    let raw = std::str::from_utf8(val.value.as_ref()).unwrap_or("");
                                    if raw.len() == 6 && raw.chars().all(|c| c.is_ascii_hexdigit()) {
                                        entries[idx] = raw.to_uppercase();
                                    }
                                }
                            }
                        }
                        continue;
                    }
                }

                Ok(Event::End(ref e)) => {
                    let tag = e.local_name().as_ref().to_vec();

                    if tag == b"clrScheme" {
                        break;
                    }
                    if tag == b"indexedColors" {
                        in_indexed_colors = false;
                        if !indexed_acc.is_empty() {
                            let arr: [String; 56] = std::array::from_fn(|i| {
                                indexed_acc.get(i).cloned().unwrap_or_else(|| {
                                    // Fall back to default system palette for entries not provided
                                    SYSTEM_INDEXED_COLORS.get(i).map(|s| s.to_string()).unwrap_or_default()
                                })
                            });
                            indexed_override = Some(arr);
                        }
                        continue;
                    }
                    // Reset current_entry_name when leaving a theme child element
                    match &*tag {
                        b"dk1" | b"lt1" | b"dk2" | b"lt2" | b"accent1" | b"accent2" | b"accent3" | b"accent4"
                        | b"accent5" | b"accent6" | b"hlink" | b"folHlink" => {
                            current_entry_name = None;
                        }
                        _ => {}
                    }
                }

                Ok(Event::Eof) => break,

                Err(e) => {
                    return Err(ExcelrsError::Parse(format!("Failed to parse clrScheme XML: {e}")));
                }

                _ => {}
            }
        }

        Ok(ThemeColorScheme {
            entries,
            indexed: indexed_override,
        })
    }
}

// ---------------------------------------------------------------------------
// Tint algorithm (private)
// ---------------------------------------------------------------------------

/// Apply the OOXML tint transformation to a 6-char RGB string.
///
/// For each RGB channel `c` (0..255) and tint `t`:
/// - `t < 0` (darken): `c' = round(c * (1.0 + t))`
/// - `t >= 0` (lighten): `c' = round(c + (255.0 - c) * t)`
///
/// Results are clamped to [0, 255] and formatted as 6-char RGB hex.
fn apply_tint(rgb: &str, tint: f64) -> String {
    let r = u8::from_str_radix(&rgb[0..2], 16).unwrap_or(0) as f64;
    let g = u8::from_str_radix(&rgb[2..4], 16).unwrap_or(0) as f64;
    let b = u8::from_str_radix(&rgb[4..6], 16).unwrap_or(0) as f64;

    let tint_rgb = |c: f64| -> u8 {
        let computed = if tint < 0.0 {
            c * (1.0 + tint)
        } else {
            c + (255.0 - c) * tint
        };
        computed.round().clamp(0.0, 255.0) as u8
    };

    format!("{:02X}{:02X}{:02X}", tint_rgb(r), tint_rgb(g), tint_rgb(b))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- A1: Default scheme length --
    #[test]
    fn test_scheme_default_len_12() {
        assert_eq!(ThemeColorScheme::default().entries.len(), 12);
    }

    // -- A2: Default accent1 value --
    #[test]
    fn test_scheme_default_accent1() {
        assert_eq!(ThemeColorScheme::default().entries[4], "4F81BD");
    }

    // -- A3: Resolve theme lt1 (index 1 = white) --
    #[test]
    fn test_resolve_theme_lt1() {
        let scheme = ThemeColorScheme::default();
        assert_eq!(scheme.resolve_theme(1, None), Some("FFFFFFFF".into()));
    }

    // -- A4: Resolve theme accent1 (index 4) --
    #[test]
    fn test_resolve_theme_accent1() {
        let scheme = ThemeColorScheme::default();
        assert_eq!(scheme.resolve_theme(4, None), Some("FF4F81BD".into()));
    }

    // -- A5: Resolve theme dk2 (index 2) --
    #[test]
    fn test_resolve_theme_dk2() {
        let scheme = ThemeColorScheme::default();
        assert_eq!(scheme.resolve_theme(2, None), Some("FF1F497D".into()));
    }

    // -- A6: Resolve theme out-of-range --
    #[test]
    fn test_resolve_theme_out_of_range() {
        let scheme = ThemeColorScheme::default();
        assert!(scheme.resolve_theme(12, None).is_none());
        assert!(scheme.resolve_theme(99, None).is_none());
    }

    // -- A7: apply_tint darken --
    #[test]
    fn test_apply_tint_darken() {
        assert_eq!(apply_tint("FF0000", -0.5), "800000");
    }

    // -- A8: apply_tint lighten --
    #[test]
    fn test_apply_tint_lighten() {
        assert_eq!(apply_tint("000000", 0.5), "808080");
    }

    // -- A9: apply_tint zero is noop --
    #[test]
    fn test_apply_tint_zero_noop() {
        assert_eq!(apply_tint("4F81BD", 0.0), "4F81BD");
    }

    // -- A10: resolve_theme with tint --
    #[test]
    fn test_resolve_theme_with_tint() {
        let scheme = ThemeColorScheme::default();
        // accent1 = "4F81BD", tint = -0.5
        // R: round(79 * 0.5) = round(39.5) = 40 = 0x28
        // G: round(129 * 0.5) = round(64.5) = 65 = 0x41
        // B: round(189 * 0.5) = round(94.5) = 95 = 0x5F
        assert_eq!(scheme.resolve_theme(4, Some(-0.5)), Some("FF28415F".into()));
    }

    // -- A11: from_xml parses clrScheme --
    #[test]
    fn test_from_xml_parses_clr_scheme() {
        let xml = r#"<a:clrScheme name="Custom">
            <a:dk1><a:srgbClr val="123456"/></a:dk1>
            <a:lt1><a:srgbClr val="ABCDEF"/></a:lt1>
            <a:dk2><a:srgbClr val="1F497D"/></a:dk2>
            <a:lt2><a:srgbClr val="EEECE1"/></a:lt2>
            <a:accent1><a:srgbClr val="4F81BD"/></a:accent1>
            <a:accent2><a:srgbClr val="C0504D"/></a:accent2>
            <a:accent3><a:srgbClr val="9BBB59"/></a:accent3>
            <a:accent4><a:srgbClr val="F79646"/></a:accent4>
            <a:accent5><a:srgbClr val="8064A2"/></a:accent5>
            <a:accent6><a:srgbClr val="4BACC6"/></a:accent6>
            <a:hlink><a:srgbClr val="0000FF"/></a:hlink>
            <a:folHlink><a:srgbClr val="800080"/></a:folHlink>
        </a:clrScheme>"#;
        let scheme = ThemeColorScheme::from_xml(xml).unwrap();
        assert_eq!(scheme.resolve_theme(0, None), Some("FF123456".into()));
        assert_eq!(scheme.resolve_theme(1, None), Some("FFABCDEF".into()));
    }

    // -- A12: from_xml custom accent1 --
    #[test]
    fn test_from_xml_custom_accent1() {
        let xml = r#"<a:clrScheme name="RedTheme">
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
        </a:clrScheme>"#;
        let scheme = ThemeColorScheme::from_xml(xml).unwrap();
        assert_eq!(scheme.resolve_theme(4, None), Some("FF123456".into()));
    }

    // -- A13: resolve_indexed default (system palette) --
    #[test]
    fn test_resolve_indexed_default() {
        let scheme = ThemeColorScheme::default();
        // Index 0 = "000000" → "FF000000"
        assert_eq!(scheme.resolve_indexed(0), Some("FF000000".into()));
        // Index 2 = "FF0000" → "FFFF0000"
        assert_eq!(scheme.resolve_indexed(2), Some("FFFF0000".into()));
        // Index 16 = "800000" → "FF800000"
        assert_eq!(scheme.resolve_indexed(16), Some("FF800000".into()));
    }

    // -- A14: resolve_indexed custom override via from_xml --
    #[test]
    fn test_resolve_indexed_custom_override() {
        // Build a clrScheme that includes <a:indexedColors> with 56 entries
        let mut xml = r#"<a:clrScheme name="Custom">
            <a:dk1><a:srgbClr val="000000"/></a:dk1>
            <a:lt1><a:srgbClr val="FFFFFF"/></a:lt1>
            <a:dk2><a:srgbClr val="1F497D"/></a:dk2>
            <a:lt2><a:srgbClr val="EEECE1"/></a:lt2>
            <a:accent1><a:srgbClr val="4F81BD"/></a:accent1>
            <a:accent2><a:srgbClr val="C0504D"/></a:accent2>
            <a:accent3><a:srgbClr val="9BBB59"/></a:accent3>
            <a:accent4><a:srgbClr val="F79646"/></a:accent4>
            <a:accent5><a:srgbClr val="8064A2"/></a:accent5>
            <a:accent6><a:srgbClr val="4BACC6"/></a:accent6>
            <a:hlink><a:srgbClr val="0000FF"/></a:hlink>
            <a:folHlink><a:srgbClr val="800080"/></a:folHlink>
            <a:indexedColors>"#
            .to_string();

        // 56 indexed entries: default palette but override index 2
        let default_palette: [&str; 56] = [
            "000000", "FFFFFF", "FF0000", "00FF00", "0000FF", "FFFF00", "FF00FF", "00FFFF", "000000", "FFFFFF",
            "FF0000", "00FF00", "0000FF", "FFFF00", "FF00FF", "00FFFF", "800000", "008000", "000080", "808000",
            "800080", "008080", "C0C0C0", "808080", "9999FF", "993366", "FFFFCC", "CCFFFF", "660066", "FF8080",
            "0066CC", "CCCCFF", "000080", "FF00FF", "FFFF00", "00FFFF", "800080", "800000", "008080", "0000FF",
            "00CCFF", "CCFFFF", "CCFFCC", "FFFF99", "99CCFF", "FF99CC", "CC99FF", "FFCC99", "3366FF", "33CCCC",
            "99CC00", "FFCC00", "FF9900", "FF6600", "666699", "969696",
        ];

        // Override index 2 with ABCDEF
        let mut overridden = default_palette;
        overridden[2] = "ABCDEF";

        for entry in &overridden {
            // rgbColor val is 8-char ARGB: "00" + 6-char RGB
            xml.push_str(&format!("<a:rgbColor val=\"00{}\"/>", entry));
        }
        xml.push_str("</a:indexedColors></a:clrScheme>");

        let scheme = ThemeColorScheme::from_xml(&xml).unwrap();
        // Index 2 was overridden
        assert_eq!(scheme.resolve_indexed(2), Some("FFABCDEF".into()));
        // Index 0 should be default
        assert_eq!(scheme.resolve_indexed(0), Some("FF000000".into()));
    }

    // -- A15: resolve_indexed out of range --
    #[test]
    fn test_resolve_indexed_out_of_range() {
        let scheme = ThemeColorScheme::default();
        assert!(scheme.resolve_indexed(56).is_none());
        assert!(scheme.resolve_indexed(99).is_none());
    }

    /// B1: to_xml round-trips through from_xml for the default scheme.
    #[test]
    fn test_to_xml_default_round_trip() {
        let scheme = ThemeColorScheme::default();
        let xml = scheme.to_xml();
        assert!(xml.contains(r#"<a:clrScheme"#));
        assert!(xml.contains(r##"name=\"Office\""##));
        assert!(xml.contains(r##"<a:accent1><a:srgbClr val=\"4F81BD\"/>"##));
        // Re-parse: accent1 (index 4) and dk1 (index 0) must survive round-trip
        let parsed = ThemeColorScheme::from_xml(&xml).unwrap();
        assert_eq!(parsed.resolve_theme(4, None), Some("FF4F81BD".into()));
        assert_eq!(parsed.resolve_theme(0, None), Some("FF000000".into()));
    }

    /// A16: rgbColor val with alpha prefix strips correctly and validates hex.
    #[test]
    fn test_from_xml_rgbcolor_strips_alpha() {
        let xml = r#"<a:clrScheme name="Test">
            <a:dk1><a:srgbClr val="000000"/></a:dk1>
            <a:lt1><a:srgbClr val="FFFFFF"/></a:lt1>
            <a:dk2><a:srgbClr val="1F497D"/></a:dk2>
            <a:lt2><a:srgbClr val="EEECE1"/></a:lt2>
            <a:accent1><a:srgbClr val="4F81BD"/></a:accent1>
            <a:accent2><a:srgbClr val="C0504D"/></a:accent2>
            <a:accent3><a:srgbClr val="9BBB59"/></a:accent3>
            <a:accent4><a:srgbClr val="F79646"/></a:accent4>
            <a:accent5><a:srgbClr val="8064A2"/></a:accent5>
            <a:accent6><a:srgbClr val="4BACC6"/></a:accent6>
            <a:hlink><a:srgbClr val="0000FF"/></a:hlink>
            <a:folHlink><a:srgbClr val="800080"/></a:folHlink>
            <a:indexedColors>
                <a:rgbColor val="00ABCDEF"/>
            </a:indexedColors>
        </a:clrScheme>"#;
        let scheme = ThemeColorScheme::from_xml(xml).unwrap();
        assert_eq!(scheme.resolve_indexed(0), Some("FFABCDEF".into()));
    }
}
