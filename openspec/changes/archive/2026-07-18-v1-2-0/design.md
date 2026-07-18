# Design: v1.2.0 — Conditional formatting

## Context

`excelrs` is a Rust (napi) native addon porting the ExcelJS API. v1.1.0 shipped
worksheet tables, but the parity matrix still lists **Conditional formatting** as
`targeted` for v1.2.0. ExcelJS exposes conditional formatting via
`ws.addConditionalFormatting({ ref, rules })` / `ws.getConditionalFormatting()`
with a broad rule-type set, serialized across two OOXML surfaces: the worksheet
`<conditionalFormatting sqref="…">` element and differential formats (`<dxfs>`)
in `xl/styles.xml`.

The codebase already proves the relevant patterns end-to-end:

- **Rule-on-worksheet model** is proven by data validation: `src/model/data_validation.rs`
  (`DataValidation` napi object) + `Worksheet.add_data_validation` at
  `src/model/worksheet.rs:389` + `parse_sheet_data_validations` in `src/reader/xlsx.rs`
  - emit loop in `src/writer/xlsx.rs`. Conditional formatting reuses this exact shape.
- **New OOXML part + reader/writer loop** is proven by tables (`src/model/table.rs`,
  `src/reader/xlsx.rs`, `src/writer/xlsx.rs`) and by the styles surface itself
  (`src/reader/styles.rs`, `src/writer/styles.rs`).
- **dxfs are currently skipped**: `src/reader/styles.rs:577` lists `dxfs` (and
  `cellStyleXfs`, `cellStyles`, `tableStyles`, `extLst`) as skipped, and the writer
  emits none. v1.2.0 must implement dxfs read + write — the one genuinely new
  substrate this change introduces.

This change is **additive** — no existing API behavior changes; a minor bump
(`1.1.0` → `1.2.0`) signals a new capability.

## Goals / Non-Goals

**Goals:**

- Deliver a full read/write round-trip for worksheet conditional formatting matching
  ExcelJS behavior across all roadmap rule types, so `excelrs` satisfies the compat
  promise for this core feature.
- Implement `<dxfs>` read + write in `xl/styles.xml` (closing the current skip), so
  differential formats survive round-trip and `dxfId` references resolve.
- Reuse the existing model → reader → writer → napi-bridge structure; add no new
  architectural substrate beyond `dxfs`.
- Advance the `exceljs-parity` matrix `conditional formatting` `targeted` → `shipped`.

**Non-Goals:**

- Computing which cells actually render styled (that is Excel's render-time job; we
  store `dxfs` faithfully, never infer cell appearance).
- Editing the shared cell styles (`cellXfs`) for conditional formatting — only
  differential formats (`dxfs`) are used.
- Slicers, table-scoped conditional formatting, or PivotTable rule types.
- Custom icon sets / arbitrary extLst extensions on `iconSet` beyond the named set
  - optional `reverse`/`showValue`.

## Decisions

### D1 — Two OOXML surfaces: worksheet `<conditionalFormatting>` + styles `<dxfs>`

A conditional format lives in the worksheet XML as `<conditionalFormatting
sqref="A1:A10">` containing one `<cfRule>` per rule, and any rule with a `style`
references a `dxf` in `xl/styles.xml` via `dxfId`. `colorScale` / `dataBar` /
`iconSet` rules carry their visuals inline (`cfvo` + colors) and use no `dxfId`.

**Rationale:** matches ExcelJS/Excel on-disk layout exactly. Diverging (e.g. merging
styles into `cellXfs`) would break interop and double-apply styles.

### D2 — Flat `CfRule` object mirroring ExcelJS, not a per-type enum

`CfRule` is a single `#[napi(object)]` struct with `type: String` plus optional
fields (`operator`, `formula`, `text`, `time_period`, `rank`, `percent`, `bottom`,
`style`, `cfvo`, `color`, `data_bar_color`, `icon_set`, `dxf_id`, `priority`).
This mirrors how ExcelJS represents a rule (one object, many nullable fields) rather
than a Rust enum per type.

**Rationale:** 1:1 with the ExcelJS public shape; the JS/TS bridge stays trivial and
serialization maps directly to OOXML attributes/children. No per-type trait/visitor
abstraction (ponytail: no enum explosion for one consumer).

### D3 — `dxfs` are a real, parsed, shared collection on the styles model

Add `dxfs: Vec<Dxf>` to the styles model. A `Dxf` records only the bits present
(font / fill / border / optional numFmt) — the differential subset. A rule with a
`style` appends a `Dxf` (deduped by content) and sets `dxfId` to its index. The
reader parses `<dxfs>` (replacing the current skip) into the same `Vec<Dxf>`.

**Rationale:** faithful round-trip of differential formats; `dxfId` is a stable
index into the shared `dxfs` collection, matching OOXML. Content-dedup keeps the
part small (ponytail: a `HashMap<Dxf, usize>` during write, no extra framework).

### D4 — `priority` is worksheet-global, unique, 1-based

Each `<cfRule>` carries `priority` (unique across the whole sheet in OOXML). On read,
`priority` is taken verbatim. On write for excelrs-authored rules, priority is
assigned by document order (`1..N`) across all rules in the sheet. `addConditionalFormatting`
accepts an optional `priority` per rule but defaults to order-based assignment.

**Rationale:** OOXML requires sheet-unique priority; ExcelJS models it implicitly via
array order. Preserving read priority verbatim keeps Excel-authored files byte-stable
where it matters for rule evaluation order.

### D5 — Full rule-type coverage, inline vs dxf by type

All roadmap rule types are emitted: `cellIs` (`operator` + `<formula>`), `expression`
(`<formula>`), `colorScale` (`<colorScale>` with `<cfvo>` + `<color>`), `dataBar`
(`<dataBar>` with `<cfvo>` + `rgb`), `iconSet` (`<iconSet>` with `iconSet` attr +
`<cfvo>`), `top10` (`rank`/`percent`/`bottom`), `unique`/`duplicate`, `containsText`
(`operator` + `text` + `<formula>`), `timePeriod` (`timePeriod`), and
`containsBlanks`/`notContainsBlanks`/`containsErrors`/`notContainsErrors` (no extra
fields). Only `colorScale`/`dataBar`/`iconSet` omit `dxfId`.

**Rationale:** roadmap pins the full set; partial coverage would re-break the compat
promise. `cfvo` type enum covers `num` / `percent` / `percentile` / `formula` / `min`
/ `max` / `autoMin` / `autoMax`.

### D6 — Round-trip fidelity acceptance bar

Every rule type's correctness is proven by a read→write→read fixture, exercising both
an Excel-authored conditional-format file and an ExcelJS-authored one
(`ws.addConditionalFormatting`, then re-read). Non-cf `dxfs` (e.g. pivot-table dxfs)
must survive round-trip unmodified. No rule type ships without a fixture. Mirrors the
approach used by `images`, `hyperlinks`, `tables`.

### D7 — Writer element ordering matches the OOXML schema

`<conditionalFormatting>` is emitted after `<sheetData>` in the worksheet, at the
schema-permitted position (after `sheetData`, alongside `dataValidations`/
`autoFilter`). `<dxfs>` is emitted in `xl/styles.xml` after `cellXfs` and before
`tableStyles` (the order enforced by `emit_styles_xml` in `src/writer/styles.rs`).
Both are omitted entirely when empty.

**Rationale:** wrong element order makes the file fail schema validation in Excel.
The data-validation emit already sits in the correct block; conditional formatting
slots in beside it.

### D8 — Writer omits parts when absent

A worksheet with no conditional formats SHALL NOT emit `<conditionalFormatting>`.
A workbook with zero `dxfs` SHALL NOT emit a `<dxfs>` element. `cfvo`/`color`/
`iconSet`-specific children are emitted only when the rule type uses them.

## Risks / Trade-offs

- **[Risk] dxfs currently skipped** → implement parse + emit (D3/D7); this is the one
  new substrate. Must not drop foreign (non-cf) dxfs or the file corrupts (see D6
  scenario + below).
- **[Risk] Foreign dxfs from other features** → a source workbook may carry `dxfs`
  from pivot tables / smart art that `excelrs` does not model. Mitigation: parse the
  common differential subset we understand; for any `dxf` with child elements we do
  not model, retain the raw inner XML and re-emit it verbatim so the file stays valid.
  (Open Question: confirm verbatim preservation is acceptable vs. dropping unknown dxfs.)
- **[Risk] dxfId collisions** → content-dedup `dxfs` during write so identical styles
  share one `dxf`; `dxfId` always points at the correct index.
- **[Risk] colorScale 2- vs 3-color** → emit `<color>` per `cfvo`; length drives
  whether OOXML sees a 2- or 3-stop scale. No special-casing.
- **[Risk] iconSet `reverse`/`showValue`/custom icons** → store optional `reverse` +
  `showValue` on the iconSet rule; custom icon arrays deferred (Non-Goal) but the
  named `iconSet` is preserved.
- **[Risk] Reader drift vs Excel-authored files** → fixtures include a real
  Excel-authored `.xlsx` with mixed rule types, not just ExcelJS output, to catch
  schema gaps.

## Migration Plan

1. Implement `dxfs` model (`Dxf`) + parse in `src/reader/styles.rs` (remove `dxfs`
   from the skip list) + emit in `src/writer/styles.rs`.
2. Implement conditional-formatting model (`src/model/conditional_formatting.rs`),
   `Worksheet.addConditionalFormatting` / `getConditionalFormatting` in
   `src/model/worksheet.rs` (near `add_data_validation`, L389), reader
   `parse_sheet_conditional_formattings` in `src/reader/xlsx.rs`, and emit in
   `src/writer/xlsx.rs` (beside data validations).
3. Add round-trip fixtures in `fixtures/` + `__test__/` covering every rule type,
   ExcelJS-authored and Excel-authored, plus foreign-dxf preservation.
4. Bump `package.json` `1.1.0` → `1.2.0`; update `CHANGELOG.md` and the `ROADMAP.md`
   parity matrix (`conditional formatting` → `shipped`); sync the `exceljs-parity`
   spec delta.
5. Archive the change when all fixtures pass.

No migration shim needed — API strictly additive.

## Open Questions

- **Remove API**: ExcelJS has no `removeConditionalFormatting`. Recommend v1.2.0 ships
  only `addConditionalFormatting` + `getConditionalFormatting`; a `removeConditionalFormatting(ref)`
  can follow if users ask. (Recommend: defer.)
- **Foreign dxf preservation**: confirm verbatim re-emit of unmodeled `dxf` children is
  the desired behavior (vs. dropping them). (Recommend: preserve verbatim to avoid
  corrupting pivot-table/smart-art workbooks.)
- **`getConditionalFormatting` shape**: return `ConditionalFormat[]` (`{ sqref, rules }`)
  grouped by range, or a flat `CfRule[]`? (Recommend: grouped by range, matching the
  `{ ref, rules }` add shape for symmetric round-trip.)
