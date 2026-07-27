//! Embedded images / drawings (v1.0.0).

use napi::bindgen_prelude::*;
use napi::sys;
use napi_derive::napi;

/// Wrapper around `Vec<u8>` that accepts both `Buffer`/`TypedArray` and
/// `Array<number>` from JavaScript at the napi FFI boundary.
///
/// `Vec<u8>` in `#[napi(object)]` structs only accepts `Array<number>`.
/// This type tries `Buffer::from_napi_value` first, then falls back to
/// `Vec<u8>::from_napi_value`. The TS type is overridden to `Buffer` via
/// `#[napi(ts_type = "Buffer")]` on the field.
#[derive(Clone, Debug)]
pub struct NapiBuffer(pub Vec<u8>);

impl FromNapiValue for NapiBuffer {
    unsafe fn from_napi_value(env: sys::napi_env, napi_val: sys::napi_value) -> napi::Result<Self> {
        // Try Buffer first (handles Buffer and TypedArray)
        if let Ok(buf) = unsafe { <Buffer as FromNapiValue>::from_napi_value(env, napi_val) } {
            return Ok(NapiBuffer(buf.to_vec()));
        }
        // Fall back to Vec<u8> (handles Array<number>)
        unsafe { <Vec<u8> as FromNapiValue>::from_napi_value(env, napi_val) }.map(NapiBuffer)
    }
}

impl ToNapiValue for NapiBuffer {
    unsafe fn to_napi_value(env: sys::napi_env, val: Self) -> napi::Result<sys::napi_value> {
        // Returns a real Node.js Buffer (zero-copy external buffer with GC finalizer).
        // Per napi.rs docs: Buffer can be created from Vec<u8>; the external buffer's
        // finalizer frees the Vec on GC, with a copy fallback on Electron (V8 Memory Cage).
        let buf: Buffer = val.0.into();
        unsafe { <Buffer as ToNapiValue>::to_napi_value(env, buf) }
    }
}

/// A single anchor point (top-left or bottom-right corner).
/// Matches ExcelJS `{ col: number, row: number }` — fractions allowed.
#[napi(object)]
#[derive(Clone, Debug)]
pub struct AnchorPoint {
    pub col: f64,
    pub row: f64,
}

/// Explicit image size (EMU-converted). Matches ExcelJS `ext: { width, height }`.
#[napi(object)]
#[derive(Clone, Debug)]
pub struct ImageSize {
    pub width: f64,
    pub height: f64,
}

/// ExcelJS-shaped anchor input: `tl` plus exactly one of `br` (two-cell) or
/// `ext` (one-cell with explicit size). `anchorType` is inferred.
#[napi(object)]
#[derive(Clone, Debug)]
pub struct ImageAnchorInput {
    pub tl: AnchorPoint,
    pub br: Option<AnchorPoint>,
    pub ext: Option<ImageSize>,
}

/// Anchor type for embedded images.
#[napi(string_enum)]
#[derive(Clone, Debug, Default, PartialEq)]
pub enum AnchorType {
    #[default]
    OneCell,
    TwoCell,
}

impl std::fmt::Display for AnchorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnchorType::OneCell => write!(f, "oneCell"),
            AnchorType::TwoCell => write!(f, "twoCell"),
        }
    }
}

impl From<&str> for AnchorType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "twocell" => AnchorType::TwoCell,
            _ => AnchorType::OneCell,
        }
    }
}

/// Anchor descriptor for an embedded image.
///
/// `anchor_type` is `"oneCell"` (image pinned to a single cell + offset) or
/// `"twoCell"` (image spans from top-left to bottom-right corners). For
/// `"oneCell"`, use `col`/`row`/`x`/`y`; for `"twoCell"`, `col2`/`row2`/`x2`/`y2`
/// describe the bottom-right corner.
#[napi(object)]
#[derive(Clone, Debug)]
pub struct ImageAnchor {
    pub anchor_type: AnchorType,
    pub col: u32,
    pub row: u32,
    pub x: u32,
    pub y: u32,
    pub col2: u32,
    pub row2: u32,
    pub x2: u32,
    pub y2: u32,
}

#[napi(object)]
#[derive(Clone, Debug)]
pub struct AddImageOptions {
    pub extension: String,
    #[napi(ts_type = "Buffer")]
    pub buffer: NapiBuffer,
    pub image_type: Option<String>,
    pub positioning: Option<String>,
    pub anchor: ImageAnchorInput,
}

#[napi(object)]
#[derive(Clone, Debug)]
pub struct ImageInfo {
    pub extension: String,
    #[napi(ts_type = "Buffer")]
    pub buffer: NapiBuffer,
    pub positioning: String,
    pub anchor: ImageAnchorInput,
}

/// Internal image record stored on a worksheet (shared by writer & reader).
#[derive(Clone, Debug)]
pub struct WorksheetImage {
    pub extension: String,
    pub buffer: Vec<u8>,
    pub positioning: String,
    pub anchor: ImageAnchor,
    /// Explicit size (cx/cy in EMU) for one-cell anchors, or None for two-cell.
    pub ext_size: Option<(u32, u32)>,
    /// Globally-assigned media index (1-based); 0 until assigned by the writer.
    pub media_index: u32,
}

// -- Constants for fractional → EMU conversion (ExcelJS-compatible) --

const EMU_PER_PX: f64 = 9525.0;
const DEFAULT_COL_WIDTH_PX: f64 = 64.0;
const DEFAULT_ROW_HEIGHT_PX: f64 = 20.0;

// -- Public → internal conversion --

/// Result of converting an `ImageAnchorInput` to internal storage form.
pub(crate) struct InternalAnchor {
    pub anchor: ImageAnchor,
    /// Size (cx, cy in EMU) for one-cell anchors with explicit ext.
    pub ext_size: Option<(u32, u32)>,
}

impl ImageAnchorInput {
    /// Convert the ExcelJS-shaped anchor to the internal OOXML storage form.
    /// Returns an error when neither `br` nor `ext` is set, or both are set.
    pub fn to_internal(&self) -> napi::Result<InternalAnchor> {
        match (&self.br, &self.ext) {
            (None, None) => Err(napi::Error::from_reason(
                "ImageAnchorInput requires exactly one of `br` (two-cell) or `ext` (one-cell); got neither",
            )),
            (Some(_), Some(_)) => Err(napi::Error::from_reason(
                "ImageAnchorInput requires exactly one of `br` (two-cell) or `ext` (one-cell); got both",
            )),
            (Some(br), None) => {
                // Two-cell: tl → br
                let (col, x) = split_col_frac(self.tl.col);
                let (row, y) = split_row_frac(self.tl.row);
                let (col2, x2) = split_col_frac(br.col);
                let (row2, y2) = split_row_frac(br.row);
                Ok(InternalAnchor {
                    anchor: ImageAnchor {
                        anchor_type: AnchorType::TwoCell,
                        col,
                        row,
                        x,
                        y,
                        col2,
                        row2,
                        x2,
                        y2,
                    },
                    ext_size: None,
                })
            }
            (None, Some(ext)) => {
                // One-cell with explicit size
                let (col, x) = split_col_frac(self.tl.col);
                let (row, y) = split_row_frac(self.tl.row);
                let cx = (ext.width * EMU_PER_PX).round() as u32;
                let cy = (ext.height * EMU_PER_PX).round() as u32;
                Ok(InternalAnchor {
                    anchor: ImageAnchor {
                        anchor_type: AnchorType::OneCell,
                        col,
                        row,
                        x,
                        y,
                        col2: 0,
                        row2: 0,
                        x2: 0,
                        y2: 0,
                    },
                    ext_size: Some((cx, cy)),
                })
            }
        }
    }
}

// -- Internal → public conversion (read-back) --

impl ImageAnchor {
    /// Convert the internal OOXML anchor back to the ExcelJS-shaped form.
    pub fn to_exceljs_shape(&self, ext_size: Option<(u32, u32)>) -> ImageAnchorInput {
        let tl = AnchorPoint {
            col: reconstruct_float(self.col, self.x, DEFAULT_COL_WIDTH_PX),
            row: reconstruct_float(self.row, self.y, DEFAULT_ROW_HEIGHT_PX),
        };
        match self.anchor_type {
            AnchorType::TwoCell => ImageAnchorInput {
                tl: tl.clone(),
                br: Some(AnchorPoint {
                    col: reconstruct_float(self.col2, self.x2, DEFAULT_COL_WIDTH_PX),
                    row: reconstruct_float(self.row2, self.y2, DEFAULT_ROW_HEIGHT_PX),
                }),
                ext: None,
            },
            AnchorType::OneCell => {
                if let Some((cx, cy)) = ext_size {
                    ImageAnchorInput {
                        tl,
                        br: None,
                        ext: Some(ImageSize {
                            width: round_to_4dp(cx as f64 / EMU_PER_PX),
                            height: round_to_4dp(cy as f64 / EMU_PER_PX),
                        }),
                    }
                } else {
                    ImageAnchorInput {
                        tl,
                        br: None,
                        ext: None,
                    }
                }
            }
        }
    }
}

// -- Helpers --

/// Split a fractional column coordinate into integer + EMU offset.
/// E.g. `5.5` → `(5, 304800)` (0.5 * 64 * 9525).
fn split_col_frac(v: f64) -> (u32, u32) {
    let int = v.floor();
    let frac = v - int;
    let off = (frac * DEFAULT_COL_WIDTH_PX * EMU_PER_PX).round() as u32;
    (int as u32, off)
}

/// Split a fractional row coordinate into integer + EMU offset.
/// E.g. `2.2` → `(2, 38100)` (0.2 * 20 * 9525).
fn split_row_frac(v: f64) -> (u32, u32) {
    let int = v.floor();
    let frac = v - int;
    let off = (frac * DEFAULT_ROW_HEIGHT_PX * EMU_PER_PX).round() as u32;
    (int as u32, off)
}

/// Reconstruct a float from integer + EMU offset (inverse of split_frac).
fn reconstruct_float(int: u32, off: u32, px_per_unit: f64) -> f64 {
    let v = int as f64 + off as f64 / (px_per_unit * EMU_PER_PX);
    round_to_4dp(v)
}

fn round_to_4dp(v: f64) -> f64 {
    (v * 10000.0).round() / 10000.0
}
