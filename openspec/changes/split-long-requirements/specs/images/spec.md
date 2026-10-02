## REMOVED Requirements

### Requirement: Worksheet exposes image add/get

**Reason**: Bundled the `addImage`/`getImages` surface, the two anchor variants, anchor-type inference, fractional EMU math, and the TypeScript-versus-runtime `Buffer` typing contract into one 1,419-character requirement.

**Migration**: Replaced by six focused requirements covering the API surface, anchor variants, anchor inference, EMU math, and Buffer typing.

### Requirement: Writer embeds media and emits drawing part

**Reason**: Combined the media/drawing/relationship emission contract with the `oneCellAnchor` ext-child rule and the `twoCellAnchor` shape rule into one over-long requirement.

**Migration**: Replaced by three requirements covering writer emission, the one-cell anchor shape, and the two-cell anchor shape.

## ADDED Requirements

### Requirement: Worksheet exposes addImage and getImages

A `Worksheet` SHALL expose `addImage(opts)` accepting
`{ extension, buffer|stream|path, type: "picture", positioning, anchor }` and returning a
handle, and `getImages()` returning the embedded images.

#### Scenario: addImage returns a handle

- **WHEN** a caller calls `addImage(opts)` with an extension and a buffer, stream, or path
- **THEN** it SHALL return an image handle

#### Scenario: getImages returns embedded images

- **WHEN** a caller calls `getImages()` on a worksheet with added images
- **THEN** it SHALL return the embedded images

### Requirement: Image anchor uses exactly one of br or ext

The `anchor` field SHALL use the ExcelJS shape: a top-left
`tl: { col: number, row: number }` point plus **exactly one** of `br` (a two-cell anchor
spanning tl→br) or `ext` (a one-cell anchor sized in pixels).

#### Scenario: br yields a two-cell anchor

- **WHEN** an anchor supplies `tl` and `br`
- **THEN** the image SHALL span from the top-left to the bottom-right cell

#### Scenario: ext yields a one-cell anchor

- **WHEN** an anchor supplies `tl` and `ext` with `width` and `height`
- **THEN** the image SHALL be a one-cell anchor sized in pixels

### Requirement: Image anchor type is inferred from the fields present

The anchor type — one-cell or two-cell — SHALL be inferred from which field is present
(`ext` versus `br`). There SHALL be no public `anchorType` enum.

#### Scenario: ext presence implies one-cell

- **WHEN** an anchor supplies `ext` and no `br`
- **THEN** it SHALL be treated as a one-cell anchor with no explicit anchor-type field

#### Scenario: br presence implies two-cell

- **WHEN** an anchor supplies `br` and no `ext`
- **THEN** it SHALL be treated as a two-cell anchor

### Requirement: Image anchor col and row accept fractional values

`col` and `row` SHALL be `number` values, and floats SHALL be allowed so sub-cell
positioning is expressible.

#### Scenario: Fractional col positions within a cell

- **WHEN** an anchor supplies `col` with a fractional part
- **THEN** the image SHALL be positioned within that cell rather than snapped to its edge

### Requirement: Fractional anchor coordinates convert to EMU offsets

The fractional part of `col` and `row` SHALL be converted to EMU offsets against default
cell dimensions: `colOff = fract(col) * 64px * 9525` and
`rowOff = fract(row) * 20px * 9525`.

#### Scenario: Fractional col produces a column offset

- **WHEN** an anchor supplies `col` with a fractional part
- **THEN** the emitted column offset SHALL be `fract(col) * 64px * 9525`

#### Scenario: Fractional row produces a row offset

- **WHEN** an anchor supplies `row` with a fractional part
- **THEN** the emitted row offset SHALL be `fract(row) * 20px * 9525`

### Requirement: Image buffer types are declared as Buffer and accepted at runtime

The `buffer` field in `AddImageOptions` and the `buffer` field in `ImageInfo` SHALL be
declared as `Buffer` in the TypeScript type declarations. At runtime, `addImage` SHALL
accept both a Node.js `Buffer` and an `Array<number>`, and `getImages()` SHALL return
`buffer` as a real Node.js `Buffer` whose bytes match what was embedded.

#### Scenario: getImages returns a real Buffer

- **WHEN** an image is added and then read back via `getImages()`
- **THEN** the returned `buffer` SHALL satisfy `Buffer.isBuffer(...) === true`

#### Scenario: Read-back bytes match what was embedded

- **WHEN** an image is added from a byte buffer and read back via `getImages()`
- **THEN** the returned bytes SHALL equal the embedded bytes

#### Scenario: addImage accepts a plain array at runtime

- **WHEN** a caller passes `buffer` as an `Array<number>` to `addImage`
- **THEN** the image SHALL be embedded successfully

### Requirement: Writer embeds media and emits drawing part with relationships

When a worksheet has images, the writer SHALL write the bytes to `xl/media/imageM.<ext>`,
emit `xl/drawings/drawingN.xml` with a `<oneCellAnchor>`/`<twoCellAnchor>` referencing the
media, and register both a `drawing` relationship (sheet `.rels` → drawing part) and an
`image` relationship (drawing `.rels` → media).

#### Scenario: Worksheet with images emits media, drawing, and both relationships

- **WHEN** a worksheet with images is written
- **THEN** the writer SHALL emit the media part, a drawing part referencing it, and the drawing and image relationships

#### Scenario: Worksheet without images emits no drawing or media

- **WHEN** a worksheet with no images is written
- **THEN** it SHALL NOT emit a drawing part or a media part

### Requirement: oneCellAnchor always contains an ext child

A `oneCellAnchor` SHALL always contain a `<xdr:ext cx=".." cy=".."/>` child (ECMA-376
20.5.2.27), even when the internal `ext_size` is absent, so the output is schema-valid and
openable by Excel.

#### Scenario: Missing internal ext size still emits ext

- **WHEN** a `oneCellAnchor` is written and the internal `ext_size` is absent
- **THEN** the emitted anchor SHALL still contain an `<xdr:ext cx=".." cy=".."/>` child

### Requirement: twoCellAnchor contains a to child and no ext

A `twoCellAnchor` SHALL contain a `<xdr:to>` child and SHALL NOT contain an `<xdr:ext>`
child.

#### Scenario: twoCellAnchor emits to and omits ext

- **WHEN** a `twoCellAnchor` is written
- **THEN** it SHALL contain an `<xdr:to>` child and no `<xdr:ext>` child