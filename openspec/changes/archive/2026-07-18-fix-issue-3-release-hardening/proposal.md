## Why

GitHub issue #3 ("v0.4.0 — Style system completion + release workflow hardening") tracks two release-workflow hardening items that are genuine defect risks, separate from the style features. This change ships only the two hardening items:

1. **Napi setter bypass** — `Worksheet::set_cell_style` (src/model/worksheet.rs:269) duplicates `Cell::set_style`'s parse+validate logic and calls the internal `set_style_raw` because of a false belief that `#[napi(setter)]` makes the Rust method unreachable. The comment (worksheet.rs:272-274) is wrong; napi-rs only wires a JS accessor and does not rename the symbol. This creates validation-drift risk: two copies of identical null→None / parse / `is_empty`→None / `validate()` logic can silently diverge.
2. **Read-path smoke test** — the `release.yml` "Functional smoke test" only exercises the writer (`setCellStyle` → `write()`). Since v0.3.0 introduced a parser, a regression that drops styles on **read** would ship silently. The release gate must round-trip a styled `.xlsx` through both write and read paths.

Targeted release: **v1.2.1** (user-specified). Note: the crate's current `Cargo.toml` version is `0.13.0` and issue #3's body references `v0.4.0`; the v1.2.1 label is taken as the intended release train for this hardening work and should be reconciled with the actual version bump at release time.

## What Changes

- **Refactor `Worksheet::set_cell_style`** to delegate to `Cell::set_style` (single source of truth) via `with_cell_mut`, capturing the `napi::Result`. Remove the duplicated parse/validate branches (worksheet.rs:274-285) and the incorrect comment at worksheet.rs:272-274.
- **Keep `Cell::set_style_raw`** (cell.rs:415) `pub` *only* for the reader/column paths that pass a pre-validated `Style` (`insert_cell_style` at worksheet.rs:803; reader call sites at xlsx.rs:2578/2607). It is no longer used by `set_cell_style`.
- **Extend the `release.yml` Functional smoke test** to write a styled workbook, read it back, and assert that cell-level `font.bold` and `fill.foreground` survive the round-trip — failing the release job on any mismatch before publish. Scope limited to cell-level font/fill only (no Hyperlink/RichText/row-level style) to avoid false failures.

No public API or observable behavior change for consumers. No dependency changes.

## Capabilities

### New Capabilities

- `release-verification`: Requirement that the release pipeline MUST verify a styled `.xlsx` round-trips through both the write and read paths (cell-level font/fill), failing the release on any style loss.

### Modified Capabilities
<!-- None. The napi setter fix is an internal refactor with no requirement-level change; `setCellStyle` already works for consumers. -->

## Impact

- **Code**: `src/model/worksheet.rs` (`set_cell_style`, comment removal), `src/model/cell.rs` (`set_style_raw` scope), `src/reader/xlsx.rs` (unchanged call sites, validated).
- **CI**: `.github/workflows/release.yml` (smoke test step extended).
- **APIs**: None changed. `setCellStyle` behaves identically.
- **Dependencies**: None.
- **Risk**: Low. Existing `cargo test test_round_trip_style_preserved` already covers the Rust round-trip; this change aligns the release gate with it.
