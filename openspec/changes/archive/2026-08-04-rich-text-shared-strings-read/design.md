## Context

The reader resolves shared strings via calamine (see `src/reader/xlsx.rs` doc comment: "Shared strings are resolved automatically by calamine — the reader never sees shared string indices"). Calamine 0.35's `Data` enum has **no `RichText` variant** — a shared string containing `<si><r><rPr>…</rPr><t>…</t></r></si>` collapses to a bare `Data::String(plain_text)`. All per-run `Font` data is irretrievably lost before excelrs sees it.

The existing inline-string rich-text parser (`parse_inline_str_rich_text_with`, added in v0.12.0) already maps `<rPr>` → `Font` and `<r>` → `RichTextRun` correctly. It only scans `<c t="inlineStr"><is><r>…</is></c>` in worksheet XML — shared-strings rich text in `xl/sharedStrings.xml` is never visited.

Reader flow at `workbook_inner_from_bytes`:

1. calamine cell walk (`workbook_to_inner_model`) — shared-string cells become `CellValue::string` (fonts already gone)
2. Step 3.10: overlay inline-string rich text by scanning each `sheetN.xml` for `t="inlineStr"`
3. (gap) No shared-strings rich-text overlay exists

## Goals / Non-Goals

- **Goals:** Read rich-text shared strings from `xl/sharedStrings.xml`; resolve `<c t="s"><v>idx</v></c>` cells whose shared-string index contains rich-text runs into `CellValue::rich_text`; preserve per-run `Font`.
- **Non-Goals:** Change the `StreamValue`/`JsStreamValue` streaming path (Gap 1, separate change). Change the in-memory writer. Parse formula-cached rich text. Support `Data::Richtext` if/when calamine adds it (N/A for 0.35).

## Decisions

### D1. Parse shared-strings rich text in a pre-pass, separate from calamine

New function `parse_shared_string_rich_text(data: &[u8]) -> Result<HashMap<u32, Vec<RichTextRun>>, ExcelrsError>` in `src/reader/xlsx.rs`. Opens the zip, reads `xl/sharedStrings.xml`, iterates `<si>` elements. For each `<si>` containing `<r>` children (rich text), parse runs using the **same `<rPr>` → `Font` logic already inlined in `parse_inline_str_rich_text_with`** — extract once into a shared `parse_run_rpr` helper to avoid duplication. Return `HashMap<index → runs>`; plain `<si><t>…</t></si>` entries are omitted (index absent ⇒ not rich text).

Reuse `Font` and `RichTextRun` types unchanged.

### D2. Overlay on cells by scanning worksheet XML for `<c t="s">` references

Following the Step 3.10 pattern, add Step 3.10b: `overlay_shared_string_rich_text(data, sheet_count, &rich_strings, &mut inner)`. For each worksheet, scan `sheetN.xml` for `<c t="s"><v>idx</v></c>` cells, resolve `idx` against the rich-strings map, and if present call `insert_cell_value(row, col, CellValue::rich_text(runs))`.

This mirrors the inline-string overlay exactly — same `ref_to_rowcol` cell-reference resolver, same `insert_cell_value` merge. Calamine's `Data::String` is already in the cell; `insert_cell_value` overwrites it with the RichText CellValue.

### D3. Placement: run after Step 3.10 (inline overlay), before hyperlink parsing

Inline strings and shared-strings rich text are independent sources. Running shared-strings overlay after the inline overlay is safe — a cell is either inline OR shared, never both. Placing it before hyperlink parsing avoids any interference.

### D4. Shared rPr parsing extracted to avoid drift

Factor the `<rPr>` → `Font` mapping currently duplicated in `parse_inline_str_rich_text_with` into a standalone `parse_rpr_font(reader, e) -> Font` (or equivalent event-driven helper) so both inline and shared-strings paths use identical logic.

## Risks / Trade-offs

- **XML scan cost** — scanning each worksheet XML twice (once for inline, once for shared-strings references). Same cost as the existing inline-string scan; additive. Acceptable for correctness parity with Excel-generated files.
- **Calamine flatten-first** — we cannot recover rich text through calamine; the pre-pass must read the raw sharedStrings XML ourselves, same as the existing inline-string pre-pass reads raw worksheet XML.
- **Empty `<t>` runs** — a `<si><r><rPr>…</rPr><t></t></r></si>` with only formatting and empty text. The existing inline parser skips runs with empty text (`if !current_text.is_empty()`). Shared-strings parser will follow the same guard for consistency — but this means a formatting-only run with empty text won't round-trip. Edge case; document as known limitation matching existing behavior.
- **Rich text index lookup** — the `t` attribute on `<c>` can be `"s"` (shared string) or absent. Only `t="s"` cells are candidates. Non-shared rich text is inline (handled by existing path).
