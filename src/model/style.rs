//! Style types: `Font`, `Fill`, `BorderStyle`, `Border`, `Alignment`, `Style`.
//!
//! All six structs are `#[napi(object)]` flat structs (ADR-11). Colors are ARGB
//! hex (8 chars) or RGB hex (6 chars). Theme color references are not supported
//! in v0.2.0 (deferred to v0.3.0, see spec §9.2.1).
//!
//! # Setter validation
//! `Style::validate()` runs seven canonical-serialization rules from spec §6.8:
//! color regex, float finiteness, string case preservation, optional-semantics
//! (`None` ≠ `Some("")`), `Fill.kind` enum, `BorderStyle.style` enum, and
//! `num_fmt` non-empty. Returns the validated (uppercased-color) `Style` on
//! success, or `Err(ExcelrsError::InvalidStyle)` on the first violation.

use napi_derive::napi;

use crate::error::ExcelrsError;

// ---------------------------------------------------------------------------
// Font
// ---------------------------------------------------------------------------

/// Font properties: name, size (points), weight, style, and color.
#[napi(object)]
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Font {
    /// Font name (e.g. "Calibri", "Arial"). Default: "Calibri".
    pub name: Option<String>,
    /// Font size in points. Default: 11. Must be finite.
    pub size: Option<f64>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underline: Option<bool>,
    /// ARGB hex (8 chars) or RGB hex (6 chars). Default: None.
    pub color: Option<String>,
}

impl Default for Font {
    fn default() -> Self {
        Font {
            name: Some("Calibri".into()),
            size: Some(11.0),
            bold: None,
            italic: None,
            underline: None,
            color: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Fill
// ---------------------------------------------------------------------------

/// Cell fill: kind, foreground, background, and pattern.
#[napi(object)]
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Fill {
    /// Fill kind: `"none"` | `"solid"` | `"pattern"`.
    /// `"gradient"` is rejected in v0.2.0 (see spec §9.2.1).
    pub kind: String,
    /// Foreground color (ARGB hex). Default: None.
    pub foreground: Option<String>,
    /// Background color (ARGB hex). Default: None.
    pub background: Option<String>,
    /// Pattern name (for `kind="pattern"`). Default: None.
    pub pattern: Option<String>,
}

impl Default for Fill {
    fn default() -> Self {
        Fill {
            kind: "none".into(),
            foreground: None,
            background: None,
            pattern: None,
        }
    }
}

// ---------------------------------------------------------------------------
// BorderStyle
// ---------------------------------------------------------------------------

/// Border line style and color for one side of a cell border.
#[napi(object)]
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct BorderStyle {
    /// Border style: `"thin"` | `"medium"` | `"thick"` | `"dashed"` |
    /// `"dotted"` | `"double"`. `"none"` is rejected; use `None` for
    /// the border side (e.g. `Border.top = None`) to express no border.
    pub style: String,
    /// Line color (ARGB hex). Default: black (`"FF000000"` in exceljs).
    pub color: Option<String>,
}

// ---------------------------------------------------------------------------
// Border
// ---------------------------------------------------------------------------

/// All four cell-border sides. Each side is optional; `None` means no border.
#[napi(object)]
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Border {
    pub top: Option<BorderStyle>,
    pub right: Option<BorderStyle>,
    pub bottom: Option<BorderStyle>,
    pub left: Option<BorderStyle>,
}

// ---------------------------------------------------------------------------
// Alignment
// ---------------------------------------------------------------------------

/// Cell content alignment and text wrapping.
#[napi(object)]
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Alignment {
    /// Horizontal: `"left"` | `"center"` | `"right"` | `"fill"` | `"justify"`.
    pub horizontal: Option<String>,
    /// Vertical: `"top"` | `"middle"` | `"bottom"`.
    pub vertical: Option<String>,
    pub wrap_text: Option<bool>,
    pub indent: Option<u32>,
}

// ---------------------------------------------------------------------------
// Style (aggregate)
// ---------------------------------------------------------------------------

/// Aggregate style object. Each sub-field is optional; `None` means
/// that aspect of formatting is left at the built-in "Normal" default.
///
/// **Semantics: full-replace.** Assigning a new `Style` replaces the
/// existing style entirely. Use the spread idiom (spec §6.9) to preserve
/// specific fields.
#[napi(object)]
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Style {
    pub font: Option<Font>,
    pub fill: Option<Fill>,
    pub border: Option<Border>,
    pub alignment: Option<Alignment>,
    /// Format code string, e.g. `"0.00%"`, `"$#,##0.00"`, `"yyyy-mm-dd"`.
    /// `None` means no format (Normal). `Some("")` is rejected.
    pub num_fmt: Option<String>,
}

// ---------------------------------------------------------------------------
// Validation helpers
// ---------------------------------------------------------------------------

/// Check that a hex color string is 6-char (RGB) or 8-char (ARGB)
/// and contains only ASCII hex digits. Returns `true` if valid.
fn is_valid_hex_color(s: &str) -> bool {
    if s.len() != 6 && s.len() != 8 {
        return false;
    }
    s.chars().all(|c| c.is_ascii_hexdigit())
}

fn validate_color(color: &mut Option<String>, field: &str) -> Result<(), ExcelrsError> {
    if let Some(c) = color {
        if c.is_empty() {
            return Err(ExcelrsError::InvalidStyle(format!(
                "{field}: empty string is not a valid color; use null for no color"
            )));
        }
        if !is_valid_hex_color(c) {
            return Err(ExcelrsError::InvalidStyle(format!(
                "{field}: '{c}' is not a valid ARGB/RGB hex string; expected 6-char RGB or 8-char ARGB"
            )));
        }
        *c = c.to_uppercase();
    }
    Ok(())
}

fn validate_float(val: Option<f64>, field: &str) -> Result<(), ExcelrsError> {
    if let Some(v) = val {
        if !v.is_finite() {
            return Err(ExcelrsError::InvalidStyle(format!(
                "{field}: {v} is not a finite number (NaN and ±Infinity are rejected)"
            )));
        }
    }
    Ok(())
}

/// Validate Fill.kind. Must be one of: "none", "solid", "pattern".
/// "gradient" is explicitly rejected with a clear message.
fn validate_fill_kind(kind: &str) -> Result<(), ExcelrsError> {
    match kind {
        "none" | "solid" | "pattern" => Ok(()),
        "gradient" => Err(ExcelrsError::InvalidStyle(
            "fill.kind: 'gradient' is not supported in v0.2.0 (see spec §9.2.1)".into(),
        )),
        other => Err(ExcelrsError::InvalidStyle(format!(
            "fill.kind: '{other}' is not valid; use 'none', 'solid', or 'pattern'"
        ))),
    }
}

/// Validate BorderStyle.style. Must be one of the OOXML line styles.
/// "none" is rejected; use `None` for the border side.
fn validate_border_style(style: &str) -> Result<(), ExcelrsError> {
    match style {
        "thin" | "medium" | "thick" | "dashed" | "dotted" | "double" => Ok(()),
        "none" => Err(ExcelrsError::InvalidStyle(
            "border.style: 'none' is not a valid border style; use null for the border side \
             (e.g. Border.top = null) to express 'no border'"
                .into(),
        )),
        other => Err(ExcelrsError::InvalidStyle(format!(
            "border.style: '{other}' is not a valid border style; \
             use 'thin', 'medium', 'thick', 'dashed', 'dotted', or 'double'"
        ))),
    }
}

/// Check that num_fmt is not `Some("")`.
fn validate_num_fmt(num_fmt: &Option<String>) -> Result<(), ExcelrsError> {
    if let Some(fmt) = num_fmt {
        if fmt.is_empty() {
            return Err(ExcelrsError::InvalidStyle(
                "num_fmt: empty string is not a valid format code; use null for no format".into(),
            ));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Style::validate
// ---------------------------------------------------------------------------

impl Style {
    /// Run all seven setter-validation rules (spec §6.8).
    ///
    /// 1. Color strings → match regex, uppercase.
    /// 2. Float fields → must be finite.
    /// 3. String fields → preserved as-given (no transformation).
    /// 4. Optional fields → `None` ≠ `Some("")`.
    /// 5. `Fill.kind` → enum allowlist.
    /// 6. `BorderStyle.style` → enum allowlist.
    /// 7. `num_fmt` → non-empty.
    pub fn validate(mut self) -> Result<Self, ExcelrsError> {
        // Font
        if let Some(ref mut font) = self.font {
            validate_float(font.size, "font.size")?;
            validate_color(&mut font.color, "font.color")?;
        }

        // Fill
        if let Some(ref mut fill) = self.fill {
            validate_fill_kind(&fill.kind)?;
            validate_color(&mut fill.foreground, "fill.foreground")?;
            validate_color(&mut fill.background, "fill.background")?;
        }

        // Border (each side)
        if let Some(ref mut border) = self.border {
            for (side, bs) in [
                ("border.top", &mut border.top),
                ("border.right", &mut border.right),
                ("border.bottom", &mut border.bottom),
                ("border.left", &mut border.left),
            ] {
                if let Some(ref mut bs) = bs {
                    validate_border_style(&bs.style)?;
                    validate_color(&mut bs.color, &format!("{side}.color"))?;
                }
            }
        }

        // num_fmt
        validate_num_fmt(&self.num_fmt)?;

        Ok(self)
    }

    /// Whether this style is "empty" — all fields are `None`.
    /// Used internally to detect `{}` (empty object) → Normal.
    pub fn is_empty(&self) -> bool {
        self.font.is_none()
            && self.fill.is_none()
            && self.border.is_none()
            && self.alignment.is_none()
            && self.num_fmt.is_none()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- 6 type construction tests --

    #[test]
    fn test_font_default() {
        let f = Font::default();
        assert_eq!(f.name, Some("Calibri".into()));
        assert_eq!(f.size, Some(11.0));
        assert!(f.bold.is_none());
        assert!(f.italic.is_none());
        assert!(f.underline.is_none());
        assert!(f.color.is_none());
    }

    #[test]
    fn test_fill_default() {
        let f = Fill::default();
        assert_eq!(f.kind, "none");
        assert!(f.foreground.is_none());
        assert!(f.background.is_none());
        assert!(f.pattern.is_none());
    }

    #[test]
    fn test_border_style_default() {
        let bs = BorderStyle::default();
        assert_eq!(bs.style, "");
        assert!(bs.color.is_none());
    }

    #[test]
    fn test_border_default() {
        let b = Border::default();
        assert!(b.top.is_none());
        assert!(b.right.is_none());
        assert!(b.bottom.is_none());
        assert!(b.left.is_none());
    }

    #[test]
    fn test_alignment_default() {
        let a = Alignment::default();
        assert!(a.horizontal.is_none());
        assert!(a.vertical.is_none());
        assert!(a.wrap_text.is_none());
        assert!(a.indent.is_none());
    }

    #[test]
    fn test_style_default() {
        let s = Style::default();
        assert!(s.font.is_none());
        assert!(s.fill.is_none());
        assert!(s.border.is_none());
        assert!(s.alignment.is_none());
        assert!(s.num_fmt.is_none());
        assert!(s.is_empty());
    }

    // -- 8 setter validation tests --

    /// Positive: a fully-populated valid Style is accepted.
    #[test]
    fn test_validate_style_valid() {
        let raw = serde_json::json!({
            "font": { "bold": true, "size": 14, "color": "FFFF0000" },
            "fill": { "kind": "solid", "foreground": "FFFFFF00" },
            "border": {
                "top": { "style": "thin", "color": "FF000000" },
                "bottom": { "style": "thin", "color": "FF000000" }
            },
            "alignment": { "horizontal": "center", "vertical": "middle" },
            "num_fmt": "0.00%"
        });
        let style: Style = serde_json::from_value(raw).unwrap();
        let validated = style.validate().unwrap();
        assert_eq!(validated.num_fmt, Some("0.00%".into()));
        assert_eq!(
            validated.font.as_ref().unwrap().bold,
            Some(true)
        );
    }

    /// Null → Normal reset.
    #[test]
    fn test_validate_style_null() {
        // via Style directly: is_empty checks
        let s = Style::default();
        assert!(s.is_empty());
    }

    /// Empty object → Normal (no error, empty).
    #[test]
    fn test_validate_style_empty_object() {
        // Constructed from an empty JSON object → all defaults (Option=None)
        // because #[serde(default)] fills missing fields with Default::default()
        let style: Style = serde_json::from_value(serde_json::json!({})).unwrap();
        assert!(style.is_empty());
        let validated = style.validate().unwrap();
        assert!(validated.is_empty());
    }

    /// Color: invalid hex string rejected.
    #[test]
    fn test_validate_color_invalid() {
        let raw = serde_json::json!({
            "font": { "color": "ZZZZZZ" }
        });
        let style: Style = serde_json::from_value(raw).unwrap();
        let result = style.validate();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("font.color"));
        assert!(err.contains("ZZZZZZ"));
    }

    /// Color: 5-char string rejected.
    #[test]
    fn test_validate_color_too_short() {
        let raw = serde_json::json!({
            "font": { "color": "FFFFF" }
        });
        let style: Style = serde_json::from_value(raw).unwrap();
        assert!(style.validate().is_err());
    }

    /// Color: input is uppercased before storing.
    #[test]
    fn test_validate_color_uppercased() {
        let raw = serde_json::json!({
            "font": { "color": "ff0000" }
        });
        let style: Style = serde_json::from_value(raw).unwrap();
        let validated = style.validate().unwrap();
        assert_eq!(
            validated.font.unwrap().color,
            Some("FF0000".into())
        );
    }

    /// Float: NaN rejected.
    #[test]
    fn test_validate_font_size_nan() {
        let style = Style {
            font: Some(Font {
                size: Some(f64::NAN),
                ..Default::default()
            }),
            ..Default::default()
        };
        let result = style.validate();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("font.size"));
        assert!(err.contains("NaN"));
    }

    /// Float: infinity rejected.
    #[test]
    fn test_validate_font_size_inf() {
        let style = Style {
            font: Some(Font {
                size: Some(f64::INFINITY),
                ..Default::default()
            }),
            ..Default::default()
        };
        assert!(style.validate().is_err());
    }

    /// Fill.kind: "gradient" rejected with specific message.
    #[test]
    fn test_validate_fill_kind_gradient() {
        let fill = Fill {
            kind: "gradient".into(),
            ..Default::default()
        };
        let style = Style {
            fill: Some(fill),
            ..Default::default()
        };
        let result = style.validate();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("gradient"));
        assert!(err.contains("v0.2.0"));
    }

    /// Fill.kind: random string rejected.
    #[test]
    fn test_validate_fill_kind_bogus() {
        let fill = Fill {
            kind: "stripe".into(),
            ..Default::default()
        };
        let style = Style {
            fill: Some(fill),
            ..Default::default()
        };
        assert!(style.validate().is_err());
    }

    /// BorderStyle.style: "none" rejected.
    #[test]
    fn test_validate_border_style_none() {
        let bs = BorderStyle {
            style: "none".into(),
            color: None,
        };
        let border = Border {
            top: Some(bs),
            ..Default::default()
        };
        let style = Style {
            border: Some(border),
            ..Default::default()
        };
        let result = style.validate();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("none"));
    }

    /// BorderStyle.style: "bogus" rejected.
    #[test]
    fn test_validate_border_style_bogus() {
        let bs = BorderStyle {
            style: "xtrathick".into(),
            color: None,
        };
        let border = Border {
            top: Some(bs),
            ..Default::default()
        };
        let style = Style {
            border: Some(border),
            ..Default::default()
        };
        assert!(style.validate().is_err());
    }

    /// Border side = None: no validation needed (style field unreachable).
    #[test]
    fn test_validate_border_side_none_pass() {
        let border = Border::default(); // all sides None
        let style = Style {
            border: Some(border),
            ..Default::default()
        };
        // Should pass: no BorderStyle to validate
        assert!(style.validate().is_ok());
    }

    /// num_fmt: Some("") rejected.
    #[test]
    fn test_validate_num_fmt_empty() {
        let style = Style {
            num_fmt: Some(String::new()),
            ..Default::default()
        };
        let result = style.validate();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("empty string"));
    }

    /// num_fmt: Some("0.00%") accepted.
    #[test]
    fn test_validate_num_fmt_valid() {
        let style = Style {
            num_fmt: Some("0.00%".into()),
            ..Default::default()
        };
        assert!(style.validate().is_ok());
    }

    /// num_fmt: None accepted (normal).
    #[test]
    fn test_validate_num_fmt_none() {
        let style = Style::default();
        assert!(style.validate().is_ok());
    }

    /// Full-replace semantics: a second assignment overwrites the first.
    #[test]
    fn test_validate_style_full_replace() {
        // This test validates that serialize-deserialize round-trips correctly:
        // the Style object is fully replaced, not merged.
        let raw = serde_json::json!({
            "num_fmt": "0.00%"
        });
        let style: Style = serde_json::from_value(raw).unwrap();
        assert!(style.font.is_none()); // not filled in from any "previous" style
        assert!(style.fill.is_none());
        assert!(style.border.is_none());
        assert!(style.alignment.is_none());
        assert_eq!(style.num_fmt, Some("0.00%".into()));
    }
}
