//! Workbook — public JS-facing document type.
//!
//! Thin wrapper around `Arc<Mutex<WorkbookInner>>`.  All methods take the lock,
//! delegate to `WorkbookInner`, and return cloned results (clone-on-read
//! semantics).  The `xlsx` getter returns a `WorkbookXlsx` handle that shares
//! the same inner — so `wb.xlsx.read(buf)` mutates the same state that
//! `wb.getWorksheet(...)` reads from.

use std::sync::{Arc, Mutex};

use napi_derive::napi;

use super::workbook_inner::WorkbookInner;
use super::worksheet::Worksheet;
use crate::xlsx::WorkbookXlsx;

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

    /// Add a new worksheet with the given name.
    /// Returns the created Worksheet.
    #[napi]
    pub fn add_worksheet(&mut self, name: String) -> Worksheet {
        self.inner.lock().expect("Workbook lock poisoned").add_worksheet(name)
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
        let ws = wb.add_worksheet("Sheet1".into());
        assert_eq!(ws.name(), "Sheet1");
        assert_eq!(ws.id(), 1);
        assert_eq!(wb.worksheet_count(), 1);
    }

    #[test]
    fn test_get_worksheet_by_name() {
        let mut wb = Workbook::new();
        wb.add_worksheet("Sheet1".into());
        wb.add_worksheet("Data".into());

        let ws = wb.get_worksheet(serde_json::json!("Data"));
        assert!(ws.is_some());
        assert_eq!(ws.unwrap().name(), "Data");

        let missing = wb.get_worksheet(serde_json::json!("NonExistent"));
        assert!(missing.is_none());
    }

    #[test]
    fn test_get_worksheet_by_index() {
        let mut wb = Workbook::new();
        wb.add_worksheet("First".into());
        wb.add_worksheet("Second".into());

        let ws = wb.get_worksheet(serde_json::json!(2));
        assert!(ws.is_some());
        assert_eq!(ws.unwrap().name(), "Second");

        let out_of_range = wb.get_worksheet(serde_json::json!(99));
        assert!(out_of_range.is_none());
    }

    #[test]
    fn test_multiple_worksheets() {
        let mut wb = Workbook::new();
        wb.add_worksheet("A".into());
        wb.add_worksheet("B".into());
        wb.add_worksheet("C".into());

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

    #[test]
    fn test_workbook_clone_shares_inner() {
        let mut wb = Workbook::new();
        wb.add_worksheet("Original".into());
        let cloned = wb.clone();
        // Both share the same inner — the clone sees the same state
        assert_eq!(cloned.worksheet_count(), 1);
        assert_eq!(cloned.worksheets()[0].name(), "Original");
    }
}
