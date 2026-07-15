//! WorkbookInner — the actual document state behind the Arc<Mutex> wall.
//!
//! This type holds all workbook data but is *not* exposed to napi-rs directly.
//! The public `Workbook` struct wraps it in `Arc<Mutex<WorkbookInner>>` so that
//! the `WorkbookXlsx` handle can mutate the same underlying state.
//!
//! All methods here are identical to the pre-refactor `Workbook` API; they just
//! operate on a struct that can be shared across an Arc.

use chrono::{DateTime, Utc};

use super::color::ThemeColorScheme;

use super::defined_name::DefinedName;
use super::worksheet::Worksheet;

/// Actual workbook state. Not exported via napi — always accessed through
/// the `Workbook` wrapper or a `WorkbookXlsx` handle.
#[derive(Debug, Clone)]
pub struct WorkbookInner {
    pub worksheets: Vec<Worksheet>,
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
    pub defined_names: Vec<DefinedName>,
    /// Theme color scheme (v0.13.0). Captured from xl/theme/theme1.xml on
    /// read; None means use ThemeColorScheme::default() (the writer does
    /// this automatically).  Lets <color theme="N"/> round-trip through an
    /// external reader that resolves theme indices.
    pub theme: Option<ThemeColorScheme>,
}

impl WorkbookInner {
    pub fn new() -> Self {
        let now = Utc::now();
        WorkbookInner {
            worksheets: Vec::new(),
            created: now,
            modified: now,
            defined_names: Vec::new(),
            theme: None,
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

    pub fn defined_names(&self) -> &[DefinedName] {
        &self.defined_names
    }

    pub fn set_defined_names(&mut self, names: Vec<DefinedName>) {
        self.defined_names = names;
        self.modified = Utc::now();
    }

    pub fn add_defined_name(&mut self, name: String, value: String, sheet: Option<String>) {
        if let Some(existing) = self
            .defined_names
            .iter_mut()
            .find(|dn| dn.name == name && dn.sheet == sheet)
        {
            existing.value = value;
        } else {
            self.defined_names.push(DefinedName { name, value, sheet });
        }
        self.modified = Utc::now();
    }

    pub fn remove_defined_name(&mut self, name: &str, sheet: Option<&str>) {
        self.defined_names
            .retain(|dn| !(dn.name == name && dn.sheet.as_deref() == sheet));
        self.modified = Utc::now();
    }

    pub fn get_defined_name(&self, name: &str, sheet: Option<&str>) -> Option<&DefinedName> {
        self.defined_names
            .iter()
            .find(|dn| dn.name == name && dn.sheet.as_deref() == sheet)
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
    fn test_workbook_inner_defined_names_default_empty() {
        let wb = WorkbookInner::new();
        assert!(wb.defined_names().is_empty());
    }

    #[test]
    fn test_workbook_inner_add_defined_name_global() {
        let mut wb = WorkbookInner::new();
        wb.add_defined_name("Rate".into(), "0.08".into(), None);
        assert_eq!(wb.defined_names().len(), 1);
        assert_eq!(wb.defined_names()[0].name, "Rate");
        assert_eq!(wb.defined_names()[0].value, "0.08");
        assert!(wb.defined_names()[0].sheet.is_none());
    }

    #[test]
    fn test_workbook_inner_add_defined_name_sheet() {
        let mut wb = WorkbookInner::new();
        wb.add_defined_name("Local".into(), "$A$1".into(), Some("Sheet1".into()));
        assert_eq!(wb.defined_names().len(), 1);
        assert_eq!(wb.defined_names()[0].sheet.as_deref(), Some("Sheet1"));
    }

    #[test]
    fn test_workbook_inner_add_defined_name_upsert() {
        let mut wb = WorkbookInner::new();
        wb.add_defined_name("X".into(), "1".into(), None);
        wb.add_defined_name("X".into(), "2".into(), None);
        assert_eq!(wb.defined_names().len(), 1);
        assert_eq!(wb.defined_names()[0].value, "2");
    }

    #[test]
    fn test_workbook_inner_remove_defined_name() {
        let mut wb = WorkbookInner::new();
        wb.add_defined_name("X".into(), "1".into(), None);
        wb.remove_defined_name("X", None);
        assert!(wb.defined_names().is_empty());
    }

    #[test]
    fn test_workbook_inner_remove_defined_name_absent_noop() {
        let mut wb = WorkbookInner::new();
        wb.remove_defined_name("NonExistent", None);
        assert!(wb.defined_names().is_empty());
    }

    #[test]
    fn test_workbook_inner_get_defined_name() {
        let mut wb = WorkbookInner::new();
        wb.add_defined_name("Rate".into(), "0.08".into(), None);
        let dn = wb.get_defined_name("Rate", None);
        assert!(dn.is_some());
        assert_eq!(dn.unwrap().value, "0.08");
    }

    #[test]
    fn test_workbook_inner_get_defined_name_missing() {
        let wb = WorkbookInner::new();
        assert!(wb.get_defined_name("Missing", None).is_none());
    }

    #[test]
    fn test_get_defined_name_scoped_and_global() {
        let mut inner = WorkbookInner::new();
        inner.add_defined_name("G".into(), "1".into(), None);
        inner.add_defined_name("S".into(), "2".into(), Some("Sheet1".into()));

        let g = inner.get_defined_name("G", None);
        assert!(g.is_some());
        let g = g.unwrap();
        assert_eq!(g.value, "1");
        assert!(g.sheet.is_none());

        let s = inner.get_defined_name("S", Some("Sheet1"));
        assert!(s.is_some());
        let s = s.unwrap();
        assert_eq!(s.value, "2");
        assert_eq!(s.sheet.as_deref(), Some("Sheet1"));

        // Scoped name not found as global
        let missing = inner.get_defined_name("S", None);
        assert!(missing.is_none());

        // Completely missing
        assert!(inner.get_defined_name("NotFound", None).is_none());
    }

    #[test]
    fn test_workbook_inner_set_defined_names() {
        let mut wb = WorkbookInner::new();
        wb.set_defined_names(vec![DefinedName::global("A", "1"), DefinedName::global("B", "2")]);
        assert_eq!(wb.defined_names().len(), 2);
        assert_eq!(wb.defined_names()[0].name, "A");
        assert_eq!(wb.defined_names()[1].name, "B");
    }

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
