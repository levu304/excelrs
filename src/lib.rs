#![deny(clippy::all)]
#![cfg_attr(docsrs, feature(doc_cfg))]

// ponytail: formula evaluation opt-in behind `formula-eval` feature.
// Uses xlstream-parse (wraps surviving formularizer-parse) for AST parsing.
pub mod csv;
pub mod error;
pub mod model;
pub mod reader;
pub mod stream;
pub mod stream_handle;
pub mod types;
pub mod writer;
pub mod xlsx;

#[cfg(feature = "formula-eval")]
#[cfg_attr(docsrs, doc(cfg(feature = "formula-eval")))]
pub mod formula;