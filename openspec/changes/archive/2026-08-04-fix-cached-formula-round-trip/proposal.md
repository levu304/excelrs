# Proposal: Fix cached formula round-trip — priority mismatch, R4 enforcement, date coverage

## Why

PR #57 ("fix: authorable cached formula results round-trip", commit `c06c030`) added authored-and-Excel cached-scalar round-tripping for formula cells, but three issues were found in review: (1) the `cell_type_attr` Formula arm checks scalar fields in a different order than the value-writing arm, so a cell carrying both `boolean` and `string` emits `t="b"` while `<v>` carries the string; (2) the `cachedValue` getter documents R4 ("recalc-only") but returns cached scalars for ALL Formula cells — including Excel-authored and JS-authored ones — because it has no way to distinguish recalc-set values from reader-set values; (3) there is no JS-authored date-formula round-trip test, leaving the `date_serial` `<v>` branch of the writer untested.

## Changes

- **`cell_type_attr` Formula arm (`src/writer/xlsx.rs`)**: reorder the field checks (`string → boolean → error_value`) to match the value-writing arm's priority (`number → string → boolean → error_value → date_serial`). `number` and `date_serial` produce no `t` attribute (fall through to `None`), matching the value-writing arm. **BREAKING**: no — fixes a bug where the emitted `t` attribute could disagree with the `<v>` content.
- **`CellInner` struct (`src/model/cell.rs`)**: add a `recalc_only: bool` field (default `false`). Only `set_cached_value_raw` (the recalc path) sets it to `true`. `cached_value()` returns `None` unless `recalc_only` is `true`, enforcing R4.
- **`set_value_raw` / `set_value` Setter / `Cell::new`**: all reset `recalc_only = false` — reader, JS setter, and direct callers are authoring/reading paths, not recalc.
- **`set_cached_value_raw`**: sets `recalc_only = true`.
- **Test `test_cached_value_getter_r4` (`src/model/cell.rs`)**: update the Formula-with-cached-scalar case to assert `None` (since `set_value_raw` sets `recalc_only = false`), and add a comment clarifying that the recalc-return path is covered by `formula/tests.rs`.
- **JS test (`__test__/cached-formula.test.ts`)**: add `JS-authored cached date formula round-trips as bare number` — `{ formula: "DATE(2025,1,1)", dateSerial: 45657 }` round-trips to `cell.value === 45657`.

## Capabilities

### Modified Capabilities

- **`cached-formula-value`**: spec §R4 (`Cell.cachedValue` getter semantics are unchanged — recalc-only) was declared a Non-Requirement in the prior change. PR #57's `cached_value()` getter actually violates this — it returns cached scalars for all Formula cells, not just recalc'd ones. This change enforces R4 by adding the `recalc_only` flag. The getter's public contract (returns `Option<CellValue>` with the same `tstype`) is unchanged; only the *when* narrows to match the documented R4 guarantee. Also adds the missing `date_serial` round-trip test scenario (spec §"date cached value round-trips") and fixes the writer `t` attribute priority to match the value-emission priority.

## Impact

- **`src/writer/xlsx.rs`**: `cell_type_attr` Formula arm reorder (low blast radius — local to `write_cell_xml`).
- **`src/model/cell.rs`**: `CellInner` struct + `Cell::new` + `set_value_raw` + `set_value` Setter + `set_cached_value_raw` + `cached_value()` getter + `test_cached_value_getter_r4` test. `cached_value()` has 4 caller sites in `formula/tests.rs` — all remain compatible (tests assert `None` without recalc, `Some` after recalc).
- **`__test__/cached-formula.test.ts`**: one new test added.
- No public API signature changes. No breaking changes to the JS surface. The `cachedValue` getter's *documented* semantics are restored, not changed.
