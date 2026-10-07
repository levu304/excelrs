# Tasks

## 1. Column live handle (Rust model)

- [x] 1.1 Give `Column` shared-ownership mutation semantics matching the `Row` precedent and add worksheet `getColumnByNum` / `getColumnByLetter` accessors with auto-create, verifying `cargo test column` passes including new live-mutation tests (handle write visible through fresh lookup)
- [x] 1.2 Cover `setColumns`-replace detach and sparse `colNum` lookup in Rust tests, verifying the new tests pass and existing `dimension-properties` writer tests still pass unchanged

## 2. getColumn FFI glue and types

- [x] 2.1 Add the `getColumn` overload to hand-maintained `index.js` glue plus `index.d.ts` / generated `native.d.ts` declarations, verifying `pnpm verify:build` passes
- [x] 2.2 Add vitest coverage for number/letter resolution, auto-create, width/hidden/style persistence through ExcelJS read-back, verifying the new `__test__/column-access.test.ts` passes

## 3. Theme-wins emission and theme1.xml passthrough

- [x] 3.1 Retain raw `theme1.xml` bytes on read and emit them verbatim with the content-type entry on write, verifying a custom-theme fixture round-trips byte-identical through a new Rust test
- [x] 3.2 Flip `emit_color_attrs` to theme-wins and update the pinned rgb-wins unit tests to the new policy, verifying `cargo test styles` passes
- [x] 3.3 Add vitest round-trips (custom-theme file keeps resolving to custom ARGB; rgb-only file output unchanged), verifying the new tests pass and ExcelJS opens the round-tripped themed file without repair

## 4. Array-formula behavior pin (tests only, no src changes)

- [x] 4.1 Commit an ExcelJS-authored `<f t="array">` fixture and add a round-trip test asserting plain-formula read (text plus cached value per cell) and plain `<f>` write with no `t`/`ref`, verifying the new test passes

## 5. Integration and release record

- [x] 5.1 Run `openspec validate --strict --specs --changes`, the full Rust suite (`cargo test`), and the vitest suite, verifying all three pass
- [x] 5.2 Record the change in `CHANGELOG.md` (getColumn additive API, theme passthrough policy, array-formula documented behavior) and ROADMAP triage rows if applicable, verifying the entries match the shipped specs
