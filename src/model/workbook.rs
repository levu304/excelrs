//! Workbook — public JS-facing document type.
//!
//! Thin wrapper around `Arc<Mutex<WorkbookInner>>`.  All methods take the lock,
//! delegate to `WorkbookInner`, and return cloned results (clone-on-read
//! semantics).  The `xlsx` getter returns a `WorkbookXlsx` handle that shares
//! the same inner — so `wb.xlsx.read(buf)` mutates the same state that
//! `wb.getWorksheet(...)` reads from.

use std::sync::{Arc, Mutex};

use napi_derive::napi;

use super::defined_name::DefinedName;
use super::header_footer::HeaderFooter;
use super::page_setup::PageSetup;
use super::sheet_protection::SheetProtection;
use super::sheet_view::SheetView;
use super::workbook_inner::WorkbookInner;
use super::workbook_view::{CalcProperties, WorkbookView};
use super::worksheet::Worksheet;
use crate::csv::WorkbookCsv;
use crate::stream_handle::WorkbookStream;
use crate::xlsx::WorkbookXlsx;

/// Options for creating a new worksheet via `Workbook.addWorksheet(name, options?)`.
///
/// Mirrors ExcelJS `AddWorksheetOptions`; each field maps to the corresponding
/// worksheet setter.
#[napi(object)]
#[derive(Clone, Debug, Default)]
pub struct AddWorksheetOptions {
    pub page_setup: Option<PageSetup>,
    pub views: Option<Vec<SheetView>>,
    pub header_footer: Option<HeaderFooter>,
    pub protection: Option<SheetProtection>,
    pub auto_filter: Option<String>,
}

/// Top-level workbook document.
///
/// Wraps `WorkbookInner` behind `Arc<Mutex<>>` so that the `WorkbookXlsx`
/// handle can mutate the workbook state via a shared reference.
///
/// # Clone-on-read semantics
/// Like all napi-rs model types, accessed worksheets are cloned across the FFI
/// boundary.  Cloning the `Workbook` itself clones the `Arc` — all clones share
/// the same inner state.
#[napi]
#[derive(Clone, Debug)]
pub struct Workbook {
    inner: Arc<Mutex<WorkbookInner>>,
}

#[napi]
impl Workbook {
    #[napi(constructor)]
    pub fn new() -> Self {
        Workbook {
            inner: Arc::new(Mutex::new(WorkbookInner::new())),
        }
    }

    /// Add a new worksheet with the given name and optional options.
    /// Returns the created Worksheet.
    #[napi]
    pub fn add_worksheet(&mut self, name: String, options: Option<AddWorksheetOptions>) -> Worksheet {
        let mut inner = self.inner.lock().expect("Workbook lock poisoned");
        let ws = inner.add_worksheet(name);

        // Apply options on the returned clone (interior mutability propagates back)
        if let Some(opts) = options {
            if let Some(ps) = opts.page_setup {
                ws.set_page_setup_inner(Some(ps));
            }
            if let Some(views) = opts.views {
                ws.set_views_inner(views);
            }
            if let Some(hf) = opts.header_footer {
                ws.set_header_footer_inner(Some(hf));
            }
            if let Some(p) = opts.protection {
                ws.set_protection_inner(Some(p));
            }
            if let Some(af) = opts.auto_filter {
                ws.set_auto_filter_range(Some(af));
            }
        }

        ws
    }

    /// Get a worksheet by name (string) or 1-indexed position (number).
    /// Returns `None` if not found.
    #[napi]
    pub fn get_worksheet(&self, name_or_index: serde_json::Value) -> Option<Worksheet> {
        self.inner
            .lock()
            .expect("Workbook lock poisoned")
            .get_worksheet(name_or_index)
    }

    #[napi(getter)]
    pub fn worksheets(&self) -> Vec<Worksheet> {
        self.inner.lock().expect("Workbook lock poisoned").worksheets()
    }

    /// Recalculate every worksheet, caching computed values with full workbook
    /// context so cross-sheet references (e.g. `Sheet2!A1`) resolve to live
    /// values. Available only when built with the `formula-eval` feature.
    #[napi]
    /// Recalculate every worksheet, caching computed values with full workbook
    /// context so cross-sheet references (e.g. `Sheet2!A1`) resolve to live
    /// values. No-op when built without `formula-eval`.
    pub fn recalculate(&self) {
        #[cfg(feature = "formula-eval")]
        {
            let inner = self.inner.lock().expect("Workbook lock poisoned");
            // Snapshot worksheets (clone-on-read, share row Arcs) once, then eval
            // each with `&inner` workbook context. The guard and each `&ws` both
            // outlive the loop; the evaluator field-reads `inner.worksheets` (no
            // re-lock), so the held Mutex never deadlocks.
            let worksheets = inner.worksheets();
            for ws in &worksheets {
                // recalc_with is infallible per-cell; discard the always-Ok
                // Result explicitly to satisfy `must_use`.
                let _ = ws.recalculate_with(Some(&*inner));
            }
        }
    }

    #[napi(getter)]
    pub fn worksheet_count(&self) -> u32 {
        self.inner.lock().expect("Workbook lock poisoned").worksheet_count()
    }

    /// ISO-8601 timestamp of workbook creation.
    #[napi(getter)]
    pub fn created(&self) -> String {
        self.inner.lock().expect("Workbook lock poisoned").created()
    }

    /// ISO-8601 timestamp of last modification.
    #[napi(getter)]
    pub fn modified(&self) -> String {
        self.inner.lock().expect("Workbook lock poisoned").modified()
    }

    /// Returns a `WorkbookXlsx` handle for async XLSX I/O.
    ///
    /// The handle shares the same underlying `Arc<Mutex<WorkbookInner>>`,
    /// so reads through `.xlsx.read(buf)` mutate this workbook's state.
    #[napi(getter)]
    pub fn xlsx(&self) -> WorkbookXlsx {
        WorkbookXlsx::new(Arc::clone(&self.inner))
    }

    /// Returns a `WorkbookStream` handle for streaming XLSX I/O
    /// (ExcelJS `workbook.stream`).
    ///
    /// The handle shares the same underlying `Arc<Mutex<WorkbookInner>>`.
    /// Streaming read/write yield/accept sheet/row/cell structures without
    /// materializing the full in-memory model.
    #[napi(getter)]
    pub fn stream(&self) -> WorkbookStream {
        WorkbookStream::new(Arc::clone(&self.inner))
    }

    /// Returns a `WorkbookCsv` handle for async CSV I/O.
    ///
    /// The handle shares the same underlying `Arc<Mutex<WorkbookInner>>`
    /// as the parent Workbook.
    #[napi(getter)]
    pub fn csv(&self) -> WorkbookCsv {
        WorkbookCsv::new(Arc::clone(&self.inner))
    }

    // -- Defined names (v0.7.0) --

    /// Snapshot of all defined names in the workbook.
    #[napi(getter)]
    pub fn defined_names(&self) -> Vec<DefinedName> {
        self.inner
            .lock()
            .expect("Workbook lock poisoned")
            .defined_names()
            .to_vec()
    }

    /// Add or upsert a defined name.
    ///
    /// Workbook-scope: matched by `name` alone.
    /// Sheet-scope: matched by `name` + `sheet`.
    #[napi]
    pub fn add_defined_name(&mut self, name: String, value: String, sheet: Option<String>) {
        self.inner
            .lock()
            .expect("Workbook lock poisoned")
            .add_defined_name(name, value, sheet);
    }

    /// Remove a defined name by `name` (and optional `sheet`).
    /// No-op if no matching name exists.
    #[napi]
    pub fn remove_defined_name(&mut self, name: String, sheet: Option<String>) {
        self.inner
            .lock()
            .expect("Workbook lock poisoned")
            .remove_defined_name(&name, sheet.as_deref());
    }

    /// Get a defined name by `name` (and optional `sheet`).
    /// Returns `None` if not found.
    #[napi]
    pub fn get_defined_name(&self, name: String, sheet: Option<String>) -> Option<DefinedName> {
        self.inner
            .lock()
            .expect("Workbook lock poisoned")
            .get_defined_name(&name, sheet.as_deref())
            .cloned()
    }

    // -- Views & calc properties (v1.0.0) --

    #[napi(getter)]
    pub fn views(&self) -> Vec<WorkbookView> {
        self.inner.lock().expect("Workbook lock poisoned").views()
    }

    #[napi(setter)]
    pub fn set_views(&mut self, views: Vec<WorkbookView>) {
        self.inner.lock().expect("Workbook lock poisoned").set_views(views)
    }

    #[napi(getter)]
    pub fn calc_properties(&self) -> Option<CalcProperties> {
        self.inner.lock().expect("Workbook lock poisoned").calc_properties()
    }

    #[napi(setter)]
    pub fn set_calc_properties(&mut self, calc: Option<CalcProperties>) {
        self.inner
            .lock()
            .expect("Workbook lock poisoned")
            .set_calc_properties(calc)
    }
}

// Internal methods (not exposed via napi)
impl Workbook {
    /// Wrap an already-constructed `WorkbookInner`.
    pub fn from_inner(inner: WorkbookInner) -> Self {
        Workbook {
            inner: Arc::new(Mutex::new(inner)),
        }
    }
}

impl Default for Workbook {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workbook_new() {
        let wb = Workbook::new();
        assert_eq!(wb.worksheet_count(), 0);
        assert!(wb.worksheets().is_empty());
    }

    #[test]
    fn test_add_worksheet() {
        let mut wb = Workbook::new();
        let ws = wb.add_worksheet("Sheet1".into(), None);
        assert_eq!(ws.name(), "Sheet1");
        assert_eq!(ws.id(), 1);
        assert_eq!(wb.worksheet_count(), 1);
    }

    #[test]
    fn test_get_worksheet_by_name() {
        let mut wb = Workbook::new();
        wb.add_worksheet("Sheet1".into(), None);
        wb.add_worksheet("Data".into(), None);

        let ws = wb.get_worksheet(serde_json::json!("Data"));
        assert!(ws.is_some());
        assert_eq!(ws.unwrap().name(), "Data");

        let missing = wb.get_worksheet(serde_json::json!("NonExistent"));
        assert!(missing.is_none());
    }

    #[test]
    fn test_get_worksheet_by_index() {
        let mut wb = Workbook::new();
        wb.add_worksheet("First".into(), None);
        wb.add_worksheet("Second".into(), None);

        let ws = wb.get_worksheet(serde_json::json!(2));
        assert!(ws.is_some());
        assert_eq!(ws.unwrap().name(), "Second");

        let out_of_range = wb.get_worksheet(serde_json::json!(99));
        assert!(out_of_range.is_none());
    }

    #[test]
    fn test_multiple_worksheets() {
        let mut wb = Workbook::new();
        wb.add_worksheet("A".into(), None);
        wb.add_worksheet("B".into(), None);
        wb.add_worksheet("C".into(), None);

        assert_eq!(wb.worksheet_count(), 3);
        let all = wb.worksheets();
        assert_eq!(all[0].name(), "A");
        assert_eq!(all[1].name(), "B");
        assert_eq!(all[2].name(), "C");
    }

    #[test]
    fn test_workbook_xlsx_getter_returns_handle() {
        let wb = Workbook::new();
        let _handle = wb.xlsx();
        // xlsx() returns a handle wrapping the same Arc
        // (can't easily test identity in Rust, but we can verify
        //  that mutations through the handle affect the Workbook)
    }

    #[test]
    fn test_workbook_from_inner() {
        let mut inner = WorkbookInner::new();
        inner.add_worksheet("FromInner".into());
        let wb = Workbook::from_inner(inner);
        assert_eq!(wb.worksheet_count(), 1);
        assert_eq!(wb.worksheets()[0].name(), "FromInner");
    }

    // -- defined names --

    #[test]
    fn test_napi_defined_names_default_empty() {
        let wb = Workbook::new();
        assert!(wb.defined_names().is_empty());
    }

    #[test]
    fn test_napi_add_defined_name_global() {
        let mut wb = Workbook::new();
        wb.add_defined_name("Rate".into(), "0.08".into(), None);
        let names = wb.defined_names();
        assert_eq!(names.len(), 1);
        assert_eq!(names[0].name, "Rate");
        assert_eq!(names[0].value, "0.08");
        assert!(names[0].sheet.is_none());
    }

    #[test]
    fn test_napi_add_defined_name_sheet() {
        let mut wb = Workbook::new();
        wb.add_defined_name("Local".into(), "$A$1".into(), Some("Sheet1".into()));
        let names = wb.defined_names();
        assert_eq!(names.len(), 1);
        assert_eq!(names[0].sheet.as_deref(), Some("Sheet1"));
    }

    #[test]
    fn test_napi_remove_defined_name() {
        let mut wb = Workbook::new();
        wb.add_defined_name("X".into(), "1".into(), None);
        wb.remove_defined_name("X".into(), None);
        assert!(wb.defined_names().is_empty());
    }

    #[test]
    fn test_napi_get_defined_name() {
        let mut wb = Workbook::new();
        wb.add_defined_name("Rate".into(), "0.08".into(), None);
        let dn = wb.get_defined_name("Rate".into(), None);
        assert!(dn.is_some());
        assert_eq!(dn.unwrap().value, "0.08");
    }

    #[test]
    fn test_napi_get_defined_name_missing() {
        let wb = Workbook::new();
        assert!(wb.get_defined_name("Missing".into(), None).is_none());
    }

    #[test]
    fn test_workbook_clone_shares_inner() {
        let mut wb = Workbook::new();
        wb.add_worksheet("Original".into(), None);
        let cloned = wb.clone();
        // Both share the same inner — the clone sees the same state
        assert_eq!(cloned.worksheet_count(), 1);
        assert_eq!(cloned.worksheets()[0].name(), "Original");
    }

    #[test]
    fn test_add_worksheet_with_options_applies_page_setup() {
        use crate::model::page_setup::PageSetup;
        let mut wb = Workbook::new();
        let ws = wb.add_worksheet(
            "Sheet1".into(),
            Some(AddWorksheetOptions {
                page_setup: Some(PageSetup {
                    orientation: Some(crate::model::page_setup::Orientation::Landscape),
                    paper_size: Some(9),
                    ..Default::default()
                }),
                views: None,
                header_footer: None,
                protection: None,
                auto_filter: None,
            }),
        );
        let ps = ws.page_setup().expect("pageSetup should be set");
        assert_eq!(ps.orientation, Some(crate::model::page_setup::Orientation::Landscape));
        assert_eq!(ps.paper_size, Some(9));
    }

    #[test]
    fn test_add_worksheet_with_options_applies_all_fields() {
        use crate::model::page_setup::PageSetup;
        use crate::model::sheet_view::SheetView;
        let mut wb = Workbook::new();
        let ws = wb.add_worksheet(
            "Sheet1".into(),
            Some(AddWorksheetOptions {
                page_setup: Some(PageSetup {
                    orientation: Some(crate::model::page_setup::Orientation::Portrait),
                    ..Default::default()
                }),
                views: Some(vec![SheetView {
                    show_grid_lines: Some(false),
                    ..Default::default()
                }]),
                auto_filter: Some("A1:C10".into()),
                protection: None,
                header_footer: None,
            }),
        );
        assert!(ws.page_setup().is_some());
        assert_eq!(ws.views().len(), 1);
        assert_eq!(ws.views()[0].show_grid_lines, Some(false));
        assert_eq!(ws.auto_filter().as_deref(), Some("A1:C10"));
    }

    #[test]
    fn test_add_worksheet_without_options_unchanged() {
        let mut wb = Workbook::new();
        let ws = wb.add_worksheet("Sheet1".into(), None);
        assert_eq!(ws.name(), "Sheet1");
        assert!(ws.page_setup().is_none());
        assert!(ws.views().is_empty());
        assert!(ws.auto_filter().is_none());
        assert_eq!(wb.worksheet_count(), 1);
    }
}
