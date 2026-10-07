# Tasks

## 1. Theme-aware style dedup

- [x] 1.1 Make font/fill/border dedup keys theme-aware per D1 (serde shape untouched) and add unit tests pinning a themed/plain twin pair per sub-table type plus byte-stability of twin-free outputs, verifying `cargo test styles` passes
- [x] 1.2 Add a mixed-twin round-trip test (themed + plain same-ARGB cells through write and re-read/ExcelJS load) proving each keeps its emission form, verifying the new test passes and `cargo test --lib` stays green

## 2. Clamp `<col>` lower bound

- [x] 2.1 Apply `lo.max(1)` in both `parse_col_dims_from_xml` and `parse_col_outline_levels_from_xml` with unit tests for `min="0"` dims/outline ignored and valid `min="1"` still parsed, verifying `cargo test --lib col` passes
- [x] 2.2 Add a round-trip test proving a `min="0"` descriptor creates no column 0 and emits no `min="0"`, verifying the new test passes alongside the existing `dimension-properties` writer tests

## 3. Integration and records

- [x] 3.1 Run `openspec validate --strict --specs --changes`, the full Rust suite (`cargo test` plus clippy), and the vitest suite, verifying all three pass
- [x] 3.2 Record the fixes in `CHANGELOG.md` (twin-dedup split, `min="0"` ignored) and update the `design.md` risk note from accepted to fixed, verifying entries match the shipped delta specs
