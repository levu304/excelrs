# Proposal: v1.0.0 — Drop-in ExcelJS-compat milestone

## Why

`excelrs` is a Rust + Node native addon that ports the ExcelJS API surface. The
roadmap (ROADMAP.md) defines v1.0.0 as the major version that closes the
**drop-in ExcelJS compatibility promise**: once v1.0.0 ships, the remaining
unshipped medium-effort parity features are delivered and `excelrs` can be
advertised as a compatible ExcelJS replacement for the covered feature areas.

v0.13.0 (latest, current package version `0.13.0`) already shipped date
preservation and theme-color write. The parity matrix still lists five feature
areas as `planned` and absent from the implemented API: **headers/footers**,
**page setup / print**, **workbook views & calc properties**, **comments**, and
**images / drawings**. These are self-contained OOXML additions — each is a
bounded part or element with a moderate, well-understood effort — and they are
the compatibility gaps most likely to break a real ExcelJS workload migrating
to `excelrs`.

## What Changes

This change bundles the five remaining medium-effort parity features into a
single v1.0.0 release, each delivered as a full read/write round-trip:

| Area | OOXML surface | ExcelJS API |
| ------ | --------------- | ------------- |
| Headers & footers | `<headerFooter>` (`oddHeader`/`oddFooter`/even/first) + format codes | `ws.headerFooter` |
| Page setup / print | `<pageMargins>`, `<pageSetup>` (`orientation`, `paperSize`, `fitTo*`), `printArea`, `printTitles` | `ws.pageSetup` |
| Workbook views & calc | `<bookViews>` (`<workbookView>`), `<calcPr fullCalcOnLoad>` | `wb.views`, `wb.calcProperties` |
| Comments | `xl/commentsN.xml` + relationship, `<commentList>/<comment>` | `cell.note`, `cell.comment` |
| Images / drawings | `xl/drawings/drawingN.xml` + `xl/media/`, anchor `<oneCellAnchor>` | `ws.addImage`, `ws.getImages` |

Each area is implemented as: model type(s) → reader parse → writer emit → public
JS/TS API bridge, mirroring the pattern already proven by `rich-text`,
`hyperlinks`, `gradient-fill`, and `worksheet-views`.

## Impact

- **New public API** on `Worksheet`: `headerFooter`, `pageSetup`. On `Workbook`:
  `views`, `calcProperties`. On cells: comment access. On `Worksheet`:
  `addImage` / `getImages`.
- **New OOXML parts** emitted by the writer: `xl/commentsN.xml` (+ sheet rels)
  and `xl/drawings/drawingN.xml` (+ `xl/media/` + rels).
- **Reader** must parse the new worksheet elements, the workbook-level
  `bookViews`/`calcPr`, comment parts, and drawing parts.
- **Package version** advances `0.13.0` → `1.0.0` (semver major).
- **No breaking changes** to already-shipped API; this is additive. The major
  bump signals the compatibility milestone, not removed surface.

## Capabilities

### New Capabilities

- `headers-footers` — worksheet header/footer read/write with `&` format codes (`specs/headers-footers/spec.md`)
- `page-setup` — page margins, page setup, print area & print titles read/write (`specs/page-setup/spec.md`)
- `workbook-views` — workbook `<bookViews>` and `<calcPr>` read/write (`specs/workbook-views/spec.md`)
- `comments` — cell comments via `xl/commentsN.xml` part + relationship (`specs/comments/spec.md`)
- `images` — image embedding via `xl/drawings/` + `xl/media/` (`specs/images/spec.md`)

### Modified Capabilities

- `exceljs-parity` — advance `comments`, `images`, `page setup / print`, `headers/footers`, and `workbook views & properties` from `planned` to `shipped` in the parity matrix (`specs/exceljs-parity/spec.md`)

## Non-Goals

Explicitly **out of scope** for v1.0.0 (carried on the roadmap as post-v1 / v2):

- Charts, pivot tables, tables, conditional formatting (heavy subsystems).
- Formula evaluation (separate interpreter).
- Streaming XLSX (separate reader/writer architecture).
- Worksheet-level `state` (visible/hidden), `tabColor`, `rowBreaks`/`colBreaks`,
  outline levels, and sheet `properties` (defaultRowHeight etc.) — these are
  `planned` but not selected for this milestone.
- Image *editing*/manipulation beyond embed + read-back; only anchor placement
  and round-trip fidelity are covered.
