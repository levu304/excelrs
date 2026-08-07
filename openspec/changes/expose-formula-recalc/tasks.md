## 1. Refactor `Worksheet` recalc core

- [x] Extract the body of `Worksheet::recalculate()` (worksheet.rs:181) into
      `fn recalculate_with(&self, workbook: Option<&WorkbookInner>)`.
- [x] In that core, replace
      `FormulaEvaluator::new(self, self.name.clone(), None)` with
      `FormulaEvaluator::new(self, self.name.clone(), workbook)`.
- [x] Keep `pub fn recalculate(&self)` (no workbook arg) as a thin wrapper
      calling `recalculate_with(None)`.

## 2. Expose `Worksheet.recalculate` to JS

- [x] Add `#[napi]` to `Worksheet::recalculate()` so it appears on the JS
      `Worksheet` class (currently internal-only).

## 3. Add `Workbook.recalculate`

- [x] In `workbook.rs`, add `#[napi] pub fn recalculate(&self)` that locks
      `inner`, snapshots `worksheets`, and calls
      `ws.recalculate_with(Some(&inner))` for each sheet.
- [x] Ensure the `MutexGuard` (and therefore `&inner`) and each `&ws` outlive
      the per-sheet evaluation loop (shared `'ws` lifetime).

## 4. TS type declarations

- [x] Confirm `napi build --features formula-eval` regenerates `native.d.ts`
      with `recalculate()` on `Workbook` and `Worksheet`.
- [x] Add `recalculate(): void` to `Workbook` and `Worksheet` in `index.d.ts`.

## 5. Verify

- [x] `cargo test --features formula-eval` passes (existing recalc tests still
      green).
- [x] `napi build --features formula-eval` succeeds; `index.d.ts` updated.
- [x] Add a JS/TS workbook: `Sheet1!B1 = =Sheet2!A1`, `Sheet2!A1 = 42`
      → after `workbook.recalculate()`, `cell.cachedValue === 42`.
- [x] Add a JS/TS test: `worksheet.recalculate()` populates single-sheet cached
      values; a cross-sheet ref caches `#REF!` and the recalc still completes.
