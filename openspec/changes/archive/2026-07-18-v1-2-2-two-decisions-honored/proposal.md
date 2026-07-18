## Why

Issue #3 tracks 7 style-system features plus 2 release-hardening items. The 2 hardening items shipped in v1.2.1. Of the 7 features, five have since landed in later releases (diagonal borders, gradient fills, hyperlinks, rich text, theme color references, and cell interior mutability are all present as shipped `openspec/specs/`). Two genuine read-path gaps remain, and the two release decisions made while triaging #3 were never written down — so they can silently regress.

1. **Merge-cells read path** — the writer has emitted `<mergeCells>` since v0.5.0, but the reader never parses it, there is no `Merge` value representation, and non-master cells are not handled. A merged workbook loses its merges on round-trip.
2. **Row-level style read path** — `Row.style: Arc<Mutex<Option<Style>>>` exists and the writer emits `<row s="N">`, but the reader never restores the row style from the `<row s="…">` attribute. Row styling is dropped on read.

This change closes both remaining code gaps and records the two decisions (link partial fixes with `Refs`, not `Closes`; the real `Fill` field is `fill.foreground`, not `fill.color`) as a repo policy doc so they are honored going forward. Once both gaps ship, issue #3 is fully resolved and may be closed.

## What Changes

- **Merge-cells read path**: parse `<mergeCells>` in the reader and populate the worksheet's merge range collection; handle non-master cells (no phantom values); add a minimal `Merge` value representation for the anchor cell for ExcelJS parity.
- **Row-level style read path**: parse the `<row s="N">` attribute in the reader and set `Row.style` (mirrors the existing writer behavior).
- **Release-process policy doc** (`CONTRIBUTING.md`): document the two decisions — (a) use `Refs #N` (keep open) for a partial fix to a multi-item issue and `Closes #N` only when fully resolved; (b) solid fills use `fill: { kind: "solid", foreground: "…" }` — there is no `fill.color` field on this library's `Fill` type.
- **Smoke-test hardening** (small): extend the `release.yml` functional smoke test added in v1.2.1 to also round-trip a merged range and a row-level style through the read path, so silent loss of either is caught before publish.

No breaking API changes — all additions are read-side and additive.

## Capabilities

### New Capabilities

- `merge-cells`: read-side parsing of merged cell ranges (`<mergeCells>`), anchor/`Merge` value representation, and non-master cell handling so merges survive a write→read round-trip.
- `row-style`: read-side restoration of row-level style from the `<row s="N">` attribute into `Row.style`.

### Modified Capabilities

- `release-verification`: extend the release smoke test to also assert a merged range and a row-level style round-trip through the read path (builds on the v1.2.1 font/fill smoke test).

## Impact

- **Reader**: `src/reader/xlsx.rs` — new `<mergeCells>` parsing and `<row s="…">` attribute handling.
- **Model**: cell value model (new `Merge` representation for the anchor) and `Row` read population.
- **Docs**: new `CONTRIBUTING.md` at repo root capturing the two release decisions.
- **CI**: `release.yml` functional smoke test extended (additional assertions only).
- **No dependencies added.** No public API removed or changed in a breaking way.
