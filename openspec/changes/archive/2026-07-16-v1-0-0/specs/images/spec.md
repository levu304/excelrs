## ADDED Requirements

### Requirement: Worksheet exposes image add/get

A `Worksheet` SHALL expose `addImage(opts)` accepting `{ extension,
buffer|stream|path, type: "picture", positioning, anchor }` and returning a
handle, and `getImages()` returning the embedded images. Anchor SHALL support
one-cell (`{ col, row, x, y }`) and two-cell (`{ tl: {...}, br: {...} }`)
positioning.

#### Scenario: Add an image

- **WHEN** `ws.addImage({ extension: "png", buffer: <bytes>, type: "picture", positioning: "oneCell", anchor: { col: 1, row: 1, x: 0, y: 0 } })`
- **THEN** `ws.getImages().length === 1` and the returned image reports `extension === "png"` and matches the bytes

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
