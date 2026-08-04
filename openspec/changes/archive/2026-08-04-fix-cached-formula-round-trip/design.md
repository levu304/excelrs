# Design: Fix cached formula round-trip

See proposal.md — Why. PR #57 commit `c06c030` introduced three review findings: a
`cell_type_attr` priority mismatch, an R4-violating `cached_value()` getter, and
missing date-formula test coverage.

## Goals / Non-Goals

**Goals:**

- Align `cell_type_attr` Formula arm field-check order with the value-writing arm so the
  emitted `t` attribute always matches the `<v>` content type.
- Enforce R4: `cachedValue` returns `None` unless the cached scalar was set by
  `Worksheet::recalculate()` (the recalc path), not by the reader or JS setter.
- Add a date-formula round-trip test to cover the writer's `date_serial` `<v>` branch.

**Non-Goals:**

- No changes to `value()` Method Formula arm (returns cached scalar regardless of
  `recalc_only` — this is the authoring/round-trip contract, not the recalc-only contract).
- No changes to the reader's two-pass logic or `map_data`.
- No changes to `recalculate()` or the `formula-eval` feature.
- No `t="str"` vs shared-string (`t="s"`) distinction changes — formula string results
  remain inline (not shared strings).

## Context

### Priority mismatch

`write_cell_xml` (`src/writer/xlsx.rs`) has two arms for Formula cells:

1. `cell_type_attr` (line ~1810): emits the `t="…"` attribute.
   Current order: `boolean → error_value → string → None`.

2. Value-writing arm (line ~1861): emits `<v>…</v>`.
   Current order: `number → string → boolean → error_value → date_serial → None`.

When a Formula cell carries both `boolean` and `string` (edge case, but possible via
JS authoring), `cell_type_attr` emits `t="b"` (boolean priority) while the value arm emits
`<v>…string…</v>` (string priority). Excel's reader uses `t` to interpret `<v>`, so this
produces a type mismatch.

### R4 violation

`cached_value()` (`src/model/cell.rs:458`) is gated only on `value_type != "Formula"`.
But all three code paths that produce a Formula-typed cell with cached scalars set
`recalc_only = false` (implicitly — there was no flag):

- Reader Pass 1 (`insert_cell_value`): sets scalar fields, then Pass 2
  (`insert_cell_formula`) sets `value_type = "Formula"`.
- JS setter (`set_value`): branch-3 folds cached scalar fields onto the Formula
  CellValue.
- `set_value_raw`: direct caller in tests.

Only `set_cached_value_raw` (called by `recalculate()`) should produce a cell whose
`cachedValue` returns the scalar.

## Decisions

### 1. Add `recalc_only: bool` to `CellInner`

A single boolean flag on `CellInner` distinguishes recalc-set cached values from
authoring/reader-set ones. `recalc_only` is `true` only after `set_cached_value_raw`.

Alternative considered: separate the recalc path into a distinct `CellValue` variant
(e.g., `FormulaRecalc { formula, cached }`). Rejected: would touch the `CellValue`
enum (many match sites, serde, TS bindings), breaking the "no new variants" promise.

### 2. Reset `recalc_only = false` in all authoring paths

`Cell::new`, `set_value_raw`, and the `set_value` setter all reset `recalc_only = false`.
This ensures reader/authoring paths never leak a `true` flag.

### 3. `cell_type_attr` Formula arm: reorder to match value arm

New order: `string → boolean → error_value → None`. (`number` and `date_serial` produce
no `t` attribute, so they fall through to `None`, matching the value arm where they
emit `<v>` without a type hint.)

This is the minimal reordering: only `string` and `boolean` swap positions. The current
order is `boolean → error_value → string`; the correct alignment is
`string → boolean → error_value`.

### 4. `value()` Method Formula arm: unchanged

The `value()` getter returns the cached scalar for all Formula cells regardless of
`recalc_only`. This is intentional: `cell.value` is the round-trip surface for
authoring/Excel cached results. `cell.cachedValue` is the recalc-only accessor.

## Risks / Trade-offs

- **R1: `cached_value()` behavior change for Excel-authored formula cells.** After this
  fix, `cell.cachedValue` returns `null` for Excel-authored cells with `<f>..</f><v>..</v>`
  on disk-read. Mitigation: this is the documented R4 contract; `cell.value` still
  returns the cached scalar. Consumers relying on `cachedValue` for disk-read cells were
  relying on undocumented behavior.

- **R2: `set_value_raw` callers in tests.** The 4 test call sites in `worksheet.rs`
  (`set_value_raw` with `CellValue { … }` and `CellValue::number(…)`) are authoring
  paths — `recalc_only = false` is correct. No test assertions need to change beyond
  `test_cached_value_getter_r4`.

- **R3: `deep_clone` preserves `recalc_only`.** `CellInner` derives `Clone`, so the new
  `bool` field is cloned automatically. `deep_clone` (`cell.rs:610`) correctly preserves
  the flag through FFI boundary crossing. Verified by
  `test_cell_value_mutation_persists_through_clone`.

## Migration Plan

No migration needed — this is a bug fix + behavior alignment. No on-disk format changes.
Roll back: revert the 4 files. Tests fail fast if `recalc_only` logic regresses.

## Open Questions

None. The `formula/tests.rs` recalc tests (`test_cached_value_after_recalculate`,
`test_cached_value_null_without_recalculate`, `test_recalculate_error_caching`) already
assert the correct behavior and will pass unchanged.
