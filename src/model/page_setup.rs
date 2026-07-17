//! Worksheet page setup / print (`CT_PageSetup` + `CT_PageMargins`).
//!
//! Mirrors ExcelJS `ws.pageSetup`. Print area and print titles are stored as
//! A1-range strings and round-trip via the workbook-defined names
//! `_xlnm.Print_Area` / `_xlnm.Print_Titles`.

use napi_derive::napi;

/// Page margins in inches (Excel `CT_PageMargins`).
#[napi(object)]
#[derive(Clone, Debug, Default)]
pub struct PageMargins {
    pub left: Option<f64>,
    pub right: Option<f64>,
    pub top: Option<f64>,
    pub bottom: Option<f64>,
    pub header: Option<f64>,
    pub footer: Option<f64>,
}

/// Page setup / print descriptor for a worksheet (mirrors ExcelJS `ws.pageSetup`).
#[napi(object)]
#[derive(Clone, Debug, Default)]
pub struct PageSetup {
    /// "portrait" or "landscape".
    pub orientation: Option<String>,
    /// Paper size index (e.g. 9 = A4, 1 = Letter).
    pub paper_size: Option<u32>,
    /// Fit the sheet to fitToWidth × fitToHeight pages.
    pub fit_to_page: Option<bool>,
    pub fit_to_width: Option<u32>,
    pub fit_to_height: Option<u32>,
    pub horizontal_dpi: Option<u32>,
    pub vertical_dpi: Option<u32>,
    pub black_and_white: Option<bool>,
    pub drawing_printed: Option<bool>,
    /// "none" | "asDisplayed" | "atEnd".
    pub cell_comments: Option<String>,
    pub copies: Option<u32>,
    /// Page margins in inches.
    pub margins: Option<PageMargins>,
    /// Print area as an A1 range string (e.g. "A1:D10").
    pub print_area: Option<String>,
    /// Print titles as an A1 range string (e.g. "Sheet1!$A:$A").
    pub print_titles: Option<String>,
}
