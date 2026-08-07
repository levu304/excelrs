## Why

The formula evaluator (#52, shipped in v2.7.0) is currently unreachable from
the public JS API: `Worksheet::recalculate()` exists but is not
`#[napi]`-annotated, no `Workbook::recalculate()` exists, and the writer/stream
never call recalculation. A host (e.g. a WOPI server ingesting formulas with
empty/stale caches) therefore cannot produce fresh computed values through the
API — even though the engine and the `Cell.cachedValue` getter ship inside the
npm binaries. The `formula-eval` spec already contracts `Worksheet.recalculate`
as public when the feature is enabled; today it is not, so the spec is
unmet. This closes that gap (issue #51, R1+R2).

## What Changes

- Expose `Worksheet.recalculate()` to JS (add `#[napi]`) — single-worksheet
  formula recalculation; caches computed scalars on cells.
- Add `Workbook.recalculate()` to JS — iterates all worksheets, recalculating
  each with full workbook context so cross-sheet references (`Sheet2!A1`)
  resolve (today they yield `#REF!` because `None` workbook context is passed).
- Update TS type declarations so both methods are visible on `Workbook` and
  `Worksheet`.
- No change to the evaluator itself; no new functions; no feature-flag changes.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `formula-eval`: adds the `Workbook.recalculate()` requirement and confirms
  `Worksheet.recalculate()` is JS-exposed with documented cross-sheet semantics.

## Impact

- `src/model/worksheet.rs` — `recalculate` gains `#[napi]`; body refactored into
  a `recalculate_with(workbook)` core.
- `src/model/workbook.rs` — new `recalculate` napi method.
- `src/formula/bridge.rs` — already accepts `Option<&WorkbookInner>` workbook
  context (no change needed).
- `index.d.ts` / generated `native.d.ts` — surface both methods.
- Built into release binaries via the existing `--features formula-eval` CI
  matrix.
