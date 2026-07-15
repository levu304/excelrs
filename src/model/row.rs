//! Row: a collection of cells indexed by column number.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use napi_derive::napi;

use super::cell::Cell;
use crate::model::style::Style;
use crate::types;

/// A row in a worksheet.
///
/// Cells are stored in a `HashMap<u32, Cell>` keyed by 1-indexed column number.
/// The row number is 1-indexed. Accessing a cell by column creates an empty cell
/// if one doesn't exist — the returned Cell is a clone (see clone-on-read semantics
/// in `cell.rs`).
#[napi]
#[derive(Clone, Debug)]
pub struct Row {
    number: u32,
    cells: HashMap<u32, Cell>,
    height: Arc<Mutex<Option<f64>>>,
    hidden: Arc<Mutex<bool>>,
    style: Arc<Mutex<Option<Style>>>,
}

#[napi]
impl Row {
    #[napi(constructor)]
    pub fn new(number: u32) -> Self {
        Row {
            number,
            cells: HashMap::new(),
            height: Arc::new(Mutex::new(None)),
            hidden: Arc::new(Mutex::new(false)),
            style: Arc::new(Mutex::new(None)),
        }
    }

    #[napi(getter)]
    pub fn number(&self) -> u32 {
        self.number
    }

    // -- height --

    #[napi(getter)]
    pub fn height(&self) -> Option<f64> {
        *self.height.lock().expect("Row height lock poisoned")
    }

    #[napi(setter)]
    pub fn set_height(&mut self, val: Option<f64>) {
        *self.height.lock().expect("Row height lock poisoned") = val;
    }

    // -- hidden --

    #[napi(getter)]
    pub fn hidden(&self) -> bool {
        *self.hidden.lock().expect("Row hidden lock poisoned")
    }

    #[napi(setter)]
    pub fn set_hidden(&mut self, val: bool) {
        *self.hidden.lock().expect("Row hidden lock poisoned") = val;
    }

    // -- style --

    #[napi(getter)]
    pub fn style(&self) -> Option<Style> {
        self.style.lock().expect("Row style lock poisoned").clone()
    }

    #[napi(setter)]
    pub fn set_style(&mut self, val: serde_json::Value) -> napi::Result<()> {
        if val.is_null() {
            *self.style.lock().expect("Row style lock poisoned") = None;
            return Ok(());
        }
        let style: Style = serde_json::from_value(val).map_err(|e| napi::Error::from_reason(format!("style: {e}")))?;
        if style.is_empty() {
            *self.style.lock().expect("Row style lock poisoned") = None;
            return Ok(());
        }
        *self.style.lock().expect("Row style lock poisoned") =
            Some(style.validate().map_err(|e| napi::Error::from_reason(e.to_string()))?);
        Ok(())
    }

    /// Get cell by 1-indexed column number. Creates an empty cell if none exists.
    /// This is the Rust backing for `Row.getCell(col: number)`.
    #[napi]
    pub fn get_cell_by_col_num(&mut self, col: u32) -> Cell {
        self.get_or_create_cell_mut(col).clone()
    }

    /// Get cell by column letter. Creates an empty cell if none exists.
    /// This is the Rust backing for `Row.getCell(col: string)`.
    #[napi]
    pub fn get_cell_by_col_letter(&mut self, col_letter: String) -> Cell {
        let col = types::col_letter_to_num(&col_letter).unwrap_or(0); // returns empty cell for invalid column letters
        self.get_cell_by_col_num(col)
    }
}

// Internal methods (not exposed via napi)
impl Row {
    /// Get a mutable reference to a cell by column number.
    /// Creates the cell if it doesn't exist.
    pub fn get_or_create_cell_mut(&mut self, col: u32) -> &mut Cell {
        let number = self.number;
        self.cells
            .entry(col)
            .or_insert_with(|| Cell::new(Cell::compute_address(number, col), number, col))
    }

    /// Set a cell's value directly (used by add_row, reader).
    pub fn set_cell_value(&mut self, col: u32, value: super::cell::CellValue) {
        let cell = self.get_or_create_cell_mut(col);
        cell.set_value_raw(value);
    }

    /// Number of cells in this row.
    pub fn cell_count(&self) -> usize {
        self.cells.len()
    }

    /// Maximum column number in this row (0 if empty).
    pub fn max_col(&self) -> u32 {
        self.cells.keys().copied().max().unwrap_or(0)
    }

    /// All cells as a sorted Vec by column number.
    pub fn sorted_cells(&self) -> Vec<&Cell> {
        let mut keys: Vec<_> = self.cells.keys().copied().collect();
        keys.sort_unstable();
        keys.iter().map(|k| &self.cells[k]).collect()
    }

    /// Like `sorted_cells` but filters out cells that are effectively empty
    /// (no value, no formula, no style). Used by the writer to avoid emitting
    /// phantom cells created by read-side `getCell`.
    pub fn written_cells(&self) -> Vec<&Cell> {
        self.sorted_cells()
            .into_iter()
            .filter(|c| !c.is_effectively_empty())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::cell::CellValue;

    #[test]
    fn test_row_new() {
        let row = Row::new(1);
        assert_eq!(row.number(), 1);
        assert!(row.height().is_none());
        assert!(!row.hidden());
        assert_eq!(row.cell_count(), 0);
    }

    #[test]
    fn test_row_get_cell_by_col_num_creates_empty() {
        let mut row = Row::new(5);
        let cell = row.get_cell_by_col_num(3);
        assert_eq!(cell.row(), 5);
        assert_eq!(cell.col(), 3);
        assert_eq!(cell.address(), "C5");
        assert_eq!(cell.value_raw().value_type, "Null");
    }

    #[test]
    fn test_row_get_cell_by_col_letter() {
        let mut row = Row::new(10);
        let cell = row.get_cell_by_col_letter("AA".into());
        assert_eq!(cell.row(), 10);
        assert_eq!(cell.col(), 27);
        assert_eq!(cell.address(), "AA10");
    }

    #[test]
    fn test_row_get_cell_returns_set_value() {
        let mut row = Row::new(1);
        row.set_cell_value(1, CellValue::number(42.0));
        let cell = row.get_cell_by_col_num(1);
        assert_eq!(cell.value_raw().number, Some(42.0));
    }

    #[test]
    fn test_row_max_col() {
        let mut row = Row::new(1);
        assert_eq!(row.max_col(), 0);
        row.set_cell_value(5, CellValue::string("hello"));
        assert_eq!(row.max_col(), 5);
    }

    #[test]
    fn test_row_sorted_cells() {
        let mut row = Row::new(1);
        row.set_cell_value(3, CellValue::string("c"));
        row.set_cell_value(1, CellValue::string("a"));
        row.set_cell_value(2, CellValue::string("b"));
        let sorted = row.sorted_cells();
        assert_eq!(sorted.len(), 3);
        assert_eq!(sorted[0].col(), 1);
        assert_eq!(sorted[1].col(), 2);
        assert_eq!(sorted[2].col(), 3);
    }
}
