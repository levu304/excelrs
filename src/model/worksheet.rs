//! Worksheet: a single sheet in a workbook, containing rows and columns.
//!
//! Rows and columns both use `Arc<Mutex<>>` for **interior mutability**:
//! when a `Worksheet` is cloned (which happens every time napi-rs returns
//! one to JS), both the original and the clone share the same underlying
//! row map and column vector.  Mutations through *any* clone propagate to
//! the one that lives inside the `WorkbookInner` — so `ws.addRow([42])` and
//! `ws.setColumns([...])` work even when `ws` was obtained from
//! `wb.addWorksheet("Sheet1")`.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Arc, Mutex};

use napi_derive::napi;

use super::cell::{Cell, CellValue};
use super::column::Column;
use super::comment::CellComment;
use super::conditional_formatting::ConditionalFormat;
use super::data_validation::DataValidation;
use super::header_footer::HeaderFooter;
use super::image::{AddImageOptions, ImageInfo, WorksheetImage};
use super::page_setup::PageSetup;
use super::row::Row;
use super::sheet_protection::SheetProtection;
use super::sheet_view::SheetView;
use super::style::Dxf;
use super::table::{AddTableOptions, Table, TableColumn, TableList, TableRow};
use crate::model::style::Style;
use crate::types;

/// Convert a raw JSON value (from `AddTableOptions.rows`) into a `CellValue`.
fn table_json_to_cell_value(v: &serde_json::Value) -> CellValue {
    match v {
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
    }
}

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
    merged_ranges: Arc<Mutex<Vec<String>>>,
    data_validations: Arc<Mutex<Vec<DataValidation>>>,
    conditional_formats: Arc<Mutex<Vec<ConditionalFormat>>>,
    auto_filter: Arc<Mutex<Option<String>>>,
    views: Arc<Mutex<Vec<SheetView>>>,
    protection: Arc<Mutex<Option<SheetProtection>>>,
    header_footer: Arc<Mutex<Option<HeaderFooter>>>,
    page_setup: Arc<Mutex<Option<PageSetup>>>,
    images: Arc<Mutex<Vec<WorksheetImage>>>,
    tables: TableList,
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
            merged_ranges: Arc::new(Mutex::new(Vec::new())),
            data_validations: Arc::new(Mutex::new(Vec::new())),
            conditional_formats: Arc::new(Mutex::new(Vec::new())),
            auto_filter: Arc::new(Mutex::new(None)),
            views: Arc::new(Mutex::new(Vec::new())),
            protection: Arc::new(Mutex::new(None)),
            header_footer: Arc::new(Mutex::new(None)),
            page_setup: Arc::new(Mutex::new(None)),
            images: Arc::new(Mutex::new(Vec::new())),
            tables: Arc::new(Mutex::new(Vec::new())),
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

    /// Merge a range of cells (e.g. "A1:C3"). Accepts an A1-style range string.
    /// Validates that the range parses to a rectangular area; stores it for
    /// emission in the writer. Duplicate ranges are silently ignored.
    #[napi]
    pub fn merge_cells(&self, range: String) -> napi::Result<()> {
        // Basic validation: ensure range parses as col:row:col:row
        let parts: Vec<&str> = range.split(':').collect();
        if parts.len() != 2 {
            return Err(napi::Error::from_reason(
                "merge_cells: range must be in format 'A1:C3' (e.g. 'A1:B2')",
            ));
        }
        let (tl, br) = (parts[0], parts[1]);
        let (col1, _row1) = crate::types::parse_address(tl)
            .map_err(|_| napi::Error::from_reason(format!("merge_cells: invalid start address '{tl}'")))?;
        let (col2, _row2) = crate::types::parse_address(br)
            .map_err(|_| napi::Error::from_reason(format!("merge_cells: invalid end address '{br}'")))?;
        if col1 > col2 || _row1 > _row2 {
            return Err(napi::Error::from_reason(
                "merge_cells: start must be before end (e.g. 'A1' before 'C3')",
            ));
        }

        let mut ranges = self
            .merged_ranges
            .lock()
            .expect("Worksheet merged_ranges lock poisoned");
        if !ranges.contains(&range) {
            ranges.push(range);
        }
        Ok(())
    }

    // -- data_validations --

    /// Get all data validations for this worksheet.
    #[napi(getter)]
    pub fn data_validations(&self) -> Vec<DataValidation> {
        self.data_validations
            .lock()
            .expect("Worksheet data_validations lock poisoned")
            .clone()
    }

    /// Add or update a data validation. Upserts by sqref.
    #[napi]
    pub fn add_data_validation(&self, dv: DataValidation) -> napi::Result<()> {
        dv.validate().map_err(|e| napi::Error::from_reason(e.to_string()))?;

        let mut validations = self
            .data_validations
            .lock()
            .expect("Worksheet data_validations lock poisoned");

        // Upsert by sqref: remove old, add new
        validations.retain(|v| v.sqref != dv.sqref);
        validations.push(dv);
        Ok(())
    }

    /// Get data validation for a specific cell reference (sqref).
    #[napi]
    pub fn get_data_validation(&self, sqref: String) -> Option<DataValidation> {
        self.data_validations
            .lock()
            .expect("Worksheet data_validations lock poisoned")
            .iter()
            .find(|v| v.sqref == sqref)
            .cloned()
    }

    // -- conditional formatting (v1.2.0) --

    /// Add or update a conditional format. Upserts by sqref.
    #[napi]
    pub fn add_conditional_formatting(&self, cf: ConditionalFormat) -> napi::Result<()> {
        if cf.sqref.trim().is_empty() {
            return Err(napi::Error::from_reason(
                "ConditionalFormat.sqref must not be empty".to_string(),
            ));
        }
        let mut formats = self
            .conditional_formats
            .lock()
            .expect("Worksheet conditional_formats lock poisoned");
        // Upsert by sqref: remove old, add new.
        formats.retain(|c| c.sqref != cf.sqref);
        // Fail loud: explicit (non-zero) priorities must be worksheet-global unique.
        // ExcelJS silently emits duplicate/ambiguous priorities; we reject them.
        let mut seen: std::collections::HashSet<u32> = std::collections::HashSet::new();
        for c in formats.iter() {
            for r in &c.rules {
                if r.priority != 0 {
                    seen.insert(r.priority);
                }
            }
        }
        for r in &cf.rules {
            if r.priority != 0 && !seen.insert(r.priority) {
                return Err(napi::Error::from_reason(format!(
                    "Duplicate conditional-format priority {}: priorities must be worksheet-global unique",
                    r.priority
                )));
            }
        }
        formats.push(cf);
        Ok(())
    }

    /// Get all conditional formats for this worksheet, grouped by range.
    #[napi]
    pub fn get_conditional_formatting(&self) -> Vec<ConditionalFormat> {
        self.conditional_formats
            .lock()
            .expect("Worksheet conditional_formats lock poisoned")
            .clone()
    }

    // -- auto_filter --

    #[napi(getter)]
    /// Get the worksheet's auto-filter range (e.g. "A1:C1"). Returns `None` if unset.
    pub fn auto_filter(&self) -> Option<String> {
        self.auto_filter
            .lock()
            .expect("Worksheet auto_filter lock poisoned")
            .clone()
    }

    #[napi(setter)]
    /// Set the worksheet's auto-filter range. Pass `null` or `""` to clear.
    pub fn set_auto_filter(&mut self, val: Option<String>) {
        let v = val.filter(|s| !s.is_empty());
        *self.auto_filter.lock().expect("Worksheet auto_filter lock poisoned") = v;
    }

    // -- views --

    #[napi(getter)]
    /// Get the worksheet's view descriptors (freeze/split panes).
    pub fn views(&self) -> Vec<SheetView> {
        self.views.lock().expect("Worksheet views lock poisoned").clone()
    }

    #[napi(setter)]
    /// Set the worksheet's view descriptors.
    pub fn set_views(&mut self, val: Vec<SheetView>) {
        *self.views.lock().expect("Worksheet views lock poisoned") = val;
    }

    // -- protection --

    #[napi(getter)]
    /// Get the worksheet's protection flags. Returns `None` if unprotected.
    pub fn protection(&self) -> Option<SheetProtection> {
        self.protection
            .lock()
            .expect("Worksheet protection lock poisoned")
            .clone()
    }

    #[napi(setter)]
    /// Set the worksheet's protection flags. Pass `null` to clear.
    pub fn set_protection(&mut self, val: Option<SheetProtection>) {
        *self.protection.lock().expect("Worksheet protection lock poisoned") = val;
    }

    // -- header_footer --

    #[napi(getter)]
    /// Get the worksheet's header/footer descriptor. Returns `None` if unset.
    pub fn header_footer(&self) -> Option<HeaderFooter> {
        self.header_footer
            .lock()
            .expect("Worksheet header_footer lock poisoned")
            .clone()
    }

    #[napi(setter)]
    /// Set the worksheet's header/footer descriptor. Pass `null` to clear.
    pub fn set_header_footer(&mut self, val: Option<HeaderFooter>) {
        *self
            .header_footer
            .lock()
            .expect("Worksheet header_footer lock poisoned") = val;
    }

    // -- page_setup --

    #[napi(getter)]
    /// Get the worksheet's page setup / print descriptor. Returns `None` if unset.
    pub fn page_setup(&self) -> Option<PageSetup> {
        self.page_setup
            .lock()
            .expect("Worksheet page_setup lock poisoned")
            .clone()
    }

    #[napi(setter)]
    /// Set the worksheet's page setup / print descriptor. Pass `null` to clear.
    pub fn set_page_setup(&mut self, val: Option<PageSetup>) {
        *self.page_setup.lock().expect("Worksheet page_setup lock poisoned") = val;
    }

    // -- data validations --

    /// Remove data validation for a specific cell reference (sqref).
    #[napi]
    pub fn remove_data_validation(&self, sqref: String) -> bool {
        let mut validations = self
            .data_validations
            .lock()
            .expect("Worksheet data_validations lock poisoned");
        let old_len = validations.len();
        validations.retain(|v| v.sqref != sqref);
        validations.len() < old_len
    }

    // -- images (v1.0.0) --

    /// Add an embedded image to the worksheet. Returns the image index.
    #[napi]
    pub fn add_image(&self, opts: AddImageOptions) -> napi::Result<u32> {
        let mut images = self.images.lock().expect("Worksheet images lock poisoned");
        let idx = images.len() as u32;
        images.push(WorksheetImage {
            extension: opts.extension.clone(),
            buffer: opts.buffer,
            positioning: opts.positioning.unwrap_or_else(|| "oneCell".to_string()),
            anchor: opts.anchor,
            media_index: 0,
        });
        Ok(idx)
    }

    /// Return all embedded images on the worksheet.
    #[napi]
    pub fn get_images(&self) -> Vec<ImageInfo> {
        self.images
            .lock()
            .expect("Worksheet images lock poisoned")
            .iter()
            .map(|img| ImageInfo {
                extension: img.extension.clone(),
                buffer: img.buffer.clone(),
                positioning: img.positioning.clone(),
                anchor: img.anchor.clone(),
            })
            .collect()
    }

    // -- tables (v1.1.0) --

    /// Add a structured table to the worksheet (ExcelJS `ws.addTable`).
    ///
    /// Writes the header row (from `columns` names), the data rows, and the
    /// optional totals row into the referenced cells, then registers the table
    /// model. Returns the created `Table`.
    #[napi]
    pub fn add_table(&self, opts: AddTableOptions) -> napi::Result<Table> {
        let name = opts.name.trim().to_string();
        if name.is_empty() {
            return Err(napi::Error::from_reason("Table name must not be empty"));
        }
        let mut tables = self.tables.lock().expect("Worksheet tables lock poisoned");
        if tables.iter().any(|t| t.name == name) {
            return Err(napi::Error::from_reason(format!(
                "Table '{name}' already exists on this worksheet"
            )));
        }
        let header_row = opts.header_row.unwrap_or(true);
        let totals_row = opts.totals_row.unwrap_or(false);
        let (sc, sr, ec, er) = self
            .parse_ref_range(&opts.ref_range)
            .ok_or_else(|| napi::Error::from_reason(format!("Invalid table ref: {}", opts.ref_range)))?;
        if sc > ec || sr > er {
            return Err(napi::Error::from_reason(format!(
                "Invalid table ref '{}': start must precede end",
                opts.ref_range
            )));
        }
        let width = (ec - sc + 1) as usize;

        // Resolve columns: explicit names, else derive from the first data row.
        let columns: Vec<TableColumn> = if !opts.columns.is_empty() {
            opts.columns.clone()
        } else {
            let first = opts.rows.first().cloned().unwrap_or_default();
            first
                .into_iter()
                .map(|v| TableColumn {
                    name: crate::model::table::cell_text(&table_json_to_cell_value(&v)),
                    ..Default::default()
                })
                .collect()
        };
        if columns.len() != width {
            return Err(napi::Error::from_reason(format!(
                "Table column count ({}) does not match ref width ({width})",
                columns.len()
            )));
        }

        // Data rows must exactly fill the ref (header/totals excluded) so the
        // written cells and the <table> range agree.
        let expected_rows = (er - sr + 1) - (header_row as u32) - (totals_row as u32);
        if opts.rows.len() as u32 != expected_rows {
            return Err(napi::Error::from_reason(format!(
                "Table data row count ({}) does not match ref height (expected {expected_rows})",
                opts.rows.len()
            )));
        }

        // Header row: write column names into the top row of the ref.
        if header_row {
            for (i, col) in columns.iter().enumerate() {
                let c = sc + i as u32;
                self.insert_cell_value(
                    sr,
                    c,
                    CellValue {
                        value_type: "String".into(),
                        string: Some(col.name.clone()),
                        ..Default::default()
                    },
                );
            }
        }

        // Data rows.
        let data_start = if header_row { sr + 1 } else { sr };
        for (ri, row) in opts.rows.iter().enumerate() {
            let r = data_start + ri as u32;
            for (ci, v) in row.iter().enumerate() {
                let c = sc + ci as u32;
                self.insert_cell_value(r, c, table_json_to_cell_value(v));
            }
        }

        // Totals row: write each column's totalsRowLabel into the last ref row.
        if totals_row {
            for (i, col) in columns.iter().enumerate() {
                if let Some(label) = &col.totals_row_label {
                    let c = sc + i as u32;
                    self.insert_cell_value(
                        er,
                        c,
                        CellValue {
                            value_type: "String".into(),
                            string: Some(label.clone()),
                            ..Default::default()
                        },
                    );
                }
            }
        }

        let display_name = opts
            .display_name
            .clone()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| name.clone());
        let emit_filter = opts.auto_filter_enabled.unwrap_or(true);
        let auto = if emit_filter {
            opts.auto_filter.clone().or_else(|| Some(opts.ref_range.clone()))
        } else {
            None
        };

        let table_rows: Vec<TableRow> = opts
            .rows
            .iter()
            .map(|row| TableRow {
                values: row.iter().map(table_json_to_cell_value).collect(),
            })
            .collect();

        let table = Table {
            name: name.clone(),
            display_name,
            ref_range: opts.ref_range.clone(),
            header_row,
            totals_row,
            columns: columns.clone(),
            rows: table_rows,
            style: opts.style.clone(),
            autofilter_ref: auto,
        };
        tables.push(table.clone());
        Ok(table)
    }

    /// Return the table with the given name, or `null` if not found.
    #[napi]
    pub fn get_table(&self, name: String) -> Option<Table> {
        self.tables
            .lock()
            .expect("Worksheet tables lock poisoned")
            .iter()
            .find(|t| t.name == name)
            .cloned()
    }

    /// Return all tables on the worksheet.
    #[napi]
    pub fn get_tables(&self) -> Vec<Table> {
        self.tables.lock().expect("Worksheet tables lock poisoned").clone()
    }

    /// Remove the named table (and its part/relationship); cells stay intact.
    #[napi]
    pub fn remove_table(&self, name: String) -> bool {
        let mut tables = self.tables.lock().expect("Worksheet tables lock poisoned");
        let before = tables.len();
        tables.retain(|t| t.name != name);
        tables.len() < before
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

    /// Accessor for merged ranges (used by writer).
    pub fn get_merged_ranges(&self) -> Vec<String> {
        self.merged_ranges
            .lock()
            .expect("Worksheet merged_ranges lock poisoned")
            .clone()
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

    /// Insert a data validation into the worksheet (used by reader).
    /// Skips invalid DVs (malformed type, empty sqref, etc.).
    pub fn insert_data_validation(&self, dv: DataValidation) {
        if dv.validate().is_err() {
            return;
        }
        let mut validations = self
            .data_validations
            .lock()
            .expect("Worksheet data_validations lock poisoned");
        validations.push(dv);
    }

    // -- images / comments (v1.0.0) --

    /// Insert an image (used by reader).
    pub fn insert_image(&self, img: WorksheetImage) {
        self.images.lock().expect("Worksheet images lock poisoned").push(img);
    }

    /// Internal: read images for the writer.
    pub fn get_images_inner(&self) -> Vec<WorksheetImage> {
        self.images.lock().expect("Worksheet images lock poisoned").clone()
    }

    // -- tables (v1.1.0) --

    /// Internal: read tables for the writer.
    pub fn get_tables_inner(&self) -> Vec<Table> {
        self.tables.lock().expect("Worksheet tables lock poisoned").clone()
    }

    /// Attach a parsed table (used by the reader).
    pub fn insert_table(&self, table: Table) {
        self.tables.lock().expect("Worksheet tables lock poisoned").push(table);
    }

    /// Parse an A1 range (e.g. "A1:C4") into (start_col, start_row, end_col, end_row).
    fn parse_ref_range(&self, ref_: &str) -> Option<(u32, u32, u32, u32)> {
        let (a, b) = ref_.split_once(':')?;
        let (sc, sr) = crate::types::parse_address(a).ok()?;
        let (ec, er) = crate::types::parse_address(b).ok()?;
        Some((sc, sr, ec, er))
    }

    /// Attach a comment to a cell at (row, col) (used by reader & API).
    pub fn insert_cell_comment(&self, row: u32, col: u32, comment: CellComment) {
        self.with_cell_mut(row, col, |cell| {
            cell.set_comment(Some(comment));
        });
    }

    /// Get all cells carrying a comment (used by writer).
    pub fn get_cell_comments(&self) -> Vec<(String, CellComment)> {
        let mut out = Vec::new();
        for row in self.rows() {
            // Use all cells (not `written_cells`) so comment-only cells that
            // carry no value still round-trip.
            for cell in row.sorted_cells() {
                let addr = cell.address();
                if let Some(comment) = cell.comment() {
                    out.push((addr, comment));
                }
            }
        }
        out
    }

    /// Get all data validations for writing (used by writer).
    pub fn get_data_validations(&self) -> Vec<DataValidation> {
        self.data_validations
            .lock()
            .expect("Worksheet data_validations lock poisoned")
            .clone()
    }

    /// Attach a parsed conditional format (used by the reader).
    pub fn insert_conditional_formatting(&self, cf: ConditionalFormat) {
        self.conditional_formats
            .lock()
            .expect("Worksheet conditional_formats lock poisoned")
            .push(cf);
    }

    /// Get all conditional formats for writing (used by writer).
    pub fn get_conditional_formatting_inner(&self) -> Vec<ConditionalFormat> {
        self.conditional_formats
            .lock()
            .expect("Worksheet conditional_formats lock poisoned")
            .clone()
    }

    /// Assign worksheet-global unique `priority` (document order) and resolve
    /// `dxfId` for each conditional-format rule, appending new differential
    /// formats to `dxfs` (deduped by canonical key). Mutates the stored rules.
    pub fn assign_conditional_formatting_dxf_ids(&self, dxfs: &mut Vec<Dxf>, dxf_map: &mut HashMap<String, u32>) {
        let mut cfs = self
            .conditional_formats
            .lock()
            .expect("Worksheet conditional_formats lock poisoned");
        let mut used: HashSet<u32> = HashSet::new();
        // Pass 1: reserve explicit priorities so auto-assignment never collides.
        for cf in cfs.iter() {
            for rule in &cf.rules {
                if rule.priority != 0 {
                    used.insert(rule.priority);
                }
            }
        }
        // Pass 2: auto-assign free priorities, then assign dxfIds for every rule.
        let mut priority = 0u32;
        for cf in cfs.iter_mut() {
            for rule in cf.rules.iter_mut() {
                if rule.priority == 0 {
                    loop {
                        priority += 1;
                        if used.insert(priority) {
                            break;
                        }
                    }
                    rule.priority = priority;
                }
                match &rule.style {
                    Some(style) => {
                        let dxf = Dxf::from_style(style);
                        if dxf.font.is_some() || dxf.fill.is_some() || dxf.border.is_some() || dxf.num_fmt.is_some() {
                            let key = serde_json::to_string(&dxf).unwrap_or_default();
                            let id = if let Some(&existing) = dxf_map.get(&key) {
                                existing
                            } else {
                                let id = dxfs.len() as u32;
                                dxfs.push(dxf);
                                dxf_map.insert(key, id);
                                id
                            };
                            rule.dxf_id = Some(id);
                        } else {
                            rule.dxf_id = None;
                        }
                    }
                    None => rule.dxf_id = None,
                }
            }
        }
    }

    // -- internal setters for reader --

    /// Set the auto-filter range (used by reader).
    pub fn set_auto_filter_range(&self, range: Option<String>) {
        *self.auto_filter.lock().expect("Worksheet auto_filter lock poisoned") = range;
    }

    /// Get the auto-filter range for writing (used by writer).
    pub fn get_auto_filter_range(&self) -> Option<String> {
        self.auto_filter
            .lock()
            .expect("Worksheet auto_filter lock poisoned")
            .clone()
    }

    pub fn set_views_inner(&self, views: Vec<SheetView>) {
        *self.views.lock().expect("Worksheet views lock poisoned") = views;
    }

    pub fn get_views_inner(&self) -> Vec<SheetView> {
        self.views.lock().expect("Worksheet views lock poisoned").clone()
    }

    pub fn set_protection_inner(&self, protection: Option<SheetProtection>) {
        *self.protection.lock().expect("Worksheet protection lock poisoned") = protection;
    }

    pub fn get_protection_inner(&self) -> Option<SheetProtection> {
        self.protection
            .lock()
            .expect("Worksheet protection lock poisoned")
            .clone()
    }

    // -- header_footer (internal, used by reader/writer) --

    pub fn set_header_footer_inner(&self, val: Option<HeaderFooter>) {
        *self
            .header_footer
            .lock()
            .expect("Worksheet header_footer lock poisoned") = val;
    }

    pub fn get_header_footer_inner(&self) -> Option<HeaderFooter> {
        self.header_footer
            .lock()
            .expect("Worksheet header_footer lock poisoned")
            .clone()
    }

    // -- page_setup (internal, used by reader/writer) --

    pub fn set_page_setup_inner(&self, val: Option<PageSetup>) {
        *self.page_setup.lock().expect("Worksheet page_setup lock poisoned") = val;
    }

    pub fn get_page_setup_inner(&self) -> Option<PageSetup> {
        self.page_setup
            .lock()
            .expect("Worksheet page_setup lock poisoned")
            .clone()
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
        assert_eq!(cell.value_raw().value_type, "Null");
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
        assert_eq!(c1.value_raw().string, Some("Alice".into()));

        let c2 = ws.get_cell_by_address("B1".into());
        assert_eq!(c2.value_raw().number, Some(30.0));

        let c3 = ws.get_cell_by_address("C1".into());
        assert_eq!(c3.value_raw().boolean, Some(true));
    }

    #[test]
    fn test_clone_shares_rows() {
        let ws = Worksheet::new("Shared".into());
        let cloned = ws.clone();

        // addRow on the clone should be visible through the original
        cloned.add_row(vec![serde_json::json!(42)]);
        assert_eq!(ws.row_count(), 1);
        assert_eq!(ws.get_cell_by_address("A1".into()).value_raw().number, Some(42.0));
    }

    #[test]
    fn test_remove_row() {
        let ws = Worksheet::new("Sheet1".into());
        ws.add_row(vec![serde_json::json!(1)]);
        ws.add_row(vec![serde_json::json!(2)]);
        assert_eq!(ws.row_count(), 2);

        ws.remove_row(1);
        assert_eq!(ws.row_count(), 2); // row_count is max row, not count of rows
        assert!(ws.get_cell_by_address("A1".into()).value_raw().value_type == "Null");
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
        cell.set_value_raw(CellValue {
            value_type: "Number".into(),
            number: Some(42.0),
            ..Default::default()
        });

        // Clone simulates napi-rs FFI boundary crossing
        let cloned = ws.clone();
        let cell_from_clone = cloned.get_cell_by_address("A1".into());
        let v = cell_from_clone.value_raw();

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
            cell2.value_raw().number,
            Some(42.0),
            "value set on missing-row getCell must persist"
        );
    }
}
