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
        // Returns Array<number>. TS type overridden to Buffer via
        // #[napi(ts_type = "Buffer")] on field — safe lie.
        unsafe { <Vec<u8> as ToNapiValue>::to_napi_value(env, val.0) }
    }
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
    pub anchor: ImageAnchor,
}

#[napi(object)]
#[derive(Clone, Debug)]
pub struct ImageInfo {
    pub extension: String,
    #[napi(ts_type = "Buffer")]
    pub buffer: NapiBuffer,
    pub positioning: String,
    pub anchor: ImageAnchor,
}

/// Internal image record stored on a worksheet (shared by writer & reader).
#[derive(Clone, Debug)]
pub struct WorksheetImage {
    pub extension: String,
    pub buffer: Vec<u8>,
    pub positioning: String,
    pub anchor: ImageAnchor,
    /// Globally-assigned media index (1-based); 0 until assigned by the writer.
    pub media_index: u32,
}
