## Why

Users migrating from ExcelJS hit a TypeScript error on `worksheet.addImage(...)`:

```ts
worksheet.addImage({ buffer, extension: "png" });
//        ^ Property 'anchor' is missing — it is required in AddImageOptions
```

ExcelJS never requires an anchor at registration time. Its anchor shape is
`{ tl: { col, row }, br: { col, row } }` (or `{ tl, ext }`), with **fractional**
col/row allowed (e.g. `col: 5.5`). excelrs instead exposes a flat OOXML-shaped
struct with a required `anchorType` enum and eight numeric fields
(`col, row, x, y, col2, row2, x2, y2`) — an internal serialization detail leaked
into the public API.

The divergence is undocumented:

- `openspec/specs/images/spec.md` already states the anchor SHALL support
  one-cell (`{ col, row, x, y }`) **and** two-cell (`{ tl: {...}, br: {...} }`)
  positioning — but the implementation does not match this.
- `README.md` shows `ws.addImage({ extension: 'png', buffer, anchor: { col: 1, row: 1 } })`,
  which does not compile against the actual `ImageAnchor` type (missing 7 fields
  - `anchorType`).

So both the public shape and the docs are wrong relative to the spec and to
ExcelJS. This change fixes the public anchor shape to match ExcelJS and brings
the README + spec back into agreement.

## What Changes

- **Public anchor shape** (`AddImageOptions.anchor`, `ImageInfo.anchor`) becomes
  the ExcelJS shape: a `tl` point plus either a `br` point (→ twoCell) or an
  `ext` size (→ oneCell). `anchorType` enum is removed from the public API and
  **inferred** from which field is present. `col`/`row` are numbers (floats
  allowed) instead of `u32`.
- **Fractional positioning** is preserved: the fractional part of `col`/`row` is
  converted to EMU offsets (`colOff`/`rowOff`) using default cell dimensions,
  matching ExcelJS semantics.
- **Internal model unchanged**: the Rust `ImageAnchor` (flat `{ col, row, x, y,
  col2, ... }` + `AnchorType`) stays as the OOXML serialization form. A
  conversion layer maps the public ExcelJS shape → internal flat struct at the
  FFI boundary. Writer/reader parsing code is untouched.
- **`getImages()`** returns `anchor` in the same ExcelJS shape (round-trip
  consistency).
- **README** updated with a compiling example and a note on the twoCell/oneCell
  inference + fractional behavior.

## Capabilities

### New Capabilities

- *(none — no new worksheet/image capabilities; this reshapes an existing one)*

### Modified Capabilities

- `images` — the `Worksheet exposes image add/get` requirement's anchor clause
  changes from the flat OOXML struct to the ExcelJS `{ tl, br }` / `{ tl, ext }`
  shape, with inferred anchor type and fractional col/row support.

## Impact

- `src/model/image.rs` — new public input types (`ImageAnchorInput`,
  `AnchorPoint`, `ImageSize`); `AddImageOptions.anchor` / `ImageInfo.anchor`
  retyped; conversion helper `ImageAnchorInput → ImageAnchor`.
- `src/model/worksheet.rs` — `add_image()` converts the input anchor before
  storing; `get_images()` converts back.
- `src/writer/xlsx.rs` — emit `<xdr:ext>` for oneCell when an explicit size is
  given (previously hardcoded `cx=0 cy=0`).
- `index.d.ts` / `native.d.ts` — regenerated from the new `#[napi(object)]`
  types.
- `README.md` — corrected, compiling `addImage` example + anchor-shape note.
- `openspec/specs/images/spec.md` — anchor requirement modernized to the
  ExcelJS shape with fractional + inference scenarios.
