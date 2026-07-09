//! Worksheet: a single sheet in a workbook, containing rows and columns.
//!
//! Rows and columns both use `Arc<Mutex<>>` for **interior mutability**:
//! when a `Worksheet` is cloned (which happens every time napi-rs returns
//! one to JS), both the original and the clone share the same underlying
//! row map and column vector.  Mutations through *any* clone propagate to
//! the one that lives inside the `WorkbookInner` — so `ws.addRow([42])` and
//! `ws.setColumns([...])` work even when `ws` was obtained from
//! `wb.addWorksheet("Sheet1")`.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use napi_derive::napi;

use super::cell::{Cell, CellValue};
use super::column::Column;
use super::row::Row;
use crate::model::style::Style;
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
    columns: Arc<Mutex<Vec<Column>>>,
}

#[napi]
impl Worksheet {
    #[napi(constructor)]
    pub fn new(name: String) -> Self {
        Worksheet {
            name,
            id: 1,
            rows: Arc::new(Mutex::new(BTreeMap::new())),
            columns: Arc::new(Mutex::new(Vec::new())),
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
    /// Returns the cell from the worksheet's internal row map, so value and style
    /// mutations on the returned cell persist into the worksheet.
    /// Creates the row (and cell) if absent.
    #[napi]
    pub fn get_cell_by_rc(&self, row: u32, col: u32) -> Cell {
        let mut rows = self.rows.lock().expect("Worksheet rows lock poisoned");
        let ws_row = rows.entry(row).or_insert_with(|| Row::new(row));
        let cell = ws_row.get_or_create_cell_mut(col);
        cell.clone()
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
        self.columns.lock().expect("Worksheet columns lock poisoned").clone()
    }

    /// Set the style of a cell at (row, col).  Bypasses clone-on-read:
    /// the cell is mutated inside the locked row map.
    #[napi]
    pub fn set_cell_style(&self, row: u32, col: u32, style: serde_json::Value) -> napi::Result<()> {
        // Use the raw setter to bypass the napi-rs setter codegen
        // (#[napi(setter)] renames the function, making it unreachable
        // when called from another Rust method).
        if style.is_null() {
            self.with_cell_mut(row, col, |cell| cell.set_style_raw(None));
            return Ok(());
        }
        let parsed: crate::model::style::Style =
            serde_json::from_value(style).map_err(|e| napi::Error::from_reason(format!("style: {e}")))?;
        if parsed.is_empty() {
            self.with_cell_mut(row, col, |cell| cell.set_style_raw(None));
            return Ok(());
        }
        let validated = parsed.validate().map_err(|e| napi::Error::from_reason(e.to_string()))?;
        self.with_cell_mut(row, col, |cell| cell.set_style_raw(Some(validated)));
        Ok(())
    }

    /// Replace the worksheet's column definitions.
    ///
    /// Accepts a JS array of column descriptor objects (header, key, width,
    /// optional hidden, optional style). Parsed server-side via serde.
    /// Each column's style is validated (matching `Cell.set_style` behavior).
    /// Replace the worksheet's column definitions.
    ///
    /// Accepts a JS array of column descriptor objects (header, key, width,
    /// optional `colNum`, optional hidden, optional style).  Parsed
    /// server-side via serde.  Each column's style is validated (matching
    /// `Cell.set_style` behavior).
    ///
    /// `colNum` auto-assignment: columns with `colNum == 0` get sequential
    /// numbers starting from `max(existing col_nums) + 1` (or 1 if none
    /// exist).  Duplicate `colNum` values across the same call are rejected.
    #[napi]
    pub fn set_columns(&self, cols: serde_json::Value) -> napi::Result<()> {
        let mut columns = self.columns.lock().expect("Worksheet columns lock poisoned");
        let mut parsed: Vec<Column> =
            serde_json::from_value(cols).map_err(|e| napi::Error::from_reason(format!("columns: {e}")))?;

        // Auto-assign col_num for entries with col_num == 0
        let next_col_num = columns.iter().map(|c| c.col_num()).max().unwrap_or(0) + 1;
        let mut next_auto = next_col_num;
        for col in &mut parsed {
            if col.col_num() == 0 {
                col.col_num = next_auto;
                next_auto += 1;
            }
        }

        // Validate uniqueness
        {
            let mut seen = std::collections::HashSet::new();
            for col in &parsed {
                if !seen.insert(col.col_num()) {
                    return Err(napi::Error::from_reason(format!(
                        "columns: duplicate col_num {}",
                        col.col_num()
                    )));
                }
            }
        }

        // Validate styles (matching Cell.set_style behavior)
        for col in &mut parsed {
            if let Some(style) = col.style.take() {
                if style.is_empty() {
                    col.style = None;
                } else {
                    col.style = Some(style.validate().map_err(|e| napi::Error::from_reason(e.to_string()))?);
                }
            }
        }

        *columns = parsed;
        Ok(())
    }
}

// Internal methods (not exposed via napi)
impl Worksheet {
    /// Set the worksheet id (used by Workbook/reader).
    pub fn set_id(&mut self, id: u32) {
        self.id = id;
    }

    /// Lock rows, get-or-create row + cell, call `f` on the mutable cell ref.
    fn with_cell_mut<F>(&self, row: u32, col: u32, f: F)
    where
        F: FnOnce(&mut Cell),
    {
        let mut rows = self.rows.lock().expect("Worksheet rows lock poisoned");
        let ws_row = rows.entry(row).or_insert_with(|| Row::new(row));
        let cell = ws_row.get_or_create_cell_mut(col);
        f(cell);
    }

    /// Insert a cell value at (row, col) — used by the reader.
    pub fn insert_cell_value(&self, row: u32, col: u32, value: CellValue) {
        self.with_cell_mut(row, col, |cell| cell.set_value_raw(value));
    }

    /// Set a formula string on a cell at (row, col) — used by the reader.
    pub fn insert_cell_formula(&self, row: u32, col: u32, formula: String) {
        self.with_cell_mut(row, col, |cell| cell.set_formula(Some(formula)));
    }

    /// Set the style on a cell at (row, col) — used by the reader.
    pub fn insert_cell_style(&self, row: u32, col: u32, style: Style) {
        self.with_cell_mut(row, col, |cell| cell.set_style_raw(Some(style)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::style::Font;

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

    #[test]
    fn test_cell_style_mutation_persists_through_clone() {
        // Regression: getCell().style = {...} on a cloned Worksheet must persist.
        let ws = Worksheet::new("Sheet1".into());
        ws.add_row(vec![serde_json::json!("hello")]);

        // Get cell, set style, clone worksheet
        let mut cell = ws.get_cell_by_address("A1".into());
        cell.set_style(serde_json::json!({
            "font": { "bold": true, "color": "FFFF0000" },
            "fill": { "kind": "solid", "foreground": "FFFFFF00" }
        }))
        .unwrap();

        // Clone simulates napi-rs FFI boundary crossing
        let cloned = ws.clone();
        let cell_from_clone = cloned.get_cell_by_address("A1".into());
        let style = cell_from_clone.style().unwrap();

        assert_eq!(style.font.as_ref().and_then(|f| f.bold), Some(true));
        assert_eq!(
            style.fill.as_ref().and_then(|f| f.foreground.clone()),
            Some("FFFFFF00".into())
        );
    }

    #[test]
    fn test_cell_value_mutation_persists_through_clone() {
        // Regression: getCell().value = x on a cloned Worksheet must persist.
        let ws = Worksheet::new("Sheet1".into());
        ws.add_row(vec![serde_json::json!(1)]);

        let mut cell = ws.get_cell_by_address("A1".into());
        cell.set_value(serde_json::json!(42));

        // Clone simulates napi-rs FFI boundary crossing
        let cloned = ws.clone();
        let cell_from_clone = cloned.get_cell_by_address("A1".into());
        let v = cell_from_clone.value();

        assert_eq!(v.value_type, "Number");
        assert_eq!(v.number, Some(42.0));
    }

    #[test]
    fn test_cell_style_mutation_on_standalone_cell_shared_arc() {
        // Two clones of the same cell share the same Arc<Mutex<CellInner>>.
        let mut cell = Cell::new("A1".into(), 1, 1);
        cell.set_style_raw(Some(Style {
            font: Some(Font {
                bold: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        }));

        let cloned_cell = cell.clone();
        assert_eq!(cloned_cell.style().unwrap().font.unwrap().bold, Some(true));
    }

    #[test]
    fn test_missing_row_getcell_persists() {
        // Regression: getCell on a row that doesn't exist yet must return a cell
        // that shares the worksheet's internal Arc<Mutex<CellInner>>, so mutations
        // persist. Before fix B, the returned Cell was standalone and writes were lost.
        let ws = Worksheet::new("Sheet1".into());

        // Get cell at a row that doesn't exist yet
        let mut cell = ws.get_cell_by_rc(5, 1);
        cell.set_value_raw(CellValue::number(42.0));
        // Dropping `cell` — the value should still be in the worksheet's internal map

        // Re-acquire the same cell from the worksheet
        let cell2 = ws.get_cell_by_rc(5, 1);
        assert_eq!(
            cell2.value().number,
            Some(42.0),
            "value set on missing-row getCell must persist"
        );
    }
}
