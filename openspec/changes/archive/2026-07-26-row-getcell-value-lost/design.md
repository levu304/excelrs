# Design: Row.getCell() value mutations lost on cloned rows

## Context

Bug fix. Internal architectural change to `Row` in `src/model/row.rs`. All other mutable fields on Row (`height`, `hidden`, `style`, `outline_level`) use `Arc<Mutex<T>>` for interior mutability — `cells` alone uses a plain `HashMap<u32, Cell>`. When `Worksheet::get_row` clones the Row (required by napi-rs pass-by-value), the cells HashMap is deep-copied and disconnected from the worksheet's original row. New cells created via `cloned_row.getCell(…)` are orphaned.

## Approach

### Change: `Row::cells` → `Arc<Mutex<HashMap<u32, Cell>>>`

```rust
// Before
pub struct Row {
    cells: HashMap<u32, Cell>,
    height: Arc<Mutex<Option<f64>>>,
    hidden: Arc<Mutex<bool>>,
    style: Arc<Mutex<Option<Style>>>,
    outline_level: Arc<Mutex<u8>>,
}

// After
pub struct Row {
    cells: Arc<Mutex<HashMap<u32, Cell>>>,
    // ... other fields unchanged
}
```

This changes the **clone semantics** of Row: cloning increments the `cells` Arc refcount — both original and cloned Row point at the same `HashMap`. Cells created via `cloned_row.getCell("A")` write into the same map the writer reads from.

No changes to `Row`'s constructor (`Row::new`), `#[napi]` getters, or setters (`number`, `height`, `hidden`, `style`, `outline_level`).

### Lock ordering

```
worksheet.rows (Mutex A) → row.cells (Mutex B) → cell.inner (Mutex C)
```

Already respected everywhere: `Worksheet::with_cell_mut` locks A then B then C. Writer locks A then B. No path locks B then A. No path locks C then B.

**Deadlock risk: none.** All existing code already locks worksheet.rows first; adding cell-level locking inside Row preserves the same order.

### All callers of `self.cells` in `Row` (11 sites)

| # | Method | Current pattern | New pattern |
| --- | -------- | ---------------- | ------------- |
| 1 | `get_or_create_cell_mut(col)` | `self.cells.entry(col).or_insert_with(…)` | `self.cells.lock().unwrap().entry(col).or_insert_with(…)` |
| 2 | `set_cell_value(col, value)` | calls `get_or_create_cell_mut` | unchanged — delegates |
| 3 | `get_cell_by_col_num(col)` | calls `get_or_create_cell_mut` | unchanged — delegates |
| 4 | `get_cell_by_col_letter(col)` | calls `get_cell_by_col_num` | unchanged |
| 5 | `cell_count()` | `self.cells.len()` | `self.cells.lock().unwrap().len()` |
| 6 | `max_col()` | `self.cells.keys().max()` | `self.cells.lock().unwrap().keys().max()` |
| 7 | `sorted_cells()` | iterates & sorts cells | lock, collect sorted clones |
| 8 | `written_cells()` | calls `sorted_cells` + filter | unchanged — delegates |
| 9 | `detach_styles()` | `self.cells.iter().map( | (k,v) | (k,v.deep_clone())).collect()` | lock, clone the whole HashMap, replace Arc |
| 10 | `clear_styles()` | `self.cells.values_mut()` | lock, iterate values_mut |
| 11 | `renumber(new_number)` | `self.cells.values_mut()` | lock, iterate values_mut |

### Detailed changes per method

#### `get_or_create_cell_mut` — CANNOT return `&mut Cell` from behind a Mutex

Current signature: `pub fn get_or_create_cell_mut(&mut self, col: u32) -> &mut Cell`

With `Arc<Mutex<HashMap>>`, a `MutexGuard` is returned, so we can't return a borrowed `&mut Cell` — the guard must live long enough.

**Option A** — Change to return `Cell` clone (Cell's interior mutability via Arc makes this safe):

```rust
pub fn get_or_create_cell(&self, col: u32) -> Cell {
    let mut cells = self.cells.lock().unwrap();
    cells.entry(col)
        .or_insert_with(|| Cell::new(Cell::compute_address(self.number, col), self.number, col))
        .clone()
}
```

Then `set_cell_value` no longer borrows:

```rust
pub fn set_cell_value(&self, col: u32, value: CellValue) {
    let cell = self.get_or_create_cell(col);
    cell.set_value_raw(value);
}
```

**Option B** — Use `with_cell_mut` closure pattern (like Worksheet):

```rust
pub fn with_cell_mut<F>(&self, col: u32, f: F)
where F: FnOnce(&mut Cell)
{
    let mut cells = self.cells.lock().unwrap();
    let cell = cells.entry(col)
        .or_insert_with(|| Cell::new(...));
    f(cell);
}
```

**Decision:** Option A. Simpler. All internal callers just need a Cell handle — Cell's interior mutability (Arc) means the clone shares state. Option B would require converting every public method to closures.

**Impact on `get_cell_by_col_num`** (napi API): already returns `Cell` by clone. Just changes which method is called:

```rust
pub fn get_cell_by_col_num(&self, col: u32) -> Cell {  // was &mut self, now &self
    self.get_or_create_cell(col)
}
```

The `&mut self` → `&self` is a nice bonus: row methods that only access cells no longer need mutable self.

#### `sorted_cells` and `written_cells`

`sorted_cells` currently returns `Vec<&Cell>` — borrowed references. With a Mutex-locked HashMap, we can't return borrows. Return `Vec<Cell>` (clones) instead:

```rust
pub fn sorted_cells(&self) -> Vec<Cell> {
    let cells = self.cells.lock().unwrap();
    let mut keys: Vec<_> = cells.keys().copied().collect();
    keys.sort_unstable();
    keys.iter().map(|k| cells[k].clone()).collect()
}
```

`written_cells` calls `sorted_cells` then `.into_iter().filter(...)`. Since sorted_cells now returns owned Vec<Cell>, written_cells works as-is but the filter calls `cell.is_effectively_empty()` on owned Cell (which borrows &self).

#### `detach_styles`

Currently replaces the cells HashMap with a new one where each Cell's Arc<Mutex<CellInner>> is independent (deep clone). With Arc<Mutex<HashMap>>:

```rust
pub fn detach_styles(&mut self) {
    let cells_lock = self.cells.lock().unwrap();
    let cloned: HashMap<_, _> = cells_lock.iter()
        .map(|(k, v)| (*k, v.deep_clone()))
        .collect();
    self.cells = Arc::new(Mutex::new(cloned));
    // ... also detach other Arc fields (unchanged)
}
```

#### `clear_styles`

```rust
pub fn clear_styles(&mut self) {
    for cell in self.cells.lock().unwrap().values_mut() {
        cell.set_style_raw(None);
    }
}
```

#### `renumber`

```rust
pub fn renumber(&mut self, new_number: u32) {
    self.number = new_number;
    for cell in self.cells.lock().unwrap().values_mut() {
        cell.renumber(new_number);
    }
}
```

## Lock contention notes

`Row` methods now acquire the cells Mutex per call. This is **not a concern** because:

1. The Mutex is uncontended in practice — only the JS thread touches rows
2. Existing code already locks `worksheet.rows` (an outer Mutex) before reaching Row methods
3. The lock is held briefly (HashMap entry insert or lookup)
4. `sorted_cells` and `written_cells` hold the lock for the full sort — but these are called by the writer only, once per row

## Table of callers outside Row

| File | What it uses | Impact |
| ------ | ------------- | -------- |
| `reader/xlsx.rs` | `ws_row.get_or_create_cell_mut(col)` | Changes to `get_or_create_cell(col)` (clone-based) |
| `reader/xlsx.rs` | `ws_row.cell_count()` | Add `.lock().unwrap()` — or better, expose a method |
| `reader/xlsx.rs` | Various Row uses via Worksheet | No change — Worksheet methods lock rows externally |
| `writer/xlsx.rs` | `row.written_cells()`, `row.sorted_cells()` | Return type changes `Vec<&Cell>` → `Vec<Cell>` — iterator adapters unchanged |
| `writer/styles.rs` | `row.cell_count()`, `row.written_cells()` | Same return-type adjustment |

**Reader callers of `get_or_create_cell_mut`**: locate exact lines to confirm.

## Risk Assessment

| Risk | Likelihood | Mitigation |
| ------ | ----------- | ------------ |
| `get_or_create_cell_mut` returns `&mut Cell` — won't compile behind Mutex | Certain | Option A (clone-based) resolves; compile-time catch |
| `sorted_cells`/`written_cells` return type change breaks writer callers | Medium | Compile-time catch — adapt writer iterators |
| Lock contention in hot path | Low | Single-threaded JS; mutex uncontended |
| `detach_styles` or `clear_styles` double-lock | Low | No call path holds cells lock when calling these (called from Worksheet with rows lock, not cells lock) |
| `renumber` hold lock during cell renumber | Low | Same as clear_styles — no re-entrant locking |
