## Context

`excelrs` mirrors exceljs. Defined names (named ranges) live in `xl/workbook.xml` as `<definedName>` children of `<definedNames>`. ExcelJS exposes them as `workbook.definedNames` with `.add(name, value, sheetIndex)` / `.remove()`. The v0.4.0 roadmap's six deferred items shipped in v0.5.0–v0.6.0; spec §9.3 lists defined names as the next roadmap capability.

Currently excelrs reads workbook metadata via calamine (which does not expose `<definedNames>`) and writes `xl/workbook.xml` with a minimal `<sheets>` block — no `<definedNames>` element. No model type exists.

## Goals / Non-Goals

- Parse `<definedNames>` from `xl/workbook.xml` on read (workbook-scope + sheet-scope via `localSheetId`).
- Emit `<definedNames>` on write (without formula evaluation — raw text).
- Expose via `Workbook` napi API.
- No formula resolution: value is stored as raw OOXML text.
- No public API breaking change.

## Decisions

### D1. New `src/model/defined_name.rs` holds `DefinedName`

```rust
pub struct DefinedName {
    pub name: String,
    pub value: String,
    pub sheet: Option<String>,
}
```

`sheet = None` → workbook-scope name. `sheet = Some("Sheet1")` → resolves to `localSheetId="0"` on write by scanning `WorkbookInner.worksheets` for a matching name.

### D2. Reader parses `xl/workbook.xml` via quick_xml

New function `parse_defined_names(data: &[u8], sheet_names: &[String]) -> Result<Vec<DefinedName>, ExcelrsError>` in `src/reader/workbook.rs`. Opens the zip (reusing `Cursor<Vec<u8>>` from `workbook_inner_from_bytes`), reads `xl/workbook.xml`, and iterates `<definedName>` elements with quick_xml. `localSheetId` is mapped to `sheet_names[localSheetId]` (clamped to None if out of range).

### D3. Writer emits `<definedNames>` after `<sheets>`

`write_workbook_xml` gains the `DefinedName` list. When non-empty, writes `<definedNames>` block. Sheet-scoped names (sheet is Some) are mapped to 0-based localSheetId by searching `worksheets[]` for a matching name; if not found, emitted without `localSheetId`.

### D4. NAPI: additive, no signature changes

`Workbook` gains:

- getter `defined_names` → `Vec<DefinedName>` (snapshot).
- `add_defined_name(name, value, sheet: Option<String>)` (upsert: workbook-scope match by name; sheet-scope match by name+sheet).
- `remove_defined_name(name, sheet: Option<String>)` (same lookup).
- `get_defined_name(name, sheet: Option<String>) -> Option<DefinedName>`.

`DefinedName` is a `#[napi(object)]` struct.

### D5. No formula evaluation / resolution

The `value` field stores the raw `<definedName>` text content verbatim (e.g. `"Sheet1!$A$1"` or `"=SUM(Sheet2!B:B)"`). No range-to-string conversion, no formula parsing. This matches what OOXML stores and what exceljs `workbook.definedNames` getter returns.

## Risks / Trade-offs

- **localSheetId ↔ sheet name mapping** assumes sheet names are unique (OOXML allows duplicates; we use first match on write, clamping on read). Acceptable since exceljs and Excel enforce unique names in practice.
- **Upsert semantics on addDefinedName** diverges from exceljs's `.add()` which creates duplicates. Excelrs replaces by (name[, sheet]) to keep the Vec manageable without a separate remove-then-add call.
