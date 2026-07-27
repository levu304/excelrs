# images Specification

## Purpose

TBD - created by archiving change v1-0-0. Update Purpose after archive.

## Requirements

### Requirement: Worksheet exposes image add/get

A `Worksheet` SHALL expose `addImage(opts)` accepting
`{ extension, buffer|stream|path, type: "picture", positioning, anchor }`
returning a handle, and `getImages()` returning the embedded images.

The `anchor` field SHALL use the ExcelJS shape: a top-left
`tl: { col: number, row: number }` point plus **exactly one** of:

- `br: { col: number, row: number }` → a two-cell anchor (image spans tl→br), OR
- `ext: { width: number, height: number }` → a one-cell anchor sized in pixels.

The anchor type (one-cell / two-cell) SHALL be **inferred** from which field
is present; there SHALL be no public `anchorType` enum. `col`/`row` SHALL be
`number` (floats allowed) so sub-cell positioning is expressible.
`col`/`row` SHALL support fractional values; the fractional part SHALL be
converted to EMU offsets against default cell dimensions
(`colOff = fract(col) * 64px * 9525`, `rowOff = fract(row) * 20px * 9525`).

The `buffer` field in `AddImageOptions` SHALL be declared as `Buffer` (not
`Array<number>`) in the TypeScript type declarations. The `buffer` field in
`ImageInfo` SHALL likewise be declared as `Buffer`. At runtime, `addImage`
SHALL accept both Node.js `Buffer` and `Array<number>` for the `buffer` field.
At runtime, `getImages()` SHALL return `buffer` as a real Node.js `Buffer`
(i.e. `Buffer.isBuffer(...) === true`) whose bytes match what was embedded.

#### Scenario: Add image with two-cell anchor (ExcelJS shape)

- **WHEN** `ws.addImage({ extension: "png", buffer: <Buffer>, type: "picture", positioning: "oneCell", anchor: { tl: { col: 0, row: 0 }, br: { col: 5.5, row: 2.2 } } })`
- **THEN** `ws.getImages().length === 1`, returned image reports `extension === "png"` and `buffer` is a `Buffer` matching input bytes; a `<twoCellAnchor>` is emitted with `colOff`/`rowOff` derived from the fractional parts (`0.5*64*9525`, `0.2*20*9525`).

#### Scenario: Add image with one-cell anchor + explicit size

- **WHEN** `ws.addImage({ extension: "png", buffer: <Buffer>, type: "picture", anchor: { tl: { col: 1, row: 1 }, ext: { width: 120, height: 60 } } })`
- **THEN** a `<oneCellAnchor>` is emitted with `<xdr:from>` at `col=1,row=1` and `<xdr:ext cx="1143000" cy="571500"/>` (pixels × 9525 EMU).

#### Scenario: Add image with number[] (backward compat)

- **WHEN** `ws.addImage({ extension: "png", buffer: [1, 2, 3], type: "picture", anchor: { tl: { col: 1, row: 1 }, br: { col: 3, row: 3 } } })`
- **THEN** the call does NOT throw at runtime and the image is stored correctly

#### Scenario: AddImageOptions.anchor accepts ExcelJS shape in TS

- **WHEN** a TypeScript project calls `ws.addImage({ extension: "png", buffer: <Buffer>, anchor: { tl: { col: 0, row: 0 }, br: { col: 5.5, row: 2.2 } } })`
- **THEN** the TypeScript compiler does NOT emit type error TS2739 or TS2345; no `anchorType` field is required.

#### Scenario: Missing br and ext is rejected

- **WHEN** `ws.addImage({ extension: "png", buffer: <Buffer>, anchor: { tl: { col: 0, row: 0 } } })`
- **THEN** the call returns an `Err` (or throws) stating that exactly one of `br`/`ext` is required; no image is added.

#### Scenario: getImages returns buffer as Buffer type

- **WHEN** a TypeScript project calls `ws.getImages()[0].buffer`
- **THEN** the TypeScript compiler resolves the type as `Buffer`

#### Scenario: getImages returns buffer as a real Buffer at runtime

- **WHEN** `ws.addImage({ extension: "png", buffer: <Buffer>, type: "picture", anchor: { tl: { col: 1, row: 1 }, br: { col: 3, row: 3 } } })` then `ws.getImages()`
- **THEN** `Buffer.isBuffer(ws.getImages()[0].buffer)` is `true` and its bytes equal the input bytes

#### Scenario: getImages returns ExcelJS-shaped anchor

- **WHEN** an image anchored at `{ tl: { col: 2, row: 3 }, br: { col: 6, row: 8 } }` is read back
- **THEN** `ws.getImages()[0].anchor` reports `tl: { col: 2, row: 3 }` and `br: { col: 6, row: 8 }` (within float precision).

For a one-cell anchor with explicit `ext` (`{ tl, ext: { width, height } }`),
the pixel size SHALL be preserved through read and write: the emitted
`<xdr:ext cx="W*9525" cy="H*9525"/>` SHALL be parsed on read and returned as
`ext: { width, height }` by `getImages()`.

#### Scenario: One-cell anchor size survives read-back

- **WHEN** an xlsx written with `anchor: { tl: { col: 1, row: 1 }, ext: { width: 120, height: 60 } }` is read back
- **THEN** `getImages()[0].anchor` reports `tl: { col: 1, row: 1 }` and `ext: { width: 120, height: 60 }` (rounded to 4 dp), and re-writing emits `<xdr:ext cx="1143000" cy="571500"/>`.

#### Scenario: getImages returns valid shape for one-cell read from file

- **WHEN** a `oneCellAnchor` with `<xdr:ext>` is read from a file
- **THEN** `getImages()[0].anchor` has `ext` set and `br` unset (valid per the "exactly one of br/ext" contract), never `br: None, ext: None`.

### Requirement: Writer embeds media and emits drawing part

When a worksheet has images, the writer SHALL write the bytes to
`xl/media/imageM.<ext>`, emit `xl/drawings/drawingN.xml` with a
`<oneCellAnchor>`/`<twoCellAnchor>` referencing the media, and register both a
`drawing` relationship (sheet `.rels` → drawing part) and an `image`
relationship (drawing `.rels` → media). A sheet without images SHALL NOT emit a
drawing part or media. A `oneCellAnchor` SHALL always contain a
`<xdr:ext cx=".." cy=".."/>` child (ECMA-376 20.5.2.27) — even when the
internal `ext_size` is absent — so the output is schema-valid and openable by
Excel. A `twoCellAnchor` SHALL contain a `<xdr:to>` child and no `<xdr:ext>`.

#### Scenario: Emit drawing and media

- **WHEN** an image is added to a sheet
- **THEN** `xl/media/image1.png` exists, `xl/drawings/drawingN.xml` contains an anchor referencing it, and the sheet `.rels` has a `drawing` relationship

#### Scenario: No images omits drawing

- **WHEN** a worksheet has no images
- **THEN** no `xl/drawings/drawingN.xml` or `xl/media/` entry is emitted for it

#### Scenario: Writer emits ext for one-cell even without explicit size

- **WHEN** a `oneCellAnchor` is written with `ext_size: None`
- **THEN** the emitted XML contains `<xdr:ext cx="0" cy="0"/>` (fallback), so the file remains valid OOXML.

### Requirement: Reader parses drawing part and media

The reader SHALL parse each sheet's drawing part (resolved via the sheet
`.rels`), resolve media rels to `xl/media/`, and populate `ws.getImages()`
with `{ extension, buffer }` and anchor metadata. A sheet without a drawing
relationship SHALL report no images.

#### Scenario: Read image bytes back

- **WHEN** a PNG was embedded and the workbook is read back
- **THEN** `ws.getImages()[0].extension === "png"` and its `buffer` equals the originally embedded bytes

#### Scenario: Round-trip preserves anchor

- **WHEN** an image was anchored at `{ tl: { col: 2, row: 3 }, br: { col: 6, row: 8 } }` and the workbook is read back
- **THEN** the read-back image's anchor reports `tl.col === 2`, `tl.row === 3`, `br.col === 6`, and `br.row === 8`
