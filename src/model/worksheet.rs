//! Worksheet: a single sheet in a workbook, containing rows and columns.
//!
//! Rows use `Arc<Mutex<BTreeMap<u32, Row>>>` for **interior mutability**:
//! when a `Worksheet` is cloned (which happens every time napi-rs returns
//! one to JS), both the original and the clone share the same underlying
//! row map.  Mutations through *any* clone propagate to the one that lives
//! inside the `WorkbookInner` — so `ws.addRow([42])` works even when `ws`
//! was obtained from `wb.addWorksheet("Sheet1")`.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use napi_derive::napi;

use super::cell::{Cell, CellValue};
use super::column::Column;
use super::row::Row;
use crate::types;

/// A single worksheet (sheet) in a workbook.
///
/// Rows are stored behind `Arc<Mutex<>>` so that any clone of a Worksheet
/// shares the same row state.  This is what makes `wb.addWorksheet() → ws`
/// → `ws.addRow(...)` work across the napi-rs FFI boundary.
#[napi]
#[derive(Clone, Debug)]
pub struct Worksheet {
    name: String,
    id: u32,
    rows: Arc<Mutex<BTreeMap<u32, Row>>>,
    columns: Vec<Column>,
}

#[napi]
impl Worksheet {
    #[napi(constructor)]
    pub fn new(name: String) -> Self {
        Worksheet {
            name,
            id: 1,
            rows: Arc::new(Mutex::new(BTreeMap::new())),
            columns: Vec::new(),
        }
    }

    // -- name --

    #[napi(getter)]
    pub fn name(&self) -> String {
        self.name.clone()
    }

    #[napi(setter)]
    pub fn set_name(&mut self, val: String) {
        self.name = val;
    }

    // -- id --

    #[napi(getter)]
    pub fn id(&self) -> u32 {
        self.id
    }

    // -- row_count --

    /// Number of rows with content (highest row index with data).
    #[napi(getter)]
    pub fn row_count(&self) -> u32 {
        self.rows
            .lock()
            .expect("Worksheet rows lock poisoned")
            .last_key_value()
            .map(|(k, _)| *k)
            .unwrap_or(0)
    }

    // -- column_count --

    /// Number of columns with content (highest column index across all rows).
    #[napi(getter)]
    pub fn column_count(&self) -> u32 {
        self.rows
            .lock()
            .expect("Worksheet rows lock poisoned")
            .values()
            .map(|r| r.max_col())
            .max()
            .unwrap_or(0)
    }

    // -- get_cell_by_address --

    /// Get cell by A1-style address string (e.g., "A1", "BC42").
    /// Returns an empty cell if the address is valid but hasn't been populated.
    #[napi]
    pub fn get_cell_by_address(&self, address: String) -> Cell {
        match types::parse_address(&address) {
            Ok((col, row)) => self.get_cell_by_rc(row, col),
            Err(_) => Cell::new(address, 0, 0),
        }
    }

    // -- get_cell_by_rc --

    /// Get cell by 1-indexed row and column numbers.
    /// Returns an empty cell if the coordinates are valid but haven't been populated.
    #[napi]
    pub fn get_cell_by_rc(&self, row: u32, col: u32) -> Cell {
        self.rows
            .lock()
            .expect("Worksheet rows lock poisoned")
            .get(&row)
            .map(|r| r.get_cell_by_col_num(col))
            .unwrap_or_else(|| {
                Cell::new(
                    types::address_to_string(col, row).unwrap_or_else(|_| format!("R{row}C{col}")),
                    row,
                    col,
                )
            })
    }

    // -- get_row --

    /// Get row by 1-indexed row number. Creates the row if it doesn't exist.
    #[napi]
    pub fn get_row(&self, row_number: u32) -> Row {
        let mut rows = self.rows.lock().expect("Worksheet rows lock poisoned");
        rows.entry(row_number).or_insert_with(|| Row::new(row_number)).clone()
    }

    // -- add_row --

    /// Add a row of cell values. Returns the created Row.
    #[napi]
    pub fn add_row(&self, values: Vec<serde_json::Value>) -> Row {
        let mut rows = self.rows.lock().expect("Worksheet rows lock poisoned");
        let next_row_num = rows.last_key_value().map(|(k, _)| *k + 1).unwrap_or(1);
        let mut row = Row::new(next_row_num);

        for (i, val) in values.iter().enumerate() {
            let col = (i + 1) as u32;

            let cv = match val {
                serde_json::Value::Number(n) => CellValue {
                    value_type: "Number".into(),
                    number: n.as_f64(),
                    ..Default::default()
                },
                serde_json::Value::String(s) => CellValue {
                    value_type: "String".into(),
                    string: Some(s.clone()),
                    ..Default::default()
                },
                serde_json::Value::Bool(b) => CellValue {
                    value_type: "Boolean".into(),
                    boolean: Some(*b),
                    ..Default::default()
                },
                _ => CellValue::default(),
            };
            row.set_cell_value(col, cv);
        }

        let row_num = row.number();
        let clone = row.clone();
        rows.insert(row_num, row);
        clone
    }

    // -- get_rows --

    /// Get a contiguous range of rows starting at `start` (1-indexed).
    /// Returns up to `count` rows.
    #[napi]
    pub fn get_rows(&self, start: u32, count: u32) -> Vec<Row> {
        let end = start.saturating_add(count);
        self.rows
            .lock()
            .expect("Worksheet rows lock poisoned")
            .range(start..end)
            .map(|(_, row)| row.clone())
            .collect()
    }

    // -- remove_row --

    /// Remove a row by number. No-op if the row doesn't exist.
    #[napi]
    pub fn remove_row(&self, row_number: u32) {
        self.rows
            .lock()
            .expect("Worksheet rows lock poisoned")
            .remove(&row_number);
    }

    // -- rows (iterable) --

    /// All rows with content, sorted by row number.
    #[napi(getter)]
    pub fn rows(&self) -> Vec<Row> {
        self.rows
            .lock()
            .expect("Worksheet rows lock poisoned")
            .values()
            .cloned()
            .collect()
    }

    // -- columns --

    #[napi(getter)]
    pub fn columns(&self) -> Vec<Column> {
        self.columns.clone()
    }
}

// Internal methods (not exposed via napi)
impl Worksheet {
    /// Set the worksheet id (used by Workbook/reader).
    pub fn set_id(&mut self, id: u32) {
        self.id = id;
    }

    /// Insert a cell value at (row, col) — used by the reader.
    pub fn insert_cell_value(&self, row: u32, col: u32, value: CellValue) {
        let mut rows = self.rows.lock().expect("Worksheet rows lock poisoned");
        let ws_row = rows.entry(row).or_insert_with(|| Row::new(row));
        ws_row.set_cell_value(col, value);
    }

    /// Replace the worksheet's column definitions.
    pub fn set_columns(&mut self, cols: Vec<Column>) {
        self.columns = cols;
    }

    /// Set a formula string on a cell at (row, col) — used by the reader.
    pub fn insert_cell_formula(&self, row: u32, col: u32, formula: String) {
        let mut rows = self.rows.lock().expect("Worksheet rows lock poisoned");
        let ws_row = rows.entry(row).or_insert_with(|| Row::new(row));
        let cell = ws_row.get_or_create_cell_mut(col);
        cell.set_formula(Some(formula));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worksheet_new() {
        let ws = Worksheet::new("Sheet1".into());
        assert_eq!(ws.name(), "Sheet1");
        assert_eq!(ws.id(), 1);
        assert_eq!(ws.row_count(), 0);
        assert_eq!(ws.column_count(), 0);
    }

    #[test]
    fn test_get_cell_fresh_worksheet() {
        let ws = Worksheet::new("Sheet1".into());
        let cell = ws.get_cell_by_address("A1".into());
        assert_eq!(cell.address(), "A1");
        assert_eq!(cell.row(), 1);
        assert_eq!(cell.col(), 1);
        assert_eq!(cell.value().value_type, "Null");
    }

    #[test]
    fn test_get_cell_by_rc() {
        let ws = Worksheet::new("Sheet1".into());
        let cell = ws.get_cell_by_rc(3, 5);
        assert_eq!(cell.address(), "E3");
        assert_eq!(cell.row(), 3);
        assert_eq!(cell.col(), 5);
    }

    #[test]
    fn test_add_row_values() {
        let ws = Worksheet::new("Data".into());
        let row = ws.add_row(vec![
            serde_json::json!("Alice"),
            serde_json::json!(30),
            serde_json::json!(true),
        ]);
        assert_eq!(row.number(), 1);
        assert_eq!(ws.row_count(), 1);

        // Verify cell values via get_cell_by_address
        let c1 = ws.get_cell_by_address("A1".into());
        assert_eq!(c1.value().string, Some("Alice".into()));

        let c2 = ws.get_cell_by_address("B1".into());
        assert_eq!(c2.value().number, Some(30.0));

        let c3 = ws.get_cell_by_address("C1".into());
        assert_eq!(c3.value().boolean, Some(true));
    }

    #[test]
    fn test_clone_shares_rows() {
        let ws = Worksheet::new("Shared".into());
        let cloned = ws.clone();

        // addRow on the clone should be visible through the original
        cloned.add_row(vec![serde_json::json!(42)]);
        assert_eq!(ws.row_count(), 1);
        assert_eq!(ws.get_cell_by_address("A1".into()).value().number, Some(42.0));
    }

    #[test]
    fn test_remove_row() {
        let ws = Worksheet::new("Sheet1".into());
        ws.add_row(vec![serde_json::json!(1)]);
        ws.add_row(vec![serde_json::json!(2)]);
        assert_eq!(ws.row_count(), 2);

        ws.remove_row(1);
        assert_eq!(ws.row_count(), 2); // row_count is max row, not count of rows
        assert!(ws.get_cell_by_address("A1".into()).value().value_type == "Null");
        // removed
    }

    #[test]
    fn test_get_rows() {
        let ws = Worksheet::new("Sheet1".into());
        ws.add_row(vec![serde_json::json!(1)]);
        ws.add_row(vec![serde_json::json!(2)]);
        ws.add_row(vec![serde_json::json!(3)]);

        let rows = ws.get_rows(2, 2);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].number(), 2);
        assert_eq!(rows[1].number(), 3);
    }

    #[test]
    fn test_rows_getter_sorted() {
        let ws = Worksheet::new("Sheet1".into());
        ws.add_row(vec![serde_json::json!(3)]);
        ws.add_row(vec![serde_json::json!(1)]);
        ws.add_row(vec![serde_json::json!(2)]);

        let all = ws.rows();
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].number(), 1);
        assert_eq!(all[1].number(), 2);
        assert_eq!(all[2].number(), 3);
    }

    #[test]
    fn test_set_name() {
        let mut ws = Worksheet::new("Old".into());
        ws.set_name("New".into());
        assert_eq!(ws.name(), "New");
    }

    #[test]
    fn test_column_count() {
        let ws = Worksheet::new("Sheet1".into());
        assert_eq!(ws.column_count(), 0);

        ws.add_row(vec![serde_json::json!(1), serde_json::json!(2), serde_json::json!(3)]);
        assert_eq!(ws.column_count(), 3);
    }
}
