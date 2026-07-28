## 1. Write failing test (TDD step 1)

- [x] 1.1 Add test in `src/writer/styles.rs` that asserts the default fill (index 0) emits `<patternFill patternType="none"/>` — currently emits `"solid"`, so test must fail
- [x] 1.2 Run `cargo test -- emit_minimal` to verify the new assertion fails with current `"solid"` emission

## 2. Apply fix (TDD step 2)

- [x] 2.1 In `src/writer/styles.rs` line 538, change `unwrap_or("solid")` to `unwrap_or("none")` in the no-fg-no-bg branch of `emit_fills`
- [x] 2.2 Run `cargo test` — the failing test from 1.1 now passes; confirm no regressions

## 3. Add Rust-level round-trip test (write → read fills)

- [x] 3.1 Add `test_round_trip_no_fill` in `src/writer/xlsx.rs` alongside existing `test_round_trip_style_preserved`: create cells with border-only (`border: { bottom: { style: "thick" } }`), write, read back, assert `cell.style().unwrap().fill` is `None`
- [x] 3.2 Add `test_round_trip_mixed_fill` in same file: one cell with explicit fill + one cell with border-only, read back, assert the border-only cell has `fill: None` while the other preserves its fill
- [x] 3.3 Run `cargo test "test_round_trip_"` — both pass

## 4. Add JS-level round-trip tests via exceljs

- [x] 4.1 Add test in `__test__/style.test.ts` (Group B): write workbook with border-only cells via excelrs, read back with exceljs, assert fills have no foreground
- [x] 4.2 Add test that writes mixed cells (some with fill, some border-only), read back with exceljs, only explicitly-filled cells have foreground color
- [x] 4.3 Add test that writes font-only and alignment-only styles (no fill), read back with exceljs, verify no accidental fill foreground
- [x] 4.4 Run `pnpm test` to verify all JS tests pass

## 5. Verify

- [x] 5.1 Run `cargo clippy -- -D warnings` — no new lint issues
- [x] 5.2 Run `cargo test` full suite — all tests pass
- [x] 5.3 Run `pnpm test` full JS integration suite — all tests pass
