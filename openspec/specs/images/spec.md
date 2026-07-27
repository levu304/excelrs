# images Specification

## Purpose

TBD - created by archiving change v1-0-0. Update Purpose after archive.

## Requirements

### Requirement: Worksheet exposes image add/get

A `Worksheet` SHALL expose `addImage(opts)` accepting `{ extension,
buffer|stream|path, type: "picture", positioning, anchor }` and returning a
handle, and `getImages()` returning the embedded images. Anchor SHALL support
one-cell (`{ col, row, x, y }`) and two-cell (`{ tl: {...}, br: {...} }`)
positioning.

The `buffer` field in `AddImageOptions` SHALL be declared as `Buffer` (not
`Array<number>`) in the TypeScript type declarations. The `buffer` field in
`ImageInfo` SHALL likewise be declared as `Buffer`. At runtime, `addImage`
SHALL accept both Node.js `Buffer` and `Array<number>` for the `buffer` field.
At runtime, `getImages()` SHALL return `buffer` as a real Node.js `Buffer`
(i.e. `Buffer.isBuffer(...) === true`) whose bytes match what was embedded.

#### Scenario: Add an image with Buffer

- **WHEN** `ws.addImage({ extension: "png", buffer: <Buffer>, type: "picture", positioning: "oneCell", anchor: { col: 1, row: 1, x: 0, y: 0 } })`
- **THEN** `ws.getImages().length === 1` and the returned image reports `extension === "png"` and `buffer` is a `Buffer` matching the input bytes

#### Scenario: Add an image with number[] (backward compat)

- **WHEN** `ws.addImage({ extension: "png", buffer: [1, 2, 3], type: "picture", positioning: "oneCell", anchor: { col: 1, row: 1, x: 0, y: 0 } })`
- **THEN** the call does NOT throw at runtime and the image is stored correctly

#### Scenario: AddImageOptions.buffer accepts Buffer type in TS

- **WHEN** a TypeScript project calls `ws.addImage({ ... buffer: x })` where `x` is a Node.js `Buffer`
- **THEN** the TypeScript compiler does NOT emit type error TS2739 or TS2345 about `buffer`

#### Scenario: getImages returns buffer as Buffer type

- **WHEN** a TypeScript project calls `ws.getImages()[0].buffer`
- **THEN** the TypeScript compiler resolves the type as `Buffer`

#### Scenario: getImages returns buffer as a real Buffer at runtime

- **WHEN** `ws.addImage({ extension: "png", buffer: <Buffer>, type: "picture", positioning: "oneCell", anchor: { col: 1, row: 1, x: 0, y: 0 } })` then `ws.getImages()`
- **THEN** `Buffer.isBuffer(ws.getImages()[0].buffer)` is `true` and its bytes equal the input bytes

### Requirement: Writer embeds media and emits drawing part

When a worksheet has images, the writer SHALL write the bytes to
`xl/media/imageM.<ext>`, emit `xl/drawings/drawingN.xml` with a
`<oneCellAnchor>`/`<twoCellAnchor>` referencing the media, and register both a
`drawing` relationship (sheet `.rels` → drawing part) and an `image`
relationship (drawing `.rels` → media). A sheet without images SHALL NOT emit a
drawing part or media.

#### Scenario: Emit drawing and media

- **WHEN** an image is added to a sheet
- **THEN** `xl/media/image1.png` exists, `xl/drawings/drawingN.xml` contains an anchor referencing it, and the sheet `.rels` has a `drawing` relationship

#### Scenario: No images omits drawing

- **WHEN** a worksheet has no images
- **THEN** no `xl/drawings/drawingN.xml` or `xl/media/` entry is emitted for it

### Requirement: Reader parses drawing part and media

The reader SHALL parse each sheet's drawing part (resolved via the sheet
`.rels`), resolve media rels to `xl/media/`, and populate `ws.getImages()`
with `{ extension, buffer }` and anchor metadata. A sheet without a drawing
relationship SHALL report no images.

#### Scenario: Read image bytes back

- **WHEN** a PNG was embedded and the workbook is read back
- **THEN** `ws.getImages()[0].extension === "png"` and its `buffer` equals the originally embedded bytes

#### Scenario: Round-trip preserves anchor

- **WHEN** an image was anchored at `{ col: 2, row: 3 }` and the workbook is read back
- **THEN** the read-back image's anchor reports `col === 2` and `row === 3`
