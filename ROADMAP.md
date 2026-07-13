# excelrs → ExcelJS Porting Roadmap

**Generated:** 2026-07-13 | **ExcelJS version pinned:** [4.4.0](https://www.npmjs.com/package/exceljs/v/4.4.0) | **excelrs version:** 0.9.0

---

## Parity Matrix

| Feature Area | Status | Shipped In | Notes |
| --- | --- | --- | --- |
| **Workbook IO** | | | |
| XLSX read | shipped | v0.1.0 | Calamine-backed |
| XLSX write | shipped | v0.1.0 | Zip + quick-xml |
| CSV read | shipped | v0.9.0 | Manual RFC 4180 parser |
| CSV write | shipped | v0.9.0 | Manual RFC 4180 serializer |
| Streaming XLSX | n-a | — | Perf-oriented; deferred from v1 |
| **Worksheet structure** | | | |
| Rows / columns CRUD | partial | v0.1.0 | `getRow`/`addRow`/`getRows`/`columns()`; no `getColumn`/`splice`/`insertRow` |
| Merge cells | shipped | v0.5.0 | mergeCells, unMergeCells |
| Freeze / split panes | planned | — | `<sheetViews>` not implemented |
| Auto filters | planned | — | `autoFilter` attribute not implemented |
| Insert / splice rows | planned | — | `insertRow`/`insertRows`/`spliceRows` not implemented |
| Duplicate row | planned | — | `DuplicateRow` not implemented |
| Column widths / headers | shipped | v0.1.0 | |
| Outline levels (rows/cols) | planned | — | Row/col grouping (`outlineLvl`) not implemented |
| Page breaks | planned | — | `rowBreaks`/`colBreaks` not implemented |
| **Cell values & types** | | | |
| Number, String, Bool, Error | shipped | v0.1.0 | |
| Formula (read/write) | shipped | v0.1.0 | Stored as string formula + cached value |
| Shared formula | shipped | v0.1.0? | Expanded on write |
| Array formula | n-a | — | Rare; deferred |
| Date/DateTime | partial | v0.1.0 | Read → ISO-8601 string; JS `new Date()` not preserved as Date type on write |
| Hyperlink | partial | v0.5.0 | Write emitted; reader doesn't parse `<hyperlinks>` |
| RichText | partial | v0.5.0 | Write emitted; reader doesn't parse inlineStr |
| **Styles** | | | |
| Font (name, size, color, bold, italic, etc.) | shipped | v0.2.0+/v0.3.0 | Full round-trip |
| Fill (solid/pattern) | shipped | v0.2.0+/v0.3.0 | |
| Fill (gradient) | partial | v0.5.0 | Write emitted; reader silently skips gradientFill (reader/styles.rs:16) |
| Border (left/right/top/bottom) | shipped | v0.2.0+/v0.3.0 | |
| Border (diagonal) | partial | v0.5.0 | Write emitted (writer/styles.rs:387); reader silently skips diagonal (reader/styles.rs:17) |
| Alignment | shipped | v0.2.0+/v0.3.0 | Full round-trip; vertical middle→center mapping |
| Number format | shipped | v0.2.0+/v0.3.0 | |
| Row-level style | shipped | v0.5.0 | |
| Theme color refs (read) | shipped | v0.6.0 | Resolved via `xl/theme/theme1.xml` |
| Theme color refs (write) | planned | — | Only read-side; `<color theme="N">` not emitted |
| Indexed color refs | shipped | v0.6.0 | 56-entry system palette |
| Tint support | shipped | v0.6.0 | OOXML tint algorithm applied on read |
| **Workbook** | | | |
| Defined names | shipped | v0.7.0 | Workbook-global + sheet-scoped |
| Workbook properties | shipped | v0.1.0 | creator, modified, created, etc. |
| Workbook views | planned | — | Not implemented |
| Calc properties | planned | — | `fullCalcOnLoad` not implemented |
| Themes (write) | planned | — | Read-only via theme1.xml |
| **Worksheet** | | | |
| Data validation | shipped | v0.8.0 | Full read/write, all types |
| State (visible/hidden) | planned | — | Not implemented |
| Tab color | planned | — | Not implemented |
| Properties (defaultRowHeight, etc.) | planned | — | Not implemented |
| Page setup / print | planned | — | Not implemented (pageMargins, orientation, paperSize, printArea, etc.) |
| Headers and footers | planned | — | Not implemented |
| Sheet protection | planned | — | Not implemented |
| **Other features** | | | |
| Comments | planned | — | Whole new OOXML part (`xl/commentsN.xml`) |
| Images / drawings | planned | — | Whole new OOXML part (`xl/drawings/`) |
| Tables | planned | — | Complex OOXML part (`xl/tables/`) |
| Conditional formatting | planned | — | Complex OOXML + dxfs |
| Charts | planned (distant) | — | Major subsystem; chart XML is very complex |
| Pivot tables | planned (distant) | — | Major subsystem; extremely complex |
| Formula evaluation | n-a | — | Separate interpreter; deferred v1+ |

**Status legend:**

- **shipped** — fully usable, matches ExcelJS API expectations
- **partial** — partially implemented; write works or read works but not both
- **planned** — not yet implemented, targeted for a future release
- **n-a** — explicitly out of scope for the drop-in compat promise (v1)

---

## Prioritized Roadmap

Prioritization: **compat value dominates effort** (D3). Items are ordered by compat value (high → low), then by effort (low → high within a tier).

### [v0.11.0] — Quick-win data completeness (low effort, high compat impact)

| Rank | Feature | Effort | Rationale |
| ------ | --------- | -------- | ----------- |
| 1 | **Hyperlinks (read)** | low | Write already ships (v0.5.0). Add `<hyperlinks>` parsing in reader. Closes a visible gap: ExcelJS users expect `cell.value = { hyperlink: '…', text: '…' }` to round-trip. |
| 2 | **Auto filters** | low | Single `<autoFilter ref="…">` attribute on worksheet; trivial to emit and parse. ExcelJS exposes `ws.autoFilter = 'A1:C1'`. High visibility feature. |
| 3 | **Freeze panes / split views** | low | `<sheetViews><sheetView><pane>`, `<sheetView state="frozen">`. Straightforward OOXML. ExcelJS: `worksheet.views = [{state: 'frozen', xSplit:…}]`. |
| 4 | **Sheet protection** | low | `<sheetProtection>` element with boolean flags. ExcelJS: `ws.properties.protection = { … }`. |

### [v0.12.0] — Rich content round-trip (medium effort)

| Rank | Feature | Effort | Rationale |
| ------ | --------- | -------- | ----------- |
| 5 | **RichText read** | low | Write already ships (v0.5.0). Add inlineStr/SI parsing in reader. High compat impact for formatted cell content. |
| 6 | **Gradient fill (read)** | low | Write already ships (v0.5.0). Currently silently skipped in reader (reader/styles.rs `gradientFill => skip`). Adding read closes a style gap. |
| 7 | **Diagonal border (read)** | low | Write already ships (v0.5.0). Currently silently skipped in reader (reader/styles.rs `diagonal`). Same pattern. |
| 8 | **JS Date preservation** | low/med | `cell.value = new Date()` → Date type across FFI. Currently string-coerced. Requires napi-rs Date type or dedicated handling. |

### [v0.13.0+] — Medium-effort additions

| Rank | Feature | Effort | Rationale |
| ------ | --------- | -------- | ----------- |
| 9 | **Theme color (write)** | med | Emit `<color theme="N">` on write. Currently only read-side. |
| 10 | **Headers and footers** | med | `<headerFooter>` element; supports format codes. ExcelJS API surface is moderate. |
| 11 | **Page setup / print** | med | `pageMargins`, `paperSize`, `orientation`, `printArea`, `printTitles`. Many attributes but each is simple. |
| 12 | **Workbook views / calc properties** | med | Workbook views + `calcPr` element. Straightforward OOXML. |
| 13 | **Comments** | med | Needs new OOXML part (`xl/commentsN.xml` + relationship). Moderate model + reader/writer. |
| 14 | **Images / drawings** | med/high | Needs drawing part (`xl/drawings/`), relationships, media extraction. Significant plumbing but self-contained. |

### [Post-v1 / v2] — Heavy subsystems

| Feature | Effort | Rationale |
| --------- | -------- | ----------- |
| Tables | high | Complex OOXML part with column definitions, auto-filter integration |
| Conditional formatting | high | Complex OOXML with dxfs, multiple rule types, priority ordering |
| Charts | very high | Entire chart engine; chart XML is extremely verbose and version-specific |
| Pivot tables | very high | Complex OOXML with pivotCache, pivotTable, multiple axis types |
| Formula evaluation | very high | Spreadsheet formula interpreter; CLO=n (not a trivial project) |
| Streaming XLSX | high | Needs streaming reader/writer architecture; perf-optimization for large files |

---

## Stale Docs Reconciliation (`docs/spec.md` §9.2.1)

The §9.2.1 deferred-items table lists six items shipped in v0.5.0–v0.6.0. All claims are **accurate** — they correctly record what shipped and in which version. However, the table has not been updated since v0.6.0: it does not account for v0.7.0 (defined names), v0.8.0 (data validation), or v0.9.0 (CSV). No stale or incorrect claims found — just an incomplete picture.

The spec document's version (v1.4.1) and `§1 Scope` also still reference v0.6.0 as the "current" scope. This is cosmetic — the content is still valid — but the metadata headers should be bumped when the spec is next modified.

---

## Change Stubs (seeded)

The top-ranked items for v0.11.0 are:

- `v0-11-0-hyperlinks-read` — Add `<hyperlinks>` parsing to the reader
- `v0-11-0-auto-filters` — Add `autoFilter` attribute support

These can be created with `openspec new change <name>` when v0.10.0 is archived.
