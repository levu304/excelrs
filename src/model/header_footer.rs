//! Worksheet header/footer (`CT_HeaderFooter`).
//!
//! Excel header/footer strings carry `&`-prefixed format codes
//! (`&L`/`&C`/`&R` alignment, `&[Page]`/`&[Date]`/`&P`/`&N` fields). These are
//! stored verbatim — excelrs does not parse or validate them, matching
//! ExcelJS behavior.

use napi_derive::napi;

/// Header/footer descriptor for a worksheet (mirrors ExcelJS `ws.headerFooter`).
#[napi(object)]
#[derive(Clone, Debug, Default)]
pub struct HeaderFooter {
    /// Left section of the odd-page header.
    pub odd_header: Option<String>,
    /// Right section of the odd-page footer.
    pub odd_footer: Option<String>,
    /// Left section of the even-page header.
    pub even_header: Option<String>,
    /// Right section of the even-page footer.
    pub even_footer: Option<String>,
    /// Left section of the first-page header.
    pub first_header: Option<String>,
    /// Right section of the first-page footer.
    pub first_footer: Option<String>,
    /// Align header/footer margins with page margins.
    pub align_with_margins: Option<bool>,
    /// Use a different first-page header/footer.
    pub different_first: Option<bool>,
    /// Use different odd- and even-page headers/footers.
    pub different_odd_even: Option<bool>,
}
