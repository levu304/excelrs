# excelrs → ExcelJS Porting Roadmap

**Generated:** 2026-07-14 | **ExcelJS version pinned:** [4.4.0](https://www.npmjs.com/package/exceljs/v/4.4.0) | **excelrs version:** 1.1.0 (released 2026-07-16)

> **Next:** v2.0.0 is the planned capstone (see *Post-v1 Roadmap* below). The v1.1.0 → v2.0.0 scope **provisional**, gated on an ExcelJS 4.4.0 API audit (Step 0) before each feature's design. *Audit complete: streaming surface is non-breaking (new `stream` namespace only); v2.0.0 is now in implementation.*

---

## Parity Matrix

| Feature Area | Status | Shipped In | Notes |
| --- | --- | --- | --- |
| **Workbook IO** | | | |
| XLSX read | shipped | v0.1.0 | Calamine-backed |
| XLSX write | shipped | v0.1.0 | Zip + quick-xml |
| CSV read | shipped | v0.9.0 | Manual RFC 4180 parser |
| CSV write | shipped | v0.9.0 | Manual RFC 4180 serializer |
| Streaming XLSX | shipped | v2.0.0 | Large-file streaming reader/writer (SAX-based) |
| **Worksheet structure** | | | |
| Rows / columns CRUD | partial | v0.1.0 | `getRow`/`addRow`/`getRows`/`columns()`; no `getColumn`/`splice`/`insertRow` |
| Merge cells | shipped | v0.5.0 | mergeCells, unMergeCells |
| Freeze / split panes | shipped | v0.11.0 | `<sheetViews><pane>` read/write implemented; `ws.views` |
| Auto filters | shipped | v0.11.0 | `<autoFilter ref>` read/write; `ws.autoFilter` |
| Insert / splice rows | targeted | v1.3.0 | `insertRow`/`insertRows`/`spliceRows` — targeted for v1.3.0 |
| Duplicate row | targeted | v1.3.0 | `duplicateRow` — targeted for v1.3.0 |
| Column widths / headers | shipped | v0.1.0 | |
| Outline levels (rows/cols) | targeted | v1.3.0 | Row/col grouping (`outlineLvl`) — targeted for v1.3.0 |
| Page breaks | targeted | v1.3.0 | `rowBreaks`/`colBreaks` — targeted for v1.3.0 |
| **Cell values & types** | | | |
| Number, String, Bool, Error | shipped | v0.1.0 | |
| Formula (read/write) | shipped | v0.1.0 | Stored as string formula + cached value |
| Shared formula | shipped | v0.1.0? | Expanded on write; streaming read resolves member cells (#25.2) |
| Array formula | n-a | — | Rare; deferred |
| Date/DateTime | shipped | v0.13.0 | Full round-trip; Date cell values preserved as JS Date via napi bridge (was ISO-8601 string) |
| Hyperlink | shipped | v0.11.0 | Full read/write round-trip; r:id → URL resolution via sheet rels |
| RichText | shipped | v0.12.0 | Full read/write round-trip; inline `<is>` strings parsed |
| **Styles** | | | |
| Font (name, size, color, bold, italic, etc.) | shipped | v0.2.0+/v0.3.0 | Full round-trip |
| Fill (solid/pattern) | shipped | v0.2.0+/v0.3.0 | |
| Fill (gradient) | shipped | v0.12.0 | Full read/write; `<gradientFill>` parsed (linear + path) |
| Border (left/right/top/bottom) | shipped | v0.2.0+/v0.3.0 | |
| Border (diagonal) | shipped | v0.12.0 | Full read/write; `<diagonal>` side + `diagonalUp`/`diagonalDown` parsed |
| Alignment | shipped | v0.2.0+/v0.3.0 | Full round-trip; vertical middle→center mapping |
| Number format | shipped | v0.2.0+/v0.3.0 | |
| Row-level style | shipped | v0.5.0 | |
| Theme color refs (read) | shipped | v0.6.0 | Resolved via `xl/theme/theme1.xml` |
| Theme color refs (write) | shipped | v0.13.0 | Emits `<color theme="N"/>` (+`tint`); ARGB resolution retained for read/public API |
| Indexed color refs | shipped | v0.6.0 | 56-entry system palette |
| Tint support | shipped | v0.6.0 | OOXML tint algorithm applied on read |
| **Workbook** | | | |
| Defined names | shipped | v0.7.0 | Workbook-global + sheet-scoped |
| Workbook properties | shipped | v0.1.0 | creator, modified, created, etc. |
| Workbook views | shipped | v1.0.0 | Workbook views + calc properties (`calcPr`) read/write |
| Calc properties | shipped | v1.0.0 | `fullCalcOnLoad` read/write (`<calcPr>`) |
| Themes (write) | planned | — | Read-only via theme1.xml |
| **Worksheet** | | | |
| Data validation | shipped | v0.8.0 | Full read/write, all types |
| State (visible/hidden) | planned | — | Not implemented |
| Tab color | planned | — | Not implemented |
| Properties (defaultRowHeight, etc.) | planned | — | Not implemented |
| Page setup / print | shipped | v1.0.0 | `pageMargins`, `paperSize`, `orientation`, `printArea`, `printTitles` read/write |
| Headers and footers | shipped | v1.0.0 | `<headerFooter>` read/write with format codes |
| Sheet protection | shipped | v0.11.0 | `<sheetProtection>` read/write; `ws.protection` |
| **Other features** | | | |
| Comments | shipped | v1.0.0 | `xl/commentsN.xml` part + relationship read/write |
| Images / drawings | shipped | v1.0.0 | `xl/drawings/` part, media extraction, anchors read/write |
| Tables | shipped | v1.1.0 | `xl/tables/` part + relationship read/write; `ws.addTable`/`getTable(s)`/`removeTable`; header/totals rows; `autoFilter` integration; header styling (metadata only) |
| Conditional formatting | shipped | v1.2.0 | `<conditionalFormatting>` + `dxfs`; rule types `cellIs`, `expression`, `colorScale`, `dataBar`, `iconSet`, `top10`, `unique`/`duplicate`, `containsText`, `timePeriod`, blanks/errors/nonBlanks; priority ordering |
| Charts | planned (distant) | — | Major subsystem; chart XML is very complex |
| Pivot tables | planned (distant) | — | Major subsystem; extremely complex |
| Formula evaluation | n-a | — | Separate interpreter; deferred v1+ |

**Status legend:**

- **shipped** — fully usable, matches ExcelJS API expectations
- **partial** — partially implemented; write works or read works but not both
- **planned** — not yet implemented, targeted for a future release
- **targeted** — planned and assigned to a specific upcoming release (see Prioritized Roadmap)
- **n-a** — explicitly out of scope for the drop-in compat promise (v1)

---

## Prioritized Roadmap

Prioritization: **compat value dominates effort** (D3). Items are ordered by compat value (high → low), then by effort (low → high within a tier).

### [v0.11.0] — ✅ Shipped (2026-07-14)

Quick-win data completeness: hyperlinks (read), auto filters, freeze panes, sheet protection — all four `planned` → `shipped`.

### [v0.12.0] — Rich content round-trip ✅

| Rank | Feature | Effort | Status |
| ------ | --------- | -------- | ------ |
| 5 | **RichText read** | low | ✅ shipped |
| 6 | **Gradient fill (read)** | low | ✅ shipped |
| 7 | **Diagonal border (read)** | low | ✅ shipped |
| 8 | **JS Date preservation** | low/med | deferred to v0.13.0 — separate FFI type-bridging effort |

### [v0.13.0] — Style write fidelity + Date preservation ✅

| Rank | Feature | Effort | Status |
| ------ | --------- | -------- | ------ |
| 9 | **Theme color (write)** | med | ✅ shipped — emits `<color theme="N"/>` (+`tint`) |
| 10 | **JS Date preservation** | med | ✅ shipped — Date cell values bridge as `napi::JsDate`; `Cell.value` returns `Date \| CellValue` |

### [v0.13.0+] — Medium-effort additions

| Rank | Feature | Effort | Rationale |
| ------ | --------- | -------- | ----------- |
| 9 | **Theme color (write)** | med | ✅ shipped (v0.13.0) — emits `<color theme="N"/>` (+`tint`) |
| 10 | **Headers and footers** | med | `<headerFooter>` element; supports format codes. ExcelJS API surface is moderate. ✅ shipped (v1.0.0) |
| 11 | **Page setup / print** | med | `pageMargins`, `paperSize`, `orientation`, `printArea`, `printTitles`. Many attributes but each is simple. ✅ shipped (v1.0.0) |
| 12 | **Workbook views / calc properties** | med | Workbook views + `calcPr` element. Straightforward OOXML. ✅ shipped (v1.0.0) |
| 13 | **Comments** | med | Needs new OOXML part (`xl/commentsN.xml` + relationship). Moderate model + reader/writer. ✅ shipped (v1.0.0) |
| 14 | **Images / drawings** | med/high | Needs drawing part (`xl/drawings/`), relationships, media extraction. Significant plumbing but self-contained. ✅ shipped (v1.0.0) |

### [v1.1.0]–[v2.0.0] Post-v1 Roadmap

The v1.0.0 drop-in compatibility milestone is complete. Post-v1 work ships as a **v1.x.x minor series** (additive, compat-value first — roadmap principle D3) and culminates in a **v2.0.0 capstone**.

> **Provisional scope.** Every release below is gated on **Step 0: audit the real ExcelJS 4.4.0 API** (already a devDependency — `npm i exceljs@4.4.0`) for that area's surface and emitted OOXML, *before* locking its design. The mapping here is the recommended target; the audit may resize any release.

| Target | Feature | Effort | Status | Notes |
| --- | --- | --- | --- | --- |
| **v1.1.0** | **Tables** | high | shipped | `ws.addTable` / `ws.getTable(s)` / `ws.removeTable`; `Table` / `TableColumn` / `TableRow` model; `xl/tables/tableN.xml` + relationship; `autoFilter` integration; header/totals rows; header styling |
| **v1.2.0** | **Conditional formatting** | high | targeted | read/write `<conditionalFormatting>` + `dxfs`; rule types `cellIs`, `expression`/formula, `colorScale`, `dataBar`, `iconSet`, `top10`, `unique`/`duplicate`, `containsText`, `timePeriod`, blanks/errors/nonBlanks; priority ordering |
| **v1.3.0** | **Worksheet-structure parity finish** | medium | targeted | `insertRow(s)` / `spliceRows` / `duplicateRow`; row/col `outlineLevel` (grouping); `rowBreaks` / `colBreaks` page breaks — closes the remaining "planned" v1.x parity-matrix rows |
| **v2.0.0** | **Streaming XLSX + parity capstone** | high | shipped | streaming reader/writer architecture for large files (SAX-based); **declares the ExcelJS-4.4.0 v1.x drop-in parity program complete** (exclusions: charts, pivot tables, formula evaluation, themes-write, sheet state, tab color, default properties); streaming surface is non-breaking (new `stream` namespace only) |

**Deferred to v3+ (distant, unchanged):**

| Feature | Effort | Rationale |
| --- | --- | --- |
| Charts | very high | Entire chart engine; chart XML is extremely verbose and version-specific |
| Pivot tables | very high | Complex OOXML with pivotCache, pivotTable, multiple axis types |
| Formula evaluation | very high | Spreadsheet formula interpreter; CLO=n (not a trivial project) |

**Deferred minor-parity (not in v2.0.0; triage after the v2.0.0 capstone):** `Themes (write)`, `State (visible/hidden)`, `Tab color`, `Properties (defaultRowHeight, etc.)`. Smaller ExcelJS API gaps, explicitly out of v2.0.0 scope but tracked for post-v2.0.0 triage.

---

## Stale Docs Reconciliation (`docs/spec.md` §9.2.1)

The §9.2.1 deferred-items table lists six items shipped in v0.5.0–v0.6.0. All claims are **accurate** — they correctly record what shipped and in which version. However, the table has not been updated since v0.6.0: it does not account for v0.7.0 (defined names), v0.8.0 (data validation), or v0.9.0 (CSV). No stale or incorrect claims found — just an incomplete picture.

The spec document's version (v1.4.1) and `§1 Scope` also still reference v0.6.0 as the "current" scope. This is cosmetic — the content is still valid — but the metadata headers should be bumped when the spec is next modified.
