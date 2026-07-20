## 1. Implement the fix

- [x] 1.1 Remove the `Ref::Cell` guard in `offset_ref_token` (src/stream.rs:768) so `Ref::Column` / `Ref::Row` tokens shift via the same `offset` → `format` path as `Cell`; the existing `validate()` column bound keeps function names verbatim.
- [x] 1.2 Add unit tests `replace_cell_names_shifts_bare_column` (`=A+B` with column offset (0,1) → `=B+C`) and `replace_cell_names_shifts_bare_row` (`=A1*5` with row offset (1,0) → `=A2*6`) to `src/stream.rs`.

## 2. Verify

- [x] 2.1 Run `cargo test --lib` and `cargo clippy --lib`; all tests pass and clippy is clean.
- [x] 2.2 Confirm the resolved formula for a bare-ref shared-formula member matches the whole-workbook (calamine) reader (covered at unit level by 1.2; no streaming round-trip fixture required).

## 3. Land

- [ ] 3.1 Stack the implementation branch on `feat/streaming-shared-formula-resolution` (PR #35) so the `offset_ref_token` history composes, push, and update the review comment discussion_r3613652786 to note the fix.
