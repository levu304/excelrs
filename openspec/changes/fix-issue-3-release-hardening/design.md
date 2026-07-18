## Context

GitHub issue #3's two release-workflow hardening items are in-scope; the seven style-system features are not. Current state:

- `Worksheet::set_cell_style` (src/model/worksheet.rs:269) re-implements `Cell::set_style`'s parse+validate across three branches, each calling the internal `Cell::set_style_raw` (cell.rs:415). A comment at worksheet.rs:272-274 claims `#[napi(setter)]` renames the Rust symbol and makes it unreachable — this is **false**; napi-rs only wires a JS accessor and leaves the Rust symbol intact.
- `.github/workflows/release.yml` "Functional smoke test" writes a styled workbook and reads nothing back. Since v0.3.0 ships a parser, a read-path style-loss regression would pass CI and ship silently.

Constraints: pure internal/CI change — no public API or observable behavior change for consumers; existing `cargo test test_round_trip_style_preserved` already proves the Rust round-trip.

## Goals / Non-Goals

**Goals**

- Make `set_cell_style` delegate to `Cell::set_style` (single source of truth), removing duplicated validation logic and the incorrect comment.
- Extend the release smoke test to round-trip a styled `.xlsx` through both write and read paths.

**Non-Goals**

- Not implementing the seven style-system features (mergeCells, `Arc<Mutex<Cell>>`, Hyperlink/RichText, theme colors, row-level style, gradient fills, diagonal borders).
- Not changing `setCellStyle` behavior from the consumer's perspective.
- Not asserting Hyperlink/RichText/row-level style in the smoke test (out of this change's scope — would risk false failures).

## Decisions

- **Delegate via `with_cell_mut`, capturing the `napi::Result`.** `with_cell_mut` (worksheet.rs:774) closes over `FnOnce(&mut Cell) -> ()`; we assign the result into an outer `mut result` and return it. *Why:* single source of truth eliminates the validation-drift risk (two copies of null→None / parse / `is_empty`→None / `validate()` at cell.rs:387-398 vs worksheet.rs:275-285). *Alternative considered:* keep the duplication but manually keep it in sync — rejected; sync-by-hand is exactly the failure mode we're removing.
- **Keep `Cell::set_style_raw` `pub`, scoped to reader/column paths.** It is genuinely needed by `insert_cell_style` (worksheet.rs:803) and reader call sites (xlsx.rs:2578/2607) that pass a pre-validated `Style`. Making it private would force re-validation there. *Why:* those paths bypass parse/validate deliberately and correctly. It is simply no longer called by `set_cell_style`.
- **Smoke-test scope = cell-level `font.bold` + `fill.foreground` only.** *Why:* these are proven to round-trip by `test_round_trip_style_preserved`; broader scope (Hyperlink/RichText/row style) is out of this change and would cause false CI failures.

## Risks / Trade-offs

- **[Risk]** If `with_cell_mut` swallows napi errors, the captured result could mask a failure. → *Mitigation:* confirm `with_cell_mut` propagates the closure's result before merging; covered by `test_round_trip_style_preserved`.
- **[Risk]** The read path may serialize `fill.foreground` in a different string form than written, causing a false smoke-test failure. → *Mitigation:* limit assertions to `font.bold` (bool) and `fill.foreground` (already proven round-trip by the existing Rust test); run the amended smoke test on a branch before relying on it in release.
- **[Risk]** Version label mismatch — crate is `0.13.0`, issue body says `v0.4.0`, this change targets `v1.2.1`. → *Mitigation:* noted in proposal; reconcile the actual version bump at release time (no code impact).

## Migration Plan

Internal refactor + CI step addition only. No consumer migration, no data-format change. Rollback is a plain `git revert` of the two files. Ship as part of the v1.2.1 release train.

## Open Questions

- What is the authoritative next version to bump (`0.13.0` → ?), given the three conflicting labels? Resolve at release time.
