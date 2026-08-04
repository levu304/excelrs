# Tasks: Fix cached formula round-trip

Implementation breakdown for priority mismatch, R4 enforcement, and date test coverage.
See `spec.md` for requirements; `design.md` for rationale.

## 1. Fix cell_type_attr priority mismatch (writer)

- [x] 1.1 In `src/writer/xlsx.rs` `cell_type_attr` Formula arm (~L1810), reorder checks from
      `boolean → error_value → string` to `string → boolean → error_value` to match the
      value-writing arm's priority (`number → string → boolean → error_value → date_serial`).
      `number` and `date_serial` fall through to `None` (no `t` attribute), matching the
      value arm.

- [x] 1.2 Verify the value-writing arm's `"Formula"` branch (~L1855) already emits `<v>`
      for all five scalar fields in the correct priority order. No change expected — confirm
      by reading.

## 2. Enforce R4: add recalc_only flag (cell model)

- [x] 2.1 In `src/model/cell.rs`, add `pub recalc_only: bool` field to `CellInner` struct
      (~L223).

- [x] 2.2 In `Cell::new` (~L245), add `recalc_only: false` to the `CellInner` constructor.

- [x] 2.3 In `set_value_raw` (~L596), change to lock mutably and set `inner.recalc_only = false`
      after `inner.value = value`.

- [x] 2.4 In `set_value` Setter (`~L360` Path 1 Date, `~L445` Path 2 JSON), add
      `inner.recalc_only = false;` in both paths.

- [x] 2.5 In `set_cached_value_raw` (~L627), add `inner.recalc_only = true;` after setting
      cached scalar fields.

- [x] 2.6 In `cached_value()` (~L458), change the guard from
      `if cv.value_type != "Formula" { return None; }` to
      `if cv.value_type != "Formula" || !inner.recalc_only { return None; }`.

## 3. Update Rust test for R4 (cell.rs)

- [x] 3.1 Update `test_cached_value_getter_r4` (~L944): the "Formula cell with cached scalar
      returns it" case must now assert `cached_value().is_none()` (because `set_value_raw`
      sets `recalc_only = false`). The recalc-return path is covered by
      `formula/tests.rs`. Add a comment explaining the split.

## 4. Add JS date-formula round-trip test

- [x] 4.1 In `__test__/cached-formula.test.ts`, add test
      `JS-authored cached date formula round-trips as bare number` after the error formula
      test (~L144): assign `{ formula: "DATE(2025,1,1)", dateSerial: 45657 }`, round-trip,
      assert `cell.value === 45657` (bare number, not JS Date).

## 5. Verify

- [x] 5.1 `cargo build` (default features) compiles.
- [x] 5.2 `cargo build --features formula-eval` compiles.
- [x] 5.3 `cargo test --lib` passes (443 Rust tests, incl. `formula/tests.rs` recalc tests).
- [x] 5.4 `cargo clippy` clean.
- [x] 5.5 `npx vitest run __test__/cached-formula.test.ts` passes (14 tests incl. new date test).
- [x] 5.6 `git diff --stat` shows changes scoped to `src/writer/xlsx.rs`, `src/model/cell.rs`,
      `__test__/cached-formula.test.ts` only.
