## 1. Fix napi setter bypass

- [x] 1.1 Remove the incorrect comment at `src/model/worksheet.rs:272-274` claiming `#[napi(setter)]` renames the Rust symbol.
- [x] 1.2 Replace the body of `Worksheet::set_cell_style` (worksheet.rs:269) to delegate to `Cell::set_style` via `with_cell_mut`, capturing the `napi::Result` into an outer `mut result` and returning it — removing the three duplicated parse/validate branches (worksheet.rs:274-285).
- [x] 1.3 Confirm `Cell::set_style_raw` (cell.rs:415) remains `pub` and is still referenced only by reader/column paths (`insert_cell_style` at worksheet.rs:803; xlsx.rs:2578/2607) via a grep, with no remaining call from `set_cell_style`.

## 2. Extend release smoke test to read path

- [x] 2.1 In `.github/workflows/release.yml`, extend the "Functional smoke test" step to: write a workbook with a cell styled `font.bold = true` and `fill: { kind: 'solid', foreground: 'FFFF0000' }`, read it back from bytes via `wb.xlsx.read(buf)`, and assert the read-back cell reports `font.bold = true` and `fill.foreground = 'FFFF0000'` (throw on mismatch). Keep the existing writer-only assertion intact.
- [x] 2.2 Run the amended smoke test on a branch and confirm it passes (scope limited to cell-level font/fill only).

## 3. Verify and validate

- [x] 3.1 Run `cargo test test_round_trip_style_preserved` and `cargo test integration_alignment_emitted_via_set_cell_style` to confirm the delegating `set_cell_style` behaves identically.
- [x] 3.2 Run `openspec validate fix-issue-3-release-hardening` and confirm the change is apply-ready.
- [x] 3.3 Run `cargo build` and the full `cargo test` suite to confirm no regressions.
