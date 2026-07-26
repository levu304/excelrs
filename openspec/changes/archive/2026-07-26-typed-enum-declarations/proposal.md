## Why

The auto-generated `index.d.ts` declares all Rust `String` fields as TypeScript `string`, losing type safety for ~20 enum-like fields. Users get no autocomplete, no compile-time validation, and can pass invalid values (`"pizza"` instead of `"solid"`) without errors. This undermines the ExcelJS drop-in promise — ExcelJS provides proper TS enum types for these fields.

## What Changes

- **Style enums**: `Fill.kind`, `BorderStyle.style`, `Alignment.horizontal`, `Alignment.vertical`, `Fill.gradientType` → string literal union types
- **Conditional formatting enums**: `CfRule.type`, `CfRule.operator`, `CfRule.timePeriod`, `Cfvo.type` → string literal union types
- **Data validation enums**: `DataValidation.type`, `DataValidation.operator`, `DataValidation.errorStyle` → string literal union types
- **Sheet view enums**: `SheetView.state`, `SheetView.activePane` → string literal union types
- **Image enums**: `ImageAnchor.anchorType` → string literal union type
- **Page setup enums**: `PageSetup.orientation`, `PageSetup.cellComments` → string literal union types
- **Cell value discriminant**: `CellValue.valueType` → string literal union type
- **Workbook view**: `WorkbookView.visibility` → string literal union type
- **No breaking JS API changes** — all values remain strings at runtime; only TypeScript-level types improve

Split approach:

- **Simple enums** (Fill.kind, BorderStyle.style, Alignment.hv, etc.): convert Rust `String` fields to `#[napi(string_enum)]` where feasible without breaking API
- **Complex discriminators** (CellValue.valueType, CfRule.type, DataValidation.type): hand-maintain TS string literal unions in a companion declaration file or post-generation patch

## Capabilities

### New Capabilities

- `typed-enum-declarations`: TypeScript type safety for all enum-like string fields in the public API surface, via a combination of Rust `string_enum` conversions where possible and a TS-level declaration overlay where not

### Modified Capabilities

*(none — no existing spec requires behavioral changes; this is purely type-declaration work)*

## Impact

- **`index.d.ts`**: Modified (post-generation patching) or supplemented with companion file
- **Rust `String` fields in `#[napi(object)]` structs**: Some converted to `#[napi(string_enum)]` — no JS API change, values stay as strings
- **`index.js`**: Unchanged (JS glue is irrelevant for enum types)
- **`native.d.ts`**: Mirrors `index.d.ts` changes (auto-generated; regenerate after Rust changes)
- **No runtime impact**: All changes are TypeScript-only; same JS values work identically
