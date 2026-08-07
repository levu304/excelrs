## Context

`Worksheet::recalculate()` (worksheet.rs:181) is `#[cfg(feature = "formula-eval")]`
but lacks `#[napi]`; its only callers are tests. `Workbook::recalculate()` does
not exist. The writer/stream never recalculate. The evaluator
(`FormulaEvaluator::new(worksheet, name, workbook)`) already accepts
`Option<&WorkbookInner>` workbook context — `Worksheet::recalculate` passes
`None`, so cross-sheet refs → `#REF!`. `Workbook` wraps
`Arc<Mutex<WorkbookInner>>` and exposes `worksheets()`. See proposal.md — Why.

## Goals / Non-Goals

Goals: make recalc reachable from JS; enable cross-sheet via workbook-scoped
recalc.

Non-Goals: no new functions, no lazy/eager/incremental dirty-graph, no
lookups (INDEX / MATCH / VLOOKUP / XLOOKUP), no auto-recalc on write. Those
are separate #51 follow-ups.

## Decisions

1. **Split recalc core from the trigger.** Extract the body of
   `Worksheet::recalculate()` into
   `fn recalculate_with(&self, workbook: Option<&WorkbookInner>)`. The public
   `recalculate()` becomes a thin `#[napi]` wrapper calling
   `recalculate_with(None)`.
2. **Add `Workbook::recalculate()` (`#[napi]`):** lock `inner`, snapshot
   `worksheets`, iterate, calling `recalculate_with(Some(&inner))` on each.
   `&inner` (via `MutexGuard` deref) and `&ws` both outlive the per-sheet eval,
   satisfying the evaluator's shared `'ws` lifetime.
3. **TS types:** napi-rs regenerates `native.d.ts` from `#[napi]`; mirror the
   additions into `index.d.ts` (hand-maintained facade) so both `Workbook` and
   `Worksheet` expose `recalculate(): void`.
4. **Keep `Worksheet.recalculate()` single-sheet (passes `None`) deliberately:**
   a worksheet has no back-reference to its workbook, so cross-sheet from the
   worksheet scope is inherently `#REF!`. `Workbook.recalculate()` is the
   cross-sheet-capable entry point.

## Risks / Trade-offs

- [Lifetime / borrow friction] evaluator's `worksheet` and `workbook` share
  lifetime `'ws`. → Mitigation: snapshot worksheets, hold the lock guard across
  the loop so `&inner` and `&ws` share the loop-body lifetime.
- [Deadlock if evaluator re-locks workbook] → Mitigation: pass
  `&WorkbookInner` (not a locked `Workbook`); the evaluator only reads
  `worksheets`, never re-locks the workbook mutex.
- [Behavior change for `Worksheet.recalculate` consumers] → Mitigation:
  previously internal/unreachable; making it public is additive.
- [Cross-sheet still `#REF!` from worksheet scope] documented limitation;
  `Workbook.recalculate()` is the intended cross-sheet path.

## Migration Plan

No data/migration. Additive API. Rollback: revert the change.

## Open Questions

- Should `Worksheet.recalculate()` accept an optional workbook reference to
  enable cross-sheet from the worksheet scope? Deferred — `Workbook.recalculate()`
  covers the need; revisit only if a worksheet-only cross-sheet call is wanted.
