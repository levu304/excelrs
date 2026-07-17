# Design: v1.1.0 — Tables

## Context

`excelrs` is a Rust (napi) native addon porting the ExcelJS API. v1.0.0 closed
the drop-in-compat milestone, but the parity matrix still lists **Tables** as
`planned` / `targeted` for v1.1.0. ExcelJS exposes tables via
`ws.addTable` / `ws.getTable(s)` / `ws.removeTable` with a `Table` /
`TableColumn` / `TableRow` model, serialized to the `xl/tables/tableN.xml`
OOXML part.

The codebase already proves the "new OOXML part + relationship + model + napi
bridge" pattern end-to-end:

- **Model types** live in `src/model/*.rs` (e.g. `image.rs`, `sheet_protection.rs`) and bridge to JS/TS via napi.
- **Worksheet methods** live on `Worksheet` in `src/model/worksheet.rs` (`add_image` at L493, `get_images` at L508) — tables reuse this exact shape.
- **Relationships** use the existing rels manager (hyperlinks resolve `r:id` → URL; comments/images register their parts in the sheet `.rels`). Tables reuse the same plumbing.
- **Reader/writer loops** for whole new parts (comments `xl/commentsN.xml`, drawings `xl/drawings/`, media) live in `src/reader/xlsx.rs` + `src/writer/xlsx.rs` and register the part in the workbook reader/writer.

This change is **additive** — no existing API behavior changes; a minor bump
(`1.0.0` → `1.1.0`) signals a new capability.

## Goals / Non-Goals

**Goals:**

- Deliver a full read/write round-trip for worksheet tables matching ExcelJS behavior so `excelrs` satisfies the compat promise for this common, data-heavy feature.
- Reuse the existing model → reader → writer → napi-bridge → rels structure; add no new architectural substrate.
- Advance the `exceljs-parity` matrix `Tables` `planned`/`targeted` → `shipped`.

**Non-Goals:**

- Slicers, structured-reference formula evaluation, computed columns, table-driven filtering of cell data.
- Auto-deriving the worksheet-level `<autoFilter>` element from a table (see D3).
- Table style *rendering* — `tableStyleInfo` is stored/round-tripped as metadata, never used to compute cell styles (ponytail: no speculative style inference).

## Decisions

### D1 — Tables are a separate OOXML part + sheet relationship

A table is its own part `xl/tables/tableN.xml` referenced by a `table`
relationship in `xl/worksheets/_rels/sheetN.xml.rels`.

**Rationale:** matches ExcelJS/Excel on-disk layout; reuses the rels manager already proven by comments/images. Alternative (embedding table data in the worksheet XML) would diverge from the spec and break interop.

### D2 — `ws.addTable` writes header + data values into the worksheet cells

`addTable(opts)` populates the header row, data rows, and optional totals row
into the referenced range's cells, then emits `xl/tables/tableN.xml`. On read,
the table part is parsed into the `Table` model and the cell values are read by
the existing cell reader. `ws.removeTable(name)` deletes the table part + rel
and **leaves the underlying cells intact** — matching ExcelJS, which removes
only the table definition.

**Rationale:** faithful to ExcelJS, where the table is a view over real cells; keeps read path simple (no special cell handling). Alternative (storing values only in the model) would desync the worksheet on round-trip.

### D3 — Table `autoFilter` is self-contained inside the table part

A table's `<autoFilter ref="…"/>` lives inside `xl/tables/tableN.xml`. The
worksheet-level `<autoFilter>` element (the `auto-filter` spec) is **not**
auto-derived from a table. Both features stay decoupled; no double-emit.

**Rationale:** avoids coupling two features and the ambiguity of which element owns the range. ExcelJS-authored tables carry `autoFilter` inside the table part; we mirror that exactly.
**Open Question:** ExcelJS also exposes `ws.autoFilter` when a table is present in some flows — confirm whether v1.1.0 should additionally set the worksheet `autoFilter` element for byte-exact ExcelJS round-trip, or leave it to the user (recommended: leave to user; decoupled is simpler and already round-trips Excel-authored files).

### D4 — `Table` / `TableColumn` / `TableRow` model

- `Table { name, display_name, ref, header_row: bool, totals_row: bool, columns: Vec<TableColumn>, rows: Vec<TableRow>, style: Option<TableStyle>, autofilter_ref: Option<String> }`
- `TableColumn { name, totals_row_label: Option<String>, totals_row_function: Option<String> }`
- `TableRow { values: Vec<CellValue> }` (header row is `rows[0]` when `header_row`)
- `TableStyle { name, show_first_column, show_last_column, show_row_stripes, show_column_stripes }` — metadata only.

**Rationale:** 1:1 with the OOXML `<table>`/`<tableColumns>`/`<tableStyleInfo>` schema; minimal but complete. No builder-layer abstraction (ponytail: no factory for one product).

### D5 — Round-trip fidelity acceptance bar

Every table feature's correctness is proven by a read→write→read fixture,
exercising both an Excel-authored table file and an ExcelJS-authored table
(`ws.addTable`, then re-read). No feature ships without a fixture. Mirrors the
approach used by `images`, `hyperlinks`, `worksheet-views`.

### D6 — Writer omits parts when absent

A worksheet with no tables SHALL NOT emit `xl/tables/` entries or `table`
relationships for that sheet. `totalsRowShown`/`<autoFilter>`/`<tableStyleInfo>`
are emitted only when the corresponding model fields are present.

## Risks / Trade-offs

- **[Risk] Totals-row formula vs label** → store both `totalsRowFunction` and `totalsRowLabel`; emit whichever is set, matching ExcelJS column options.
- **[Risk] Name collisions / invalid `ref`** → validate `name` uniqueness per worksheet and that `ref` covers `header + data (+totals)`; return a clear `ExcelrsError` rather than emitting a corrupt part.
- **[Risk] `displayName` vs `name`** → Excel requires a distinct `displayName` when the name collides with a defined name; default `displayName = name` and let the user override.
- **[Risk] Reader drift vs Excel-authored files** → fixtures include a real Excel-authored `.xlsx` table, not just ExcelJS output, to catch schema gaps.

## Migration Plan

1. Implement model (`src/model/table.rs`), worksheet methods, reader/writer part, rels registration, napi bridge.
2. Add round-trip fixtures in `fixtures/` + `__test__/`.
3. Bump `package.json` `1.0.0` → `1.1.0`; update `CHANGELOG.md` and the `ROADMAP.md` parity matrix (`tables` → `shipped`).
4. Sync the `exceljs-parity` spec delta and archive the change when all fixtures pass.

No migration shim needed — API strictly additive.

## Open Questions

- Worksheet-level `<autoFilter>` derivation from a table (D3) — confirm desired behavior before finalizing public API. (Recommend decoupled.)
- Should `headerRow: false` tables still populate `rows[0]` as data? (Recommend: when `header_row` is false, the first data row is `rows[0]`, no header model row.)
