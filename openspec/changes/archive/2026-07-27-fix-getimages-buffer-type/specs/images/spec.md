## MODIFIED Requirements

### Requirement: Worksheet exposes image add/get

A `Worksheet` SHALL expose `addImage(opts)` accepting `{ extension, buffer|stream|path, type: "picture", positioning, anchor }` and returning a handle, and `getImages()` returning the embedded images. Anchor SHALL support one-cell (`{ col, row, x, y }`) and two-cell (`{ tl: {...}, br: {...} }`) positioning.

The `buffer` field in `AddImageOptions` SHALL be declared as `Buffer` (not `Array<number>`) in the TypeScript type declarations. The `buffer` field in `ImageInfo` SHALL likewise be declared as `Buffer`. At runtime, `addImage` SHALL accept both Node.js `Buffer` and `Array<number>` for the `buffer` field. At runtime, `getImages()` SHALL return `buffer` as a real Node.js `Buffer` (i.e. `Buffer.isBuffer(...) === true`) whose bytes match what was embedded.

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
