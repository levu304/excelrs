# Proposal: Row.getCell() value mutations lost on cloned rows

## Why

Cell values set via `row.getCell().value = x` are silently lost when the row was obtained from `worksheet.getRow()`. The value appears to be set in JS (no error), but the writer omits it.

### Root cause

`Row` stores cells in a plain `HashMap<u32, Cell>`, cloned by value when the Row is cloned. `worksheet.getRow()` returns a Row **clone** (napi-rs passes by value). The cloned row's cell HashMap is entirely independent from the original row inside the worksheet. When `row.getCell("A")` creates a new cell, it inserts into the cloned row's HashMap — invisible to the writer, which reads from the worksheet's original row map.

All other mutable Row fields (`height`, `hidden`, `style`, `outline_level`) use `Arc<Mutex<>>` and survive cloning correctly — only `cells` doesn't.

```typescript
const ws = workbook.addWorksheet('test');
const row = ws.getRow(1);
row.height = 50;              // ✓ works (Arc<Mutex<>>)
const cell = row.getCell('A');
cell.value = 'Hello!';        // ✗ silently lost (cells is plain HashMap)
```

### Scope

- Only `row.getCell(…)` → set value is affected
- `ws.getCell(…)` → set value works correctly (directly inserts into the worksheet's row map)
- Pre-existing cells (created via `addRow`, reader, etc.) ARE shared — their `Arc<Mutex<CellInner>>` crosses the clone boundary. The bug only hits when `getCell` **creates a cell that didn't already exist**.

This is a correctness bug: the API surface claims `getRow().getCell().value = x` works (ExcelJS compatibility), but the write path silently drops these values.

## What Changes

Make `Row::cells` use `Arc<Mutex<HashMap<u32, Cell>>>` — the same interior-mutability pattern already used by `height`, `style`, `hidden`, and `outline_level`. All internal methods that access `self.cells` adjust to lock the Mutex.

No API surface changes. No TypeScript type changes. Internal architecture change only.

## Non-Goals

- No new public API
- No spec changes (behavioral spec already says this should work)
- No changes to Worksheet, Cell, or Column types
- No performance regression on hot paths (lock overhead is one additional Mutex acquisition per cell access — negligible vs the already-locked `worksheet.rows` mutex)
