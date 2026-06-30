//! Error types for excelrs.
//!
//! All errors are `thiserror`-derived and annotated with `#[napi]` for automatic
//! JS `Error` subclass mapping. The `ExcelrsError` enum covers every failure mode
//! in the model layer. Reader/writer variants (Parse, Write, Zip, Xml) are defined
//! here for forward compatibility.

/// Typed error enum for all excelrs operations.
///
/// Mapped to JS `Error` subclasses via napi-rs. Each variant includes a
/// descriptive message via `thiserror::Display`.
#[derive(Debug, thiserror::Error)]
pub enum ExcelrsError {
    /// I/O error (file read/write).
    #[error("Failed to read file: {0}")]
    Io(#[from] std::io::Error),

    /// Invalid or corrupt XLSX format.
    #[error("Invalid XLSX format: {0}")]
    Parse(String),

    /// Requested sheet does not exist.
    #[error("Sheet '{0}' not found")]
    SheetNotFound(String),

    /// Malformed or out-of-range cell address.
    #[error("Invalid cell address: {0}")]
    InvalidAddress(String),

    /// Invalid style value or combination (spec §6.8 validation rules).
    #[error("{0}")]
    InvalidStyle(String),

    /// Error during XLSX write.
    #[error("Write error: {0}")]
    Write(String),

    /// ZIP (de)compression error.
    #[error("ZIP error: {0}")]
    Zip(String),

    /// XML parse/serialize error.
    #[error("XML error: {0}")]
    Xml(String),
}

/// napi-rs requires `#[napi]` on error types for JS Error mapping.
/// The flat string variant maps to a JS `Error` with the Display message.
impl From<ExcelrsError> for napi::Error {
    fn from(err: ExcelrsError) -> Self {
        napi::Error::from_reason(err.to_string())
    }
}
