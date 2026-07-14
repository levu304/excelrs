## Context

`excelrs` v0.11.0 is the current release. The `CellValue` model
(`src/model/cell.rs`) already carries `rich_text: Option<Vec<RichTextRun>>`, and
the `Fill`/`Border` style structs (`src/model/style.rs`) already carry gradient
and diagonal fields. The **writer** (`src/writer/styles.rs`,
`src/writer/xlsx.rs`) already emits all three. The **reader** is the only gap:

- `src/reader/xlsx.rs::map_data` collapses everything calamine returns into
  `Number`/`String`/`Boolean`/`Error` — no `RichText` variant is produced.
- `src/reader/styles.rs::parse_style_table` hits `b"gradientFill" => { /* skip */ }`
  and only parses `left`/`right`/`top`/`bottom` borders (diagonal ignored).

All three are therefore "read the thing we already write" — symmetric, low-risk
parser additions. No model or writer changes are required.

## Goals / Non-Goals

**Goals:**

- Reader recovers rich text, gradient fills, and diagonal borders so that
  files written by `excelrs` (or Excel/ExcelJS) round-trip losslessly.
- Advance the three parity-matrix entries `partial` → `shipped`.

**Non-Goals:**

- JS `Date` value type — deferred to v0.13.0 (separate FFI type-bridging work).
- Theme-color **write**, comments, images, tables, conditional formatting, charts,
  pivot tables — out of scope per roadmap.
- Any writer or model-struct changes.

## Decisions

### D1 — Gradient fill: parse into the existing `gradient_degree` field, not `gradient_angle`

The `Fill` struct has both `gradient_degree` (the field the writer actually
emits) and `gradient_angle` (writer comment: *"deprecated and never emitted —
silently ignored"*). On read we populate `gradient_degree` / `gradient_type` /
`gradient_stops` / path geometry to match exactly what the writer outputs.
`gradient_angle` stays deprecated and unread. **Rationale:** keep read and write
symmetric on the field that actually serializes; avoids reviving a dead field.
(If a future change wants `gradient_angle` semantics, that's a separate
decision.)

### D2 — Diagonal border: treat `<diagonal>` like the other four sides + capture border-level attrs

In `parse_style_table`, the border-side parser already handles
`left`/`right`/`top`/`bottom`. Add `diagonal` to that same match, mapping into
`Border.diagonal`. Separately, read the `diagonalUp` / `diagonalDown` boolean
attributes off the `<border>` element (currently not captured) into
`Border.diagonal_up` / `Border.diagonal_down`. **Rationale:** reuses the proven
side-parsing accumulator; no new control flow.

### D3 — Rich text: prefer calamine's rich-text variant, fall back to direct `<is>` parsing

`map_data` builds `CellValue` from calamine `Data`. The clean path is to map a
calamine rich-text variant (e.g. `Data::Richtext`) into
`CellValue::rich_text(runs)` with per-run `Font`. **If calamine collapses rich
text to a plain string** (version-dependent), the fallback is to parse
`xl/worksheets/sheetN.xml` `<c><is><r>` inline strings directly — mirroring the
existing `parse_sheet_cell_styles` pass — so recovery does not depend on
calamine's rich-text support. **See Open Questions (Q1).**

## Risks / Trade-offs

- **[Risk] calamine rich-text exposure is version-specific** → Mitigation: confirm
  in a 30-minute spike (Q1); if absent, use the direct `<is>` parse fallback
  (D3). Either way the public `CellValue.rich_text` contract is unchanged.
- **[Risk] `<si>` shared strings with runs** vs **inline `<is>`** are two OOXML
  shapes. → Mitigation: handle both; per-run `rPr` font maps to `RichTextRun.font`
  using the existing `Font` model (name/size/bold/italic/underline/color).
- **[Risk] gradient `degree` vs `angle`** confusion → Mitigation: D1 pins read to
  `gradient_degree`; `gradient_angle` stays dead.
- **[Trade-off]** Three independent parser additions in one release; each is
  independently testable so a regression in one cannot silently break the others.

## Migration Plan

- Read-only additions behind existing code paths; no stored-format or API change.
- Each feature ships with a round-trip fixture (write-by-excelrs → read-back
  assertion). No rollback needed beyond a normal version bump.
- Update the `reader/styles.rs` "v0.3.0 limitations" doc comment (gradient +
  diagonal no longer skipped).

## Open Questions

- **Q1 (spike):** Does the pinned `calamine 0.35` expose rich text as a distinct
  `Data` variant, or does it collapse to `Data::String`? Determines D3 path.
  **Recommended:** verify before task start; the direct-`<is>` fallback is the
  safe default if calamine collapses.
- **Q2:** Should read recover `gradient_angle` if present in source XML, or
  strictly mirror the writer (`gradient_degree` only)? Decision D1 says mirror the
  writer; revisit only if a real file is found that relies on `angle`.
