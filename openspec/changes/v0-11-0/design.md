## Context

`excelrs` is a Rust/Node ExcelJS drop-in replacement. ROADMAP.md (the parity matrix) still marks four worksheet features `planned`: hyperlinks (read), auto filters, freeze/split panes, and sheet protection. Each is a single OOXML element on the worksheet part (`xl/worksheets/sheetN.xml`) — all `low` effort, `high` compat value. Hyperlink *write* already ships (v0.5.0); this change adds the reader half so the round-trip closes.

Existing reference patterns:

- **Reader** (`src/reader/xlsx.rs`) already parses `<dataValidations>` by reading the sheet XML directly from the zip archive, because calamine exposes none of these elements. The same direct-from-archive strategy applies to all four new elements.
- **Writer** (`src/writer/xlsx.rs`) already emits `<hyperlinks>` + a `xl/worksheets/_rels/sheetN.xml.rels` part via `collect_sheet_hyperlinks` / `write_sheet_rels`.
- **Model** (`src/model/worksheet.rs`) holds shared state in `Arc<Mutex<>>` clones; `DataValidation` is the established template for a sheet-scoped collection. `CellValue` already has `hyperlink` / `hyperlink_text` fields (used by the writer).

## Goals / Non-Goals

**Goals:**

- Read & write worksheet auto-filter (`autoFilter` attribute).
- Read & write freeze/split pane state (`<sheetViews><sheetView><pane>`).
- Read & write sheet protection flags (`<sheetProtection>`).
- Read worksheet hyperlinks from `<hyperlinks>` + relationships, producing the same `{text, hyperlink}` `CellValue` shape the writer emits.

**Non-Goals:**

- Workbook-level protection / `workbookProtection` (`workbookPr` password) — out of scope.
- Pane scroll state beyond the standard `xSplit`/`ySplit`/`topLeftCell`/`activePane`/`state` attributes.
- Auto-filter *column definitions* (`<filterColumn>`, `<autoFilter><filterColumn>`) — only the range attribute is targeted (matches ExcelJS `ws.autoFilter = "A1:C1"`).
- Selective granular protection options beyond the boolean flags ExcelJS exposes via `ws.protection`.

## Decisions

### D1. Parse new elements directly from the zip archive (match data-validation)

**Why:** calamine does not expose `autoFilter`, `sheetViews`, `sheetProtection`, or `hyperlinks`. Reuse `parse_sheet_data_validations`'s "open `xl/worksheets/sheet{N}.xml` from the archive, parse with quick-xml, attach by sheet index" pattern for each new element.
**Alternative considered:** extend calamine — rejected (upstream dependency, overkill for 4 attributes).

### D2. Reuse the `Arc<Mutex<>>` shared-state model pattern for new collections

**Why:** `Worksheet` clones (via `clone_worksheet` / `Arc`) must observe the same autoFilter/views/protection. Mirror `dataValidations` exactly: store `Arc<Mutex<...>>` inside the inner worksheet and expose getters/setters on the `napi` `Worksheet` wrapper.
**Alternative considered:** per-clone copies — rejected (would desync clones, breaking the existing contract).

### D3. Hyperlink read resolves the relationship part

**Why:** `<hyperlink r:id="rIdN">` only carries a relationship id; the actual URL lives in `xl/worksheets/_rels/sheetN.xml.rels`. The reader must map `rIdN → Target` and set `CellValue { value_type: "Hyperlink", hyperlink: <url>, hyperlink_text: <cell's existing string> }` at the `ref` cell, reusing the writer's existing `CellValue` hyperlink shape for a clean round-trip.
**Alternative considered:** store the raw `r:id` on the cell — rejected (ExcelJS users expect the resolved URL string).

### D4. Emit new writer elements in CT_Worksheet schema order

**Why:** OOXML requires a fixed element sequence inside `<worksheet>`. Canonical order: `sheetViews` → `sheetProtection` → `autoFilter` → `mergeCells` → `dataValidations` → `hyperlinks`. The writer inserts each new block at its correct position; no reordering of existing blocks.
**Alternative considered:** append all four at the end — rejected (invalid XML per schema, Excel may reject).

### D5. FFI exposure mirrors existing worksheet properties

**Why:** add `ws.autoFilter` (string getter/setter), `ws.views` / `ws.set_views` (array of `{ state, xSplit, ySplit, topLeftCell, activePane }`), and `ws.protection` / `ws.set_protection` (object of boolean flags). Keep `index.d.ts` / `native.d.ts` in sync with `src/lib.rs`.

## Risks / Trade-offs

- **[Risk] Relationship resolution for hyperlinks** (sheetN.xml.rels path/namespace mismatch) → Mitigation: reuse the writer's existing rels builder naming (`xl/worksheets/_rels/sheet{N}.xml.rels`); unit-test against a file written by excelrs itself (round-trip guarantee).
- **[Risk] Partial/garbage files with malformed `<pane>` or missing attributes** → Mitigation: quick-xml tolerant attribute parsing (optional attributes default), same `validate()`-style guard used for `DataValidation` — drop unparseable entries rather than erroring the whole read.
- **[Risk] CT_Worksheet ordering regressions across Excel/LibreOffice/ExcelJS writers** → Mitigation: emit strictly in schema order and add a round-trip fixture asserting the four elements survive read→write→read.

## Migration Plan

1. Add model fields + `Arc<Mutex<>>` state on inner `Worksheet`; FFI getters/setters on `napi` `Worksheet`.
2. Add reader parsers (4 functions, archive-backed) wired into `workbook_to_inner_model`.
3. Add writer emission at correct schema positions; add hyperlink rels read path.
4. Bump `excelrs` to `0.11.0`; update `CHANGELOG.md` and the `exceljs-parity` matrix (`planned` → `shipped`).
5. No rollback needed — purely additive; previous files remain readable.

## Open Questions

- Should `ws.views` be a single view (ExcelJS allows multiple `<sheetView>`, but typical usage is one)? Plan assumes an array (ExcelJS `worksheet.views` is already an array) but the writer emits the first/freeze view only for v0.11.0.
