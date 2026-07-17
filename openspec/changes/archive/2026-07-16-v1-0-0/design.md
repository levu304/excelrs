# Design: v1.0.0 — Drop-in ExcelJS-compat milestone

## Context

`excelrs` is a Rust (napi) + Node native addon porting the ExcelJS API. Each
release is one OpenSpec change; v0.13.0 (package `0.13.0`) is current. The
parity matrix still lists five areas as `planned` and unimplemented:
headers/footers, page setup / print, workbook views & calc, comments, images.

The codebase already proves the pattern this change reuses:

- **Model types** live in `src/model/*.rs` and bridge to JS/TS via napi
  (e.g. `cell.rs`, `sheet_protection.rs`).
- **Worksheet element reader/writer** loops live in `src/reader/xlsx.rs` and
  `src/writer/xlsx.rs`; whole new parts (rels, media) are registered by the
  workbook reader/writer in `src/reader/workbook.rs` and `src/writer/xlsx.rs`.
- **Relationships** use an existing rels manager (hyperlinks already resolve
  `r:id` → URL via sheet rels), so comments/drawings reuse the same plumbing.

This change is **additive** — no existing API or behavior changes; the major
version bump signals the compatibility milestone.

## Goals / Non-Goals

**Goals**

- Deliver all five areas as full read/write round-trips matching ExcelJS
  behavior, so `excelrs` satisfies the drop-in-compat promise for those areas.
- Reuse the existing model → reader → writer → napi-bridge structure; no new
  architectural substrate.
- Advance the `exceljs-parity` matrix for the five areas from `planned` to
  `shipped`.

**Non-Goals** (see proposal): charts, pivot tables, tables, conditional
formatting, formula evaluation, streaming XLSX, and worksheet state/tab
color/outline levels.

## Decisions

### D1 — Whole new parts reuse the rels manager (comments, drawings)

Comments and images require new OOXML parts with relationships:

- `xl/worksheets/_rels/sheetN.xml.rels` gains a `comments` relationship →
  `../commentsN.xml`.
- `xl/worksheets/_rels/sheetN.xml.rels` gains a `drawing` relationship →
  `../drawings/drawingN.xml`; the drawing part gains a relationship to
  `../media/imageM.png`.

**Decision:** emit these through the existing rels manager rather than a
separate mechanism, mirroring how hyperlinks already attach `r:id` rels to a
sheet. **Alternative considered:** inline media as base64 — rejected (bloats
the zip and breaks Excel interop). **Alternative:** a generic "extra parts"
registry — rejected as over-abstraction for two part types.

### D2 — Headers/footers and page setup are worksheet elements, not new parts

Both live inside `xl/worksheets/sheetN.xml` (`<headerFooter>` and
`<pageMargins>`/`<pageSetup>`). They follow the same reader/writer insertion
points as `worksheet-views` (after `<dimension>`, ordered per CT_Worksheet).

**Decision:** implement as parsed worksheet model fields + element emitters, no
new part. Format codes (`&L`, `&C`, `&R`, `&[Page]`, `&[Date]`, …) are stored
**verbatim** as strings — ExcelJS does not parse them, and neither do we
(ponytail: no speculative format-code interpreter).

### D3 — Workbook views & calc are workbook-level elements

`<bookViews>` (`<workbookView xWindow yWindow … activeTab state>`) and
`<calcPr fullCalcOnLoad="1"/>` live in `xl/workbook.xml`. **Decision:** model
them on the `Workbook` (read/write in the workbook reader/writer), matching
ExcelJS `wb.views` (array) and `wb.calcProperties`.

### D4 — Comments model: one `Note` per anchored cell

ExcelJS exposes `cell.note` (legacy comment) / `cell.comment`. **Decision:**
model a `Comment { text, author, ... }` on the cell, serialize to
`xl/commentsN.xml` (`<commentList><comment ref="A1" authorId="0"><text>…`)
with a `commentsN.xml.rels`-free author table. Read-back populates `cell.note`.

### D5 — Images model: anchor + media bytes

**Decision:** `ws.addImage({ extension, buffer/stream, type:"picture",
positioning, anchor })` writes bytes to `xl/media/imageM.<ext>` and emits a
`<oneCellAnchor>` (or `<twoCellAnchor>`) in the sheet's drawing part.
Read-back parses the drawing part, resolves the media rel, and returns
`{ extension, buffer }`. Rich anchor math (editAs, from/to col/row offsets) is
kept minimal — one-cell and two-cell anchors only; no rotation/cropping.

### D6 — Round-trip fidelity is the acceptance bar

Each feature's correctness is proven by a read→write→read fixture, matching
the approach used by `rich-text`, `hyperlinks`, and `worksheet-views`. No
feature ships without a fixture exercising Excel-authored and ExcelJS-authored
inputs.

## Risks / Trade-offs

- **Drawing/media plumbing** (D1/D5) is the largest surface — new parts, rels,
  and binary media handling. Mitigated by reusing the rels manager.
- **Anchor fidelity** for images is intentionally limited (D5) — complex
  `editAs`/rotation cases are out of scope; acceptable for v1.0.0.
- **Format-code passthrough** (D2) means we do not validate header/footer code
  syntax; matches ExcelJS behavior, but malformed codes pass through unchanged.
- **Version bump to 1.0.0** is semantic, not behavioral — downstream consumers
  pinning `^0.13` will not auto-upgrade; that is intended (major signals the
  milestone).

## Migration Plan

1. Implement features behind the existing model/reader/writer/bridge structure.
2. Add round-trip fixtures per feature under `fixtures/`/`__test__/`.
3. Bump `package.json` `version` `0.13.0` → `1.0.0` and update `CHANGELOG.md`.
4. Update `ROADMAP.md` parity matrix (the `exceljs-parity` spec delta records
   this) so the five areas read `shipped`.
5. No migration shim needed — API is strictly additive.

## Open Questions

- Should `cell.comment` (new-style thread) and `cell.note` (legacy) both be
  exposed, or only `note` for v1.0.0? (Recommend `note` only; thread comments
  deferred.)
- Image `positioning` (`oneCell`/`absolute`/`twoCell`) — confirm ExcelJS enum
  names before finalizing the public API in tasks.
