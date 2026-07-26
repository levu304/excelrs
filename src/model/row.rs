//! Row: a collection of cells indexed by column number.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use napi_derive::napi;

use super::cell::Cell;
use super::cell::CellValue;
use crate::model::style::{apply_style, Style};
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
    cells: Arc<Mutex<HashMap<u32, Cell>>>,
    height: Arc<Mutex<Option<f64>>>,
    hidden: Arc<Mutex<bool>>,
    style: Arc<Mutex<Option<Style>>>,
    outline_level: Arc<Mutex<u8>>,
}

#[napi]
impl Row {
    #[napi(constructor)]
    pub fn new(number: u32) -> Self {
        Row {
            number,
            cells: Arc::new(Mutex::new(HashMap::new())),
            height: Arc::new(Mutex::new(None)),
            hidden: Arc::new(Mutex::new(false)),
            style: Arc::new(Mutex::new(None)),
            outline_level: Arc::new(Mutex::new(0)),
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

    // -- outline level (grouping) --

    /// Outline/grouping level for this row, `0`–`7` (Excel's cap). `0` means no grouping.
    #[napi(getter)]
    pub fn outline_level(&self) -> u32 {
        *self.outline_level.lock().expect("Row outline_level lock poisoned") as u32
    }

    /// Set the outline/grouping level. Values are clamped to `0`–`7`.
    #[napi(setter)]
    pub fn set_outline_level(&mut self, val: u32) {
        *self.outline_level.lock().expect("Row outline_level lock poisoned") = val.min(7) as u8;
    }

    #[napi(getter)]
    pub fn style(&self) -> Option<Style> {
        self.style.lock().expect("Row style lock poisoned").clone()
    }

    #[napi(setter)]
    pub fn set_style(&mut self, val: Option<Style>) -> napi::Result<()> {
        let mut guard = self.style.lock().expect("Row style lock poisoned");
        apply_style(&mut guard, val)
    }

    /// Get cell by 1-indexed column number. Creates an empty cell if none exists.
    /// This is the Rust backing for `Row.getCell(col: number)`.
    #[napi]
    pub fn get_cell_by_col_num(&self, col: u32) -> Cell {
        self.get_or_create_cell(col)
    }

    /// Get cell by column letter. Creates an empty cell if none exists.
    /// This is the Rust backing for `Row.getCell(col: string)`.
    #[napi]
    pub fn get_cell_by_col_letter(&self, col_letter: String) -> Cell {
        let col = types::col_letter_to_num(&col_letter).unwrap_or(0); // returns empty cell for invalid column letters
        self.get_cell_by_col_num(col)
    }
}

// Internal methods (not exposed via napi)
impl Row {
    /// Get a cell by column number, creating it if it doesn't exist.
    /// Returns a cloned Cell — the clone shares the underlying `Arc<Mutex<CellInner>>`
    /// so mutations via the clone are visible to all references to this row.
    pub fn get_or_create_cell(&self, col: u32) -> Cell {
        let mut cells = self.cells.lock().expect("Row cells lock poisoned");
        cells
            .entry(col)
            .or_insert_with(|| Cell::new(Cell::compute_address(self.number, col), self.number, col))
            .clone()
    }

    /// Set a cell's value directly (used by add_row, reader).
    pub fn set_cell_value(&self, col: u32, value: super::cell::CellValue) {
        let mut cell = self.get_or_create_cell(col);
        cell.set_value_raw(value);
    }

    /// Set the row style directly from a resolved `Style` — used by the reader.
    /// Skips JSON validation (the style came from the source file's style table).
    pub fn set_style_raw(&mut self, val: Option<Style>) {
        *self.style.lock().expect("Row style lock poisoned") = val;
    }

    /// Create independent Arc copies of style and cell-inner fields so that
    /// subsequent mutations (e.g. via `clear_styles`) don't corrupt the source row.
    pub(crate) fn detach_styles(&mut self) {
        let style_val = self.style.lock().expect("Row style lock poisoned").clone();
        self.style = Arc::new(Mutex::new(style_val));
        let cloned = {
            let cells_lock = self.cells.lock().expect("Row cells lock poisoned");
            cells_lock
                .iter()
                .map(|(&k, v)| (k, v.deep_clone()))
                .collect::<HashMap<_, _>>()
        };
        self.cells = Arc::new(Mutex::new(cloned));
    }

    /// Internal: clear the row-level style and every cell's style. Used by
    /// `Worksheet.duplicateRow` when `includeStyle` is false.
    pub fn clear_styles(&mut self) {
        *self.style.lock().expect("Row style lock poisoned") = None;
        for cell in self.cells.lock().expect("Row cells lock poisoned").values_mut() {
            cell.set_style_raw(None);
        }
    }

    /// Internal: renumber this row to `new_number`, updating the cached row
    /// number and renumbering every cell (row + A1 address) so addresses stay
    /// consistent after insert/splice/duplicate shifts.
    pub fn renumber(&mut self, new_number: u32) {
        self.number = new_number;
        for cell in self.cells.lock().expect("Row cells lock poisoned").values_mut() {
            cell.renumber(new_number);
        }
    }

    /// Internal: build a row at `number` from a list of JS values, reusing the
    /// same value mapping as `Worksheet.add_row`.
    pub fn from_values(number: u32, values: &[serde_json::Value]) -> Self {
        let row = Row::new(number);
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
        row
    }

    /// Number of cells in this row.
    pub fn cell_count(&self) -> usize {
        self.cells.lock().expect("Row cells lock poisoned").len()
    }

    /// Maximum column number in this row (0 if empty).
    pub fn max_col(&self) -> u32 {
        self.cells
            .lock()
            .expect("Row cells lock poisoned")
            .keys()
            .copied()
            .max()
            .unwrap_or(0)
    }

    /// All cells as a sorted Vec by column number.
    pub fn sorted_cells(&self) -> Vec<Cell> {
        let cells = self.cells.lock().expect("Row cells lock poisoned");
        let mut keys: Vec<_> = cells.keys().copied().collect();
        keys.sort_unstable();
        keys.iter().map(|k| cells[k].clone()).collect()
    }

    /// Like `sorted_cells` but filters out cells that are effectively empty
    /// (no value, no formula, no style). Used by the writer to avoid emitting
    /// phantom cells created by read-side `getCell`.
    pub fn written_cells(&self) -> Vec<Cell> {
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
    use crate::model::style::Font;

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
        let row = Row::new(5);
        let cell = row.get_cell_by_col_num(3);
        assert_eq!(cell.row(), 5);
        assert_eq!(cell.col(), 3);
        assert_eq!(cell.address(), "C5");
        assert_eq!(cell.value_raw().value_type, "Null");
    }

    #[test]
    fn test_row_get_cell_by_col_letter() {
        let row = Row::new(10);
        let cell = row.get_cell_by_col_letter("AA".into());
        assert_eq!(cell.row(), 10);
        assert_eq!(cell.col(), 27);
        assert_eq!(cell.address(), "AA10");
    }

    #[test]
    fn test_row_get_cell_returns_set_value() {
        let row = Row::new(1);
        row.set_cell_value(1, CellValue::number(42.0));
        let cell = row.get_cell_by_col_num(1);
        assert_eq!(cell.value_raw().number, Some(42.0));
    }

    #[test]
    fn test_row_set_style_some() {
        let mut row = Row::new(1);
        let style = Style {
            font: Some(Font {
                bold: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        };
        row.set_style(Some(style)).unwrap();
        assert!(row.style().is_some());
        assert_eq!(row.style().unwrap().font.unwrap().bold, Some(true));
    }

    #[test]
    fn test_row_set_style_none() {
        let mut row = Row::new(1);
        // Pre-set via raw
        row.set_style_raw(Some(Style {
            font: Some(Font {
                bold: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        }));
        assert!(row.style().is_some());
        // Reset with None
        row.set_style(None).unwrap();
        assert!(row.style().is_none());
    }

    #[test]
    fn test_row_set_style_empty_object() {
        let mut row = Row::new(1);
        // {} in JS all-None Style is_empty() normalizes to None
        row.set_style(Some(Style::default())).unwrap();
        assert!(row.style().is_none());
    }

    #[test]
    fn test_row_set_style_rejects_invalid() {
        let mut row = Row::new(1);
        // Empty num_fmt string is invalid
        let invalid = Style {
            num_fmt: Some("".into()),
            ..Default::default()
        };
        assert!(row.set_style(Some(invalid)).is_err());
    }

    #[test]
    fn test_row_max_col() {
        let row = Row::new(1);
        assert_eq!(row.max_col(), 0);
        row.set_cell_value(5, CellValue::string("hello"));
        assert_eq!(row.max_col(), 5);
    }

    #[test]
    fn test_row_sorted_cells() {
        let row = Row::new(1);
        row.set_cell_value(3, CellValue::string("c"));
        row.set_cell_value(1, CellValue::string("a"));
        row.set_cell_value(2, CellValue::string("b"));
        let sorted = row.sorted_cells();
        assert_eq!(sorted.len(), 3);
        assert_eq!(sorted[0].col(), 1);
        assert_eq!(sorted[1].col(), 2);
        assert_eq!(sorted[2].col(), 3);
    }

    #[test]
    fn test_row_getcell_orphan() {
        // Regression: clone a Row, create a cell on the clone, then verify
        // the original Row sees it (cells must use Arc<Mutex<>> to share state).
        let row = Row::new(1);
        let clone = row.clone();
        let mut cell = clone.get_cell_by_col_num(3);
        cell.set_value_raw(CellValue::number(42.0));
        assert_eq!(row.cell_count(), 1, "original row should see cell created on clone");
        let cell_from_original = row.get_cell_by_col_num(3);
        assert_eq!(cell_from_original.value_raw().number, Some(42.0));
    }
}
