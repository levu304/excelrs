//! Embedded images / drawings (v1.0.0).

use napi_derive::napi;

/// Anchor descriptor for an embedded image.
///
/// `anchor_type` is `"oneCell"` (image pinned to a single cell + offset) or
/// `"twoCell"` (image spans from top-left to bottom-right corners). For
/// `"oneCell"`, use `col`/`row`/`x`/`y`; for `"twoCell"`, `col2`/`row2`/`x2`/`y2`
/// describe the bottom-right corner.
#[napi(object)]
#[derive(Clone, Debug)]
pub struct ImageAnchor {
    pub anchor_type: String,
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
    pub buffer: Vec<u8>,
    pub image_type: Option<String>,
    pub positioning: Option<String>,
    pub anchor: ImageAnchor,
}

#[napi(object)]
#[derive(Clone, Debug)]
pub struct ImageInfo {
    pub extension: String,
    pub buffer: Vec<u8>,
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
