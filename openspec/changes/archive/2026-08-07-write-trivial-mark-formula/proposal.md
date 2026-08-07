## Why

The `cleanup-value-type-from-tag` change centralized every discriminant **read** through `CellType::from_tag`, but it left one **write** site assigning the discriminant tag directly as a raw string literal: `worksheet.rs` `insert_cell_formula` does `cv.value_type = "Formula".to_string()`. It had to, because `CellValue` has no API that marks an *existing* value as a formula while preserving its other cached fields — `CellValue::formula()` returns a fresh value and would discard the Pass-1 cached scalar (number/string/etc.). So the single un-centralized discriminant mutation stayed, explained only by a comment. This change closes that gap with a tiny, type-safe builder.

## What Changes

- Add `CellValue::mark_formula(formula)` — a builder that sets `value_type = "Formula"` and `formula`, returning `self` so all other cached fields are preserved.
- Replace the raw `cv.value_type = "Formula".to_string()` mutation in `insert_cell_formula` with `cv = cv.mark_formula(formula)`.

No public API, ABI, or behavior change. The written tag string is identical (`"Formula"`), and both `value_type` and `formula` fields are set exactly as before.

## Capabilities

### New Capabilities

*(none — pure refactor, no behavior change)*

### Modified Capabilities

*(none — `skip_specs: true` declared; no requirement changes)*

## Impact

- `src/model/cell.rs`: new `mark_formula` method on `impl CellValue`.
- `src/model/worksheet.rs`: `insert_cell_formula` call-site update only.
- No dependency, public API, or feature-flag (`formula-eval`) behavior change.
