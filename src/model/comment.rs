//! Cell comment / note (v1.0.0).

use napi_derive::napi;

/// A cell comment (note). `text` is the comment body; `author` is optional.
#[napi(object)]
#[derive(Clone, Debug, Default)]
pub struct CellComment {
    pub text: String,
    pub author: Option<String>,
}
