# Proposal

## Why

ExcelJS migrants hitting `ws.getColumn` get a `TypeError` (the method does not exist), themed files silently downgrade to static rgb on round-trip (theme-switching in Excel stops working), and legacy array formulas lose their `t="array"` semantics without any documented rationale. Each gap is small; together they are the highest compat-value-per-effort work left outside charts, pivots, and full formula evaluation.

## What Changes

- Add live `Worksheet.getColumn` accepting a 1-indexed number or column letter (JS-glue overload, same pattern as `getCell`), auto-creating the column definition when absent. Mutations through the returned handle (`width`, `hidden`, `style`, `outlineLevel`, `header`, `key`) persist into the worksheet model. Key-based lookup, `spliceColumns`, and `eachColumn` are explicitly out of scope (`addRow` takes positional arrays only, so `key` has no consumer).
- Flip style emission to theme-wins: when a color carries a theme reference, emit `<color theme tint>` instead of the resolved rgb, and pass `xl/theme/theme1.xml` through read-to-write so the reference resolves. No new authoring API (`color_theme`/`tint` stay `#[napi(skip)]`).
- Record the legacy array-formula behavior as specified: `<f t="array" ref>` reads as a plain formula (text plus cached `<v>` preserved on every carrying cell) and writes back as plain `<f>`. Verified by probe: values survive, array-ness does not, Excel opens the result without repair. Preserve-opaque and spill evaluation stay deferred.

## Capabilities

### New Capabilities

- `column-access`: live per-column lookup and mutation by letter or number, including auto-create-on-read and persistence of handle mutations into the worksheet model.

### Modified Capabilities

- `theme-color-references`: emission flips from resolved-ARGB-wins to theme-reference-wins, plus `theme1.xml` read-to-write passthrough.
- `cached-formula-value`: legacy `<f t="array">` degradation documented (text and cached value preserved, array type and ref dropped on write).

## Impact

- Rust: `src/model/column.rs` (live handle via interior mutability), `src/model/worksheet.rs` (`get_column` accessors), `src/writer/styles.rs` (`emit_color_attrs` policy), `src/writer/xlsx.rs` (theme part emission + content types), `src/reader/styles.rs` + `src/reader/xlsx.rs` (retain raw `theme1.xml` bytes).
- JS: `index.js` glue (`getColumn` overload), `index.d.ts` + `native.d.ts` (signatures), `scripts/apply-glue.cjs` pipe output if declarations change shape.
- Tests: Rust unit tests plus vitest round-trips (excelrs-to-excelrs and excelrs-to-ExcelJS); array behavior pinned by a committed fixture test, not new engine code.
- No dependency changes. No breaking changes: `setColumns`/`columns` semantics unchanged, rgb-only files emit byte-identical styles output.
