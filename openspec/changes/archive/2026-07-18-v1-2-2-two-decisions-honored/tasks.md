## 1. Merge-cells read path

- [ ] 1.1 Confirm where `Worksheet::merge_cells` persists merges (field name) and where the writer reads them from; add the collection if it is missing.
- [ ] 1.2 Parse `<mergeCells>` in `src/reader/xlsx.rs` and populate the worksheet merge collection.
- [ ] 1.3 Handle non-master cells on read: the anchor keeps its value, other cells in the range carry no phantom value. Add a round-trip test (`mergeCells` → write → read → same range + anchor value).
- [ ] 1.4 (Parity, optional) Add a minimal `CellValue::Merge(range)` variant on the anchor cell; keep the worksheet-level collection as the serialization source of truth.

## 2. Row-level style read path

- [ ] 2.1 Add `insert_row_style` in the reader that resolves the `<row s="N">` index via `style_table.resolve_style` and sets `Row.style` (mirror `insert_cell_style`).
- [ ] 2.2 Add a round-trip test asserting `Row.style` survives a write → read cycle.

## 3. Release-process policy doc

- [ ] 3.1 Create `CONTRIBUTING.md` at repo root documenting the two release decisions: (a) link partial fixes to a multi-item issue with `Refs #N` (keep open) and use `Closes #N` only when fully resolved; (b) solid fills use `fill: { kind: "solid", foreground: "…" }` — there is no `fill.color` field on this library's `Fill` type.

## 4. Smoke-test hardening

- [ ] 4.1 Extend the `release.yml` functional smoke test (added in v1.2.1) to also write a merged range and a styled row, read the workbook back, and assert both survive the read path; the release job must fail if either assertion is false.

## 5. Release

- [ ] 5.1 Bump `package.json` to `1.2.2`, add a `CHANGELOG.md` entry under `### Fixed`, and tag `v1.2.2` (same tag-driven flow as v1.2.1). Close issue #3 once all features ship.
