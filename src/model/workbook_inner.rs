//! WorkbookInner — the actual document state behind the Arc<Mutex> wall.
//!
//! This type holds all workbook data but is *not* exposed to napi-rs directly.
//! The public `Workbook` struct wraps it in `Arc<Mutex<WorkbookInner>>` so that
//! the `WorkbookXlsx` handle can mutate the same underlying state.
//!
//! All methods here are identical to the pre-refactor `Workbook` API; they just
//! operate on a struct that can be shared across an Arc.

use chrono::{DateTime, Utc};

use super::worksheet::Worksheet;

/// Actual workbook state. Not exported via napi — always accessed through
/// the `Workbook` wrapper or a `WorkbookXlsx` handle.
#[derive(Debug, Clone)]
pub struct WorkbookInner {
    pub worksheets: Vec<Worksheet>,
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
}

impl WorkbookInner {
    pub fn new() -> Self {
        let now = Utc::now();
        WorkbookInner {
            worksheets: Vec::new(),
            created: now,
            modified: now,
        }
    }

    /// Add a new worksheet with the given name. Returns the created Worksheet.
    pub fn add_worksheet(&mut self, name: String) -> Worksheet {
        let id = (self.worksheets.len() + 1) as u32;
        let mut ws = Worksheet::new(name);
        ws.set_id(id);
        self.modified = Utc::now();
        let clone = ws.clone();
        self.worksheets.push(ws);
        clone
    }

    /// Get a worksheet by name (string) or 1-indexed position (number).
    /// Returns `None` if not found.
    pub fn get_worksheet(&self, name_or_index: serde_json::Value) -> Option<Worksheet> {
        match name_or_index {
            serde_json::Value::String(name) => self.worksheets.iter().find(|ws| ws.name() == name).cloned(),
            serde_json::Value::Number(n) => {
                let idx = n.as_f64().map(|f| f as usize)?;
                if idx >= 1 && idx <= self.worksheets.len() {
                    Some(self.worksheets[idx - 1].clone())
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn worksheets(&self) -> Vec<Worksheet> {
        self.worksheets.clone()
    }

    pub fn worksheet_count(&self) -> u32 {
        self.worksheets.len() as u32
    }

    /// ISO-8601 timestamp of workbook creation.
    pub fn created(&self) -> String {
        self.created.to_rfc3339()
    }

    /// ISO-8601 timestamp of last modification.
    pub fn modified(&self) -> String {
        self.modified.to_rfc3339()
    }

    pub fn set_worksheets(&mut self, worksheets: Vec<Worksheet>) {
        self.worksheets = worksheets;
    }
}

impl Default for WorkbookInner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workbook_inner_new() {
        let wb = WorkbookInner::new();
        assert_eq!(wb.worksheet_count(), 0);
        assert!(wb.worksheets().is_empty());
    }

    #[test]
    fn test_workbook_inner_add_worksheet() {
        let mut wb = WorkbookInner::new();
        let ws = wb.add_worksheet("Sheet1".into());
        assert_eq!(ws.name(), "Sheet1");
        assert_eq!(ws.id(), 1);
        assert_eq!(wb.worksheet_count(), 1);
    }

    #[test]
    fn test_workbook_inner_get_worksheet_by_name() {
        let mut wb = WorkbookInner::new();
        wb.add_worksheet("Sheet1".into());
        wb.add_worksheet("Data".into());

        let ws = wb.get_worksheet(serde_json::json!("Data"));
        assert!(ws.is_some());
        assert_eq!(ws.unwrap().name(), "Data");

        let missing = wb.get_worksheet(serde_json::json!("NonExistent"));
        assert!(missing.is_none());
    }

    #[test]
    fn test_workbook_inner_get_worksheet_by_index() {
        let mut wb = WorkbookInner::new();
        wb.add_worksheet("First".into());
        wb.add_worksheet("Second".into());

        let ws = wb.get_worksheet(serde_json::json!(2));
        assert!(ws.is_some());
        assert_eq!(ws.unwrap().name(), "Second");

        let out_of_range = wb.get_worksheet(serde_json::json!(99));
        assert!(out_of_range.is_none());
    }

    #[test]
    fn test_workbook_inner_multiple_worksheets() {
        let mut wb = WorkbookInner::new();
        wb.add_worksheet("A".into());
        wb.add_worksheet("B".into());
        wb.add_worksheet("C".into());

        assert_eq!(wb.worksheet_count(), 3);
        let all = wb.worksheets();
        assert_eq!(all[0].name(), "A");
        assert_eq!(all[1].name(), "B");
        assert_eq!(all[2].name(), "C");
    }
}
