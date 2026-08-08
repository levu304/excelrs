## Why

The reader aligns ~20 per-sheet parsers by `xl/worksheets/sheet{i+1}.xml` **file index**, implicitly assuming file order equals workbook display order. That assumption is false for hand-reordered workbooks, where the `<sheet>` display order does not match the `sheetN.xml` numbering. For those inputs, per-sheet data such as `tabColor`/defaults silently attaches to the wrong worksheet. The writer renumbers on output so round-trips self-heal, but the read path is wrong for third-party inputs - a correct reader must not misattribute data between sheets.

## What Changes

- Add a single resolver that reads `xl/workbook.xml.rels` (`rId → target`) and the `<sheet r:id>` order in `xl/workbook.xml`, producing the real worksheet file path for each sheet in display order.
- Thread the resolved path list through the ~20 bulk per-sheet parsers (the shared cell-style parser self-resolves to keep its public signature compatible with the streaming reader), replacing the `format!("xl/worksheets/sheet{}.xml", i + 1)` positional indexing.
- Remove the now-obsolete file-index assumption and its `ponytail:` deferral note.
- Add a regression test that builds a reordered `<sheet>`/`.rels` workbook and asserts each sheet's metadata lands on the correct worksheet.

No public API change. No new dependencies (reuses the existing `parse_sheet_rels` rels-parsing pattern).

## Capabilities

### New Capabilities

- `sheet-path-alignment`: reader resolves each worksheet to its actual file via the workbook relationship graph (rId→target) instead of positional `sheetN.xml` indexing.

### Modified Capabilities
<!-- none -->

## Impact

- **Code**: `src/reader/xlsx.rs` — new resolver function + ~20 bulk call-site swaps from file-index to resolved path; `src/reader/styles.rs` — shared cell-style parser self-resolves via the same resolver.
- **Behavior**: reading reordered third-party xlsx now attributes metadata to the correct sheet.
- **Risk**: mechanical but touches many call sites; covered by a new regression test plus the existing round-trip suite.
