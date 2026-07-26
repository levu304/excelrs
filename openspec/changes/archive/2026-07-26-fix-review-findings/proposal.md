## Why

Four issues were identified in a post-archive code review of the `style-setter-type-safety` change. None are bugs or regressions, but they represent latent technical debt: redundant error-conversion code, untested edge-case paths, and duplicated control-flow logic across three model entities. Fixing them now prevents the patterns from spreading to future work.

## What Changes

1. **Remove redundant `map_err` on `validate()`** — 5 calls in `src/model/` wrap `s.validate()` with `.map_err(|e| napi::Error::from_reason(e.to_string()))?` when bare `?` suffices via existing `From<ExcelrsError> for napi::Error` impl.
2. **Extract shared `apply_style` helper** — The identical match-arm pattern (`None`/`Some(empty)`/`Some(valid)`) is copy-pasted across `Cell::set_style`, `Row::set_style`, `Column::set_style`, plus a variant in `Worksheet::set_columns`. Extract a `pub(crate)` helper in `style.rs` so all four sites share one normalization source of truth.
3. **Add missing edge-case tests** — Add `test_<entity>_set_style_empty_object` for Row and Column (Cell already has it). Add `test_<entity>_set_style_rejects_invalid` for Cell, Row, and Column (no entity covers the validation-error path).

No runtime behavioral changes. No API surface changes. TS type declarations unaffected.

## Capabilities

### New Capabilities

None — this change is internal code quality improvements with no observable behavior at the JS/TS API boundary.

### Modified Capabilities

None — no spec-level requirements change.

## Impact

- **Affected files**: `src/model/style.rs` (new helper), `src/model/cell.rs`, `src/model/row.rs`, `src/model/column.rs`, `src/model/worksheet.rs`
- **APIs**: No public API changes. The extracted helper is `pub(crate)` — internal only.
- **Tests**: New unit tests in Cell, Row, Column test modules (2 per module: empty-object + validation-rejection).
- **Dependencies**: None.
- **Risk**: Very low. Each change is a mechanical simplification backed by tests.
