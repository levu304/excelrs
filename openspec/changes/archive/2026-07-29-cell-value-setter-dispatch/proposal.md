## Why

The public `cell.value =` setter silently drops every object value as `Null`. In `src/model/cell.rs` (`set_value`, line 269) the setter takes `napi::Unknown`, converts via `serde_json::Value::from_napi_value`, and only matches Number/String/Bool — any object (including `{ richText: [...] }`) hits the `_ => CellValue::default()` branch. Rich text, hyperlink, and formula values are therefore unreachable from JS, even though the `CellValue` model (`rich_text`) and the XLSX writer (emits `t="inlineStr"` + `<is><r>`) fully support them. This breaks the user's canonical ExcelJS idiom `cell.value = { richText: [...] }` and contradicts `openspec/specs/rich-text/spec.md`, whose scenario already assumes that assignment works. Separately, `index.d.ts:23` types the setter as `set value(val: unknown)`, so `Cell.value` shows as `unknown` with no type safety or autocomplete.

## What Changes

- Widen `Cell.set_value` to dispatch object-shaped values by shape: `richText` → RichText, `hyperlink` → Hyperlink, `formula` → Formula; honors an explicit `valueType` (e.g. `{ valueType: "Date", dateSerial }`) so read-back round-trip works.
- Keep the existing `Date` path and primitive (Number/String/Boolean/Null) fallback.
- Retype the `Cell.value` setter in the generated `index.d.ts` from `unknown` to `CellValue | string | number | boolean | Date | null` (via `scripts/apply-glue.cjs`, which already post-processes the generated types).
- Add a JS-side test proving `cell.value = { richText: [...] }` round-trips.
- **No breaking changes**: only previously-broken behavior is fixed; widening `unknown` to a union stays assignable from every value that `unknown` accepted.

## Capabilities

### New Capabilities

- `cell-value-dispatch`: Public `Cell.value` setter routes object-shaped cell values (RichText, Hyperlink, Formula) and preserves explicit-discriminant round-trip via shape inference.

### Modified Capabilities

- `rich-text`: Clarify that the public setter accepts `cell.value = { richText: [...] }` and add a JS-API round-trip requirement (closes the spec/implementation drift where the scenario assumed it worked).

## Impact

- `src/model/cell.rs` — `set_value` (line 269) dispatch logic.
- `index.d.ts` / `native.d.ts` setter type (line 23) + `scripts/apply-glue.cjs` (declaration-merge patch).
- `__test__/` — new rich-text setter test.
- XLSX writer (`src/writer/xlsx.rs`) is unchanged (already correct).
- No new dependencies.
