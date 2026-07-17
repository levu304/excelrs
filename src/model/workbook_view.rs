//! Workbook view (`CT_WorkbookView`) and calculation properties (`CT_CalcPr`).
//!
//! Mirrors ExcelJS `wb.views` (array of view descriptors) and
//! `wb.calcProperties`.

use napi_derive::napi;

/// A single workbook view descriptor (`CT_WorkbookView`).
#[napi(object)]
#[derive(Clone, Debug, Default)]
pub struct WorkbookView {
    pub x_window: Option<u32>,
    pub y_window: Option<u32>,
    pub window_width: Option<u32>,
    pub window_height: Option<u32>,
    pub active_tab: Option<u32>,
    pub first_sheet: Option<u32>,
    pub minimized: Option<bool>,
    pub show_horizontal_scroll: Option<bool>,
    pub show_vertical_scroll: Option<bool>,
    pub tab_ratio: Option<u32>,
    pub visibility: Option<String>,
}

/// Workbook calculation properties (`CT_CalcPr`).
#[napi(object)]
#[derive(Clone, Debug, Default)]
pub struct CalcProperties {
    pub full_calc_on_load: Option<bool>,
    pub calc_id: Option<u32>,
    pub calc_mode: Option<String>,
    pub ref_full_calc: Option<bool>,
    pub iterate: Option<bool>,
    pub iterate_count: Option<u32>,
    pub iterate_delta: Option<f64>,
}
