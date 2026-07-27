## MODIFIED Requirements

### Requirement: Worksheet exposes image add/get

A `Worksheet` SHALL expose `addImage(opts)` accepting
`{ extension, buffer|stream|path, type: "picture", positioning, anchor }`
returning a handle, and `getImages()` returning embedded images.

The `anchor` field SHALL use the ExcelJS shape: a top-left `tl: { col: number, row: number }`
point plus **exactly one** of:

- `br: { col: number, row: number }` → a two-cell anchor (image spans tl→br), OR
- `ext: { width: number, height: number }` → a one-cell anchor sized in pixels.

The anchor type (one-cell / two-cell) SHALL be **inferred** from which field is
present; there SHALL be no public `anchorType` enum. `col`/`row` SHALL be
`number` (floats allowed) so sub-cell positioning is expressible.

`col`/`row` SHALL support fractional values; the fractional part SHALL be
converted to EMU offsets against default cell dimensions
(`colOff = fract(col) * 64px * 9525`, `rowOff = fract(row) * 20px * 9525`).

The `buffer` field in `AddImageOptions` SHALL be declared as `Buffer` (not
`Array<number>`) in TypeScript type declarations. `buffer` in `ImageInfo` SHALL
likewise be declared as `Buffer`. At runtime, `addImage` SHALL accept both
Node.js `Buffer` and `Array<number>` for `buffer`. `getImages()` SHALL return a
real Node.js `Buffer`.

#### Scenario: Add image with two-cell anchor (ExcelJS shape)

- **WHEN** `ws.addImage({ extension: "png", buffer: <Buffer>, type: "picture", positioning: "oneCell", anchor: { tl: { col: 0, row: 0 }, br: { col: 5.5, row: 2.2 } } })`
- **THEN** `ws.getImages().length === 1`, returned image reports `extension === "png"` and `buffer` is a `Buffer` matching input bytes; a `<twoCellAnchor>` is emitted with `colOff`/`rowOff` derived from the fractional parts (`0.5*64*9525`, `0.2*20*9525`).

#### Scenario: Add image with one-cell anchor + explicit size

- **WHEN** `ws.addImage({ extension: "png", buffer: <Buffer>, type: "picture", anchor: { tl: { col: 1, row: 1 }, ext: { width: 120, height: 60 } } })`
- **THEN** a `<oneCellAnchor>` is emitted with `<xdr:from>` at `col=1,row=1` and `<xdr:ext cx="1143000" cy="571500"/>` (pixels × 9525 EMU).

#### Scenario: Add image number[] (backward compat)

- **WHEN** `ws.addImage({ extension: "png", buffer: [1, 2, 3], type: "picture", anchor: { tl: { col: 1, row: 1 }, br: { col: 3, row: 3 } } })`
- **THEN** call does NOT throw at runtime; image stored correctly.

#### Scenario: AddImageOptions.anchor accepts ExcelJS shape in TS

- **WHEN** a TypeScript project calls `ws.addImage({ extension: "png", buffer: <Buffer>, anchor: { tl: { col: 0, row: 0 }, br: { col: 5.5, row: 2.2 } } })`
- **THEN** TypeScript does NOT report TS2739 / TS2345; no `anchorType` field is required.

#### Scenario: Missing br and ext is rejected

- **WHEN** `ws.addImage({ extension: "png", buffer: <Buffer>, anchor: { tl: { col: 0, row: 0 } } })`
- **THEN** the call returns an `Err` (or throws) stating that exactly one of `br`/`ext` is required; no image is added.

#### Scenario: getImages returns ExcelJS-shaped anchor

- **WHEN** an image anchored at `{ tl: { col: 2, row: 3 }, br: { col: 6, row: 8 } }` is read back
- **THEN** `ws.getImages()[0].anchor` reports `tl: { col: 2, row: 3 }` and `br: { col: 6, row: 8 }` (within float precision).
