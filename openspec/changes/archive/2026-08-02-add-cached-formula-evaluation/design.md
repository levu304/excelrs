# Design: Cached Formula Evaluation

## Context

excelrs is a `Workbook(Arc<Mutex<WorkbookInner>>)` I/O layer (calamine read,
zip+quick-xml write, `WorkbookStream` SAX streaming). Formula strings are
preserved as opaque `<f>` payloads. The only model seam is read accessors
on `Worksheet` (`get_cell_by_address`, `get_cell_by_rc`) and `Cell`
(`formula()` read-only, `value_raw()` returns `CellValue`).

`xlstream-parse` v0.4.0 wraps the surviving `formulizer-parse` v2.0.0 parser
(re-exports via transitive deps). It provides `parse()`, `Ast::root()`,
`NodeRef::view()`, and child accessors. `xlstream-core` v0.4.0 provides
`Value`, `CellError`, `ExcelDate` types. Both are small crates (<5 KB each)
with no Arrow/rayon dependency.

## Architecture

```
src/formula/
├── mod.rs      — module root, Scalar type alias
└── bridge.rs   — FormulaEvaluator (~450 lines): AST walker + cell resolver
                  + operator dispatch + function table

Cell (src/model/cell.rs)       — additive cachedValue getter
Worksheet (src/model/worksheet.rs) — additive recalculate() method
Writer (src/writer/xlsx.rs)    — existing <f>/<v> path, just needs cached value populated
```

The evaluator is **stateless per-call**: `FormulaEvaluator` borrows
`&Worksheet` + `Option<&WorkbookInner>`, evaluates a formula, returns the
`Scalar`. No engine state stored on `WorkbookInner`.

## Decisions

### 1. Engine: xlstream-parse + custom evaluator

`formularizer-eval` (400+ functions) was yanked. `xlstream-parse` provides
the identical AST structure (same `formulizer-parse` source). We write a
custom evaluator for the ~20 most-used Excel functions, avoiding the
Arrow/rayon dependency weight entirely.

- **Alternative:** Build parser from scratch — rejected: 800+ lines of
  tokenizer/parser/duodemo edge cases; `xlstream-parse` already wraps the
  battle-tested parser.
- **Alternative:** `xlstream-core` streaming evaluator — rejected:
  row-by-row streaming can't resolve whole-range refs; not single-cell
  eval compatible.

### 2. Cargo feature `formula-eval` (off by default)

Gates `xlstream-parse` + `xlstream-core` deps and the evaluation code.

```toml
[features]
formula-eval = ["dep:xlstream-parse", "dep:xlstream-core"]
```

Default build graph is unchanged — no overhead for existing consumers.

### 3. Stateless per-cell evaluation

`Worksheet::recalculate()` iterates formula cells, creating a fresh
`FormulaEvaluator` for each. Cycle detection uses a `HashSet<CellKey>`
threaded through the evaluation — seeded with the formula cell's address
to catch self-references.

- **Alternative:** Store per-workbook engine in `WorkbookInner` — rejected:
  would mutate the shared model under `Arc<Mutex<_>>` and force evaluation
  state into every build.

### 4. Range resolution

`NodeView::RangeRef` may have `Option<u32>` bounds for whole-column
(`A:A`) or whole-row (`1:1`) references. The evaluator resolves these
against `Worksheet::row_count()` / `column_count()` to bound the grid.

### 5. Error propagation

Evaluation produces `Outcome { Value | Error }`. Errors are "sticky":
any error operand propagates to the result (Excel semantics). Cell
errors (`#DIV/0!`, `#N/A`, etc.) are stored as `CellError` and rendered
as string sentinels via `cell_error_to_string`.

### 6. Write path integration

`Cell.cachedValue` getter reads from `CellValue`'s cached fields
(`number`/`string`/`boolean`/`error_value`). The writer already emits
`<v>` at `writer/xlsx.rs:1845-1850` — it just needs the cached value
populated, which `recalculate` does.

### 7. Streaming path caveat

`WorkbookStream` (`stream.rs`) deliberately drops cached `<v>` on its
read path. `Cell.cachedValue` returns `null` for streaming cells.
This is a documented gap, not a regression — the streaming reader
preserves formula strings only.

## Risks & Trade-offs

- **Limited function set (20 vs 400+):** Consumers needing functions like
  VLOOKUP, INDEX, etc. fall back to `#NAME?`. Mitigation: functions are
  dispatched via a table in `call_function`; new functions can be added
  incrementally.
- **No dynamic arrays / spill ranges:** Out of scope for first phase;
  `NodeView::Array` is supported for inline `{1,2;3,4}` array constants.
- **Recursive cell resolution on Formula cells:** When a referenced cell
  is itself a Formula, we parse and evaluate its formula inline using
  `parse()` again. The `Ast` lives on the stack frame so `NodeRef`
  borrows are valid within the scope.
