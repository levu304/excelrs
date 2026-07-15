//! Style types: `Font`, `Fill`, `BorderStyle`, `Border`, `Alignment`, `Style`.
//!
//! All six structs are `#[napi(object)]` flat structs (ADR-11). Colors are ARGB
//! hex (8 chars) or RGB hex (6 chars). Theme color references (`theme="N"`) and indexed
//! colors (`indexed="N"`) in `xl/styles.xml` are resolved to ARGB on read via
//! `xl/theme/theme1.xml` and the standard 56-entry system-indexed palette.
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
    /// Internal: originating theme index (0–11) when `color` came from a
    /// `<color theme="N"/>` reference. Not exposed to JS; preserves the theme
    /// link on write. `None` for ARGB/RGB colors.
    #[napi(skip)]
    #[serde(skip)]
    pub color_theme: Option<u8>,
    /// Internal: tint applied to the theme color (`tint` attribute). Not exposed.
    #[napi(skip)]
    #[serde(skip)]
    pub color_tint: Option<f64>,
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
            color_theme: None,
            color_tint: None,
        }
    }
}

impl Font {
    /// Validate font fields that appear in rich-text runs.
    /// Called by `CellValue::validate()` before writing.
    pub fn validate(&mut self) -> Result<(), ExcelrsError> {
        validate_float(self.size, "rich_text.font.size")?;
        validate_color(&mut self.color, "rich_text.font.color")?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Fill
// ---------------------------------------------------------------------------

/// A single gradient stop: color + position.
#[napi(object)]
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct GradientStop {
    /// ARGB hex color (8 chars) or RGB hex (6 chars).
    pub color: String,
    /// Position in [0.0, 1.0].
    pub position: f64,
}

/// Cell fill: kind, foreground, background, and pattern.
#[napi(object)]
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Fill {
    /// Fill kind: `"none"` | `"solid"` | `"pattern"` | `"gradient"`.
    pub kind: String,
    /// Foreground color (ARGB hex). Default: None.
    pub foreground: Option<String>,
    /// Background color (ARGB hex). Default: None.
    pub background: Option<String>,
    /// Internal: theme index for `foreground` when it came from a theme ref.
    #[napi(skip)]
    #[serde(skip)]
    pub foreground_theme: Option<u8>,
    /// Internal: tint for `foreground`.
    #[napi(skip)]
    #[serde(skip)]
    pub foreground_tint: Option<f64>,
    /// Internal: theme index for `background` when it came from a theme ref.
    #[napi(skip)]
    #[serde(skip)]
    pub background_theme: Option<u8>,
    /// Internal: tint for `background`.
    #[napi(skip)]
    #[serde(skip)]
    pub background_tint: Option<f64>,
    /// Pattern name (for `kind="pattern"`). Default: None.
    pub pattern: Option<String>,
    // -- gradient fields (v0.5.0) --
    /// Gradient type: `"linear"` or `"path"`. Only used when `kind="gradient"`.
    pub gradient_type: Option<String>,
    /// Gradient angle in degrees (linear). Only used when `kind="gradient"`.
    pub gradient_degree: Option<f64>,
    /// Gradient angle as left/right angle (for path gradients). Only used when `kind="gradient"`.
    pub gradient_angle: Option<f64>,
    /// Gradient stops. Only used when `kind="gradient"`.
    pub gradient_stops: Option<Vec<GradientStop>>,
    // -- path gradient geometry (v0.5.0) --
    /// Left edge position (0.0–1.0) for path gradients. Only used when `kind="gradient"` and `gradient_type="path"`.
    pub gradient_left: Option<f64>,
    /// Right edge position (0.0–1.0) for path gradients. Only used when `kind="gradient"` and `gradient_type="path"`.
    pub gradient_right: Option<f64>,
    /// Top edge position (0.0–1.0) for path gradients. Only used when `kind="gradient"` and `gradient_type="path"`.
    pub gradient_top: Option<f64>,
    /// Bottom edge position (0.0–1.0) for path gradients. Only used when `kind="gradient"` and `gradient_type="path"`.
    pub gradient_bottom: Option<f64>,
}

impl Default for Fill {
    fn default() -> Self {
        Fill {
            kind: "none".into(),
            foreground: None,
            background: None,
            foreground_theme: None,
            foreground_tint: None,
            background_theme: None,
            background_tint: None,
            pattern: None,
            gradient_type: None,
            gradient_degree: None,
            gradient_angle: None,
            gradient_stops: None,
            gradient_left: None,
            gradient_right: None,
            gradient_top: None,
            gradient_bottom: None,
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
    /// Internal: originating theme index (0–11) for theme colors. Not exposed.
    #[napi(skip)]
    #[serde(skip)]
    pub color_theme: Option<u8>,
    /// Internal: tint applied to the theme color. Not exposed.
    #[napi(skip)]
    #[serde(skip)]
    pub color_tint: Option<f64>,
}

// ---------------------------------------------------------------------------
// Border
// ---------------------------------------------------------------------------

/// All cell-border sides plus diagonals. Each side is optional; `None` means no border.
#[napi(object)]
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Border {
    pub top: Option<BorderStyle>,
    pub right: Option<BorderStyle>,
    pub bottom: Option<BorderStyle>,
    pub left: Option<BorderStyle>,
    // -- diagonal borders (v0.5.0) --
    /// Diagonal border line style. Only valid edges between top-left ↔ bottom-right.
    pub diagonal: Option<BorderStyle>,
    /// Whether the diagonal line goes up (bottom-left to top-right).
    /// OOXML attribute `diagonalUp` on the `<border>` element.
    pub diagonal_up: Option<bool>,
    /// Whether the diagonal line goes down (top-left to bottom-right).
    /// OOXML attribute `diagonalDown` on the `<border>` element.
    pub diagonal_down: Option<bool>,
}

// ---------------------------------------------------------------------------
// Alignment
// ---------------------------------------------------------------------------

/// Cell content alignment and text wrapping.
#[napi(object)]
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
#[serde(default, rename_all = "camelCase")]
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
#[serde(default, rename_all = "camelCase")]
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

/// Validate float is finite AND within [min, max].
fn validate_float_range(val: Option<f64>, field: &str, min: f64, max: f64) -> Result<(), ExcelrsError> {
    validate_float(val, field)?;
    if let Some(v) = val {
        if !(min..=max).contains(&v) {
            return Err(ExcelrsError::InvalidStyle(format!(
                "{field}: {v} is outside the valid range [{min}, {max}]"
            )));
        }
    }
    Ok(())
}

/// Validate Fill.kind. Must be one of: "none", "solid", "pattern", "gradient".
fn validate_fill_kind(kind: &str) -> Result<(), ExcelrsError> {
    match kind {
        "none" | "solid" | "pattern" | "gradient" => Ok(()),
        other => Err(ExcelrsError::InvalidStyle(format!(
            "fill.kind: '{other}' is not valid; use 'none', 'solid', 'pattern', or 'gradient'"
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
            // Gradient validation
            if fill.kind == "gradient" {
                let gt = fill.gradient_type.as_deref();
                match gt {
                    Some("path") => {
                        // Path gradient: require left/right/top/bottom geometry
                        for (field, val) in [
                            ("fill.gradient_left", fill.gradient_left),
                            ("fill.gradient_right", fill.gradient_right),
                            ("fill.gradient_top", fill.gradient_top),
                            ("fill.gradient_bottom", fill.gradient_bottom),
                        ] {
                            validate_float_range(val, field, 0.0, 1.0)?;
                        }
                        if fill.gradient_left.is_none()
                            || fill.gradient_right.is_none()
                            || fill.gradient_top.is_none()
                            || fill.gradient_bottom.is_none()
                        {
                            return Err(ExcelrsError::InvalidStyle(
                                "fill: path gradient requires gradientLeft/Right/Top/Bottom".into(),
                            ));
                        }
                    }
                    Some("linear") | None => {
                        // Linear gradient: degree is optional
                        validate_float(fill.gradient_degree, "fill.gradient_degree")?;
                    }
                    Some(other) => {
                        return Err(ExcelrsError::InvalidStyle(format!(
                            "fill.gradient_type: '{other}' is not valid; use 'linear' or 'path'"
                        )));
                    }
                }
                // gradient_angle is deprecated and never emitted — intentionally not validated.
                // Set it and it is silently ignored (see emit_fills for the real gradient attributes).
                match &fill.gradient_stops {
                    Some(stops) if stops.len() >= 2 => {
                        for (i, stop) in stops.iter().enumerate() {
                            if !is_valid_hex_color(&stop.color) {
                                return Err(ExcelrsError::InvalidStyle(format!(
                                    "fill.gradient_stops[{}].color: '{}' is not a valid ARGB/RGB hex string",
                                    i, stop.color
                                )));
                            }
                            if !(0.0..=1.0).contains(&stop.position) {
                                return Err(ExcelrsError::InvalidStyle(format!(
                                    "fill.gradient_stops[{}].position: {} is outside [0.0, 1.0]",
                                    i, stop.position
                                )));
                            }
                        }
                    }
                    _ => {
                        return Err(ExcelrsError::InvalidStyle(
                            "fill.gradient_stops: a gradient fill requires at least 2 stops (positions in [0,1])"
                                .into(),
                        ));
                    }
                }
            }
        }

        // Border (each side including diagonal)
        if let Some(ref mut border) = self.border {
            for (side, bs) in [
                ("border.top", &mut border.top),
                ("border.right", &mut border.right),
                ("border.bottom", &mut border.bottom),
                ("border.left", &mut border.left),
                ("border.diagonal", &mut border.diagonal),
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
            "numFmt": "0.00%"
        });
        let style: Style = serde_json::from_value(raw).unwrap();
        let validated = style.validate().unwrap();
        assert_eq!(validated.num_fmt, Some("0.00%".into()));
        assert_eq!(validated.font.as_ref().unwrap().bold, Some(true));
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
        assert_eq!(validated.font.unwrap().color, Some("FF0000".into()));
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

    /// Fill.kind: "gradient" accepted in v0.5.0 with valid gradient fields.
    #[test]
    fn test_validate_fill_kind_gradient_accept() {
        let fill = Fill {
            kind: "gradient".into(),
            gradient_type: Some("linear".into()),
            gradient_degree: Some(90.0),
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
        let style = Style {
            fill: Some(fill),
            ..Default::default()
        };
        let result = style.validate();
        assert!(result.is_ok(), "gradient fill should be accepted: {:?}", result.err());
    }

    /// Gradient stop color validation
    #[test]
    fn test_validate_gradient_stop_invalid_color() {
        let fill = Fill {
            kind: "gradient".into(),
            gradient_type: Some("linear".into()),
            gradient_stops: Some(vec![GradientStop {
                color: "ZZZ".into(),
                position: 0.0,
            }]),
            ..Default::default()
        };
        let style = Style {
            fill: Some(fill),
            ..Default::default()
        };
        assert!(style.validate().is_err());
    }

    /// Gradient stop position out of range
    #[test]
    fn test_validate_gradient_stop_position_out_of_range() {
        let fill = Fill {
            kind: "gradient".into(),
            gradient_stops: Some(vec![GradientStop {
                color: "FFFF0000".into(),
                position: 1.5,
            }]),
            ..Default::default()
        };
        let style = Style {
            fill: Some(fill),
            ..Default::default()
        };
        assert!(style.validate().is_err());
    }

    /// P3b — Path gradient geometry not range-checked [0,1].
    #[test]
    fn test_validate_gradient_path_geometry_out_of_range() {
        let fill = Fill {
            kind: "gradient".into(),
            gradient_type: Some("path".into()),
            gradient_left: Some(2.0),
            gradient_right: Some(1.0),
            gradient_top: Some(0.0),
            gradient_bottom: Some(1.0),
            gradient_stops: Some(vec![
                GradientStop {
                    position: 0.0,
                    color: "FFFF0000".into(),
                },
                GradientStop {
                    position: 1.0,
                    color: "FF0000FF".into(),
                },
            ]),
            ..Default::default()
        };
        let style = Style {
            fill: Some(fill),
            ..Default::default()
        };
        assert!(
            style.validate().is_err(),
            "gradient_left=2.0 is outside [0,1] and must be rejected"
        );
    }

    /// P3d — Single gradient stop boundary test (regression lock-in).
    #[test]
    fn test_validate_gradient_single_stop_rejected() {
        let fill = Fill {
            kind: "gradient".into(),
            gradient_type: Some("linear".into()),
            gradient_degree: Some(90.0),
            gradient_stops: Some(vec![GradientStop {
                position: 0.0,
                color: "FFFF0000".into(),
            }]),
            ..Default::default()
        };
        let style = Style {
            fill: Some(fill),
            ..Default::default()
        };
        assert!(style.validate().is_err(), "exactly 1 gradient stop must be rejected");
    }

    /// Gradient with no stops must be rejected.
    #[test]
    fn test_validate_gradient_requires_stops() {
        let fill = Fill {
            kind: "gradient".into(),
            gradient_type: Some("linear".into()),
            gradient_degree: Some(90.0),
            gradient_stops: None,
            ..Default::default()
        };
        let style = Style {
            fill: Some(fill),
            ..Default::default()
        };
        assert!(style.validate().is_err(), "gradient with no stops should be rejected");
    }

    /// Gradient with an empty stop list must be rejected.
    #[test]
    fn test_validate_gradient_empty_stops_rejected() {
        let fill = Fill {
            kind: "gradient".into(),
            gradient_type: Some("linear".into()),
            gradient_stops: Some(vec![]),
            ..Default::default()
        };
        let style = Style {
            fill: Some(fill),
            ..Default::default()
        };
        assert!(
            style.validate().is_err(),
            "gradient with empty stops should be rejected"
        );
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
            ..Default::default()
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
            ..Default::default()
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
            "numFmt": "0.00%"
        });
        let style: Style = serde_json::from_value(raw).unwrap();
        assert!(style.font.is_none()); // not filled in from any "previous" style
        assert!(style.fill.is_none());
        assert!(style.border.is_none());
        assert!(style.alignment.is_none());
        assert_eq!(style.num_fmt, Some("0.00%".into()));
    }
}
