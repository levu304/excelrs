## Why

The reader drops one-cell image size on read. `parse_drawing_xml` never parses
`<xdr:ext cx=".." cy=".."/>`, so `WorksheetImage.ext_size` is hardcoded `None`.
Two consequences:

1. `getImages()` returns `{ tl, br: None, ext: None }` — an **invalid** anchor
   per the new "exactly one of `br`/`ext`" contract introduced by PR #39, and
   re-adding it via `addImage` returns `Err`.
2. Re-saving emits a `<xdr:oneCellAnchor>` with no `<xdr:ext>` — **invalid OOXML**
   (ECMA-376 20.5.2.27 requires `ext` in `oneCellAnchor`); Excel rejects the file.

Any real `.xlsx` with a one-cell image (Excel, ExcelJS, or excelrs v2.4+) loses
the image's size on round-trip, and the re-saved file is corrupt.

## What Changes

- Reader parses `<xdr:ext cx=".." cy=".."/>` inside `parse_drawing_xml` and
  threads the size through as `ext_size` (replacing the hardcoded `None`).
- Writer always emits `<xdr:ext>` for `oneCellAnchor`, falling back to
  `cx="0" cy="0"` when `ext_size` is `None`, so the output is always schema-valid.
- No public API changes: `getImages()` and `addImage()` keep their current
  `ImageAnchorInput` shapes; the fix makes the round-trip actually preserve size.

Not breaking — purely corrects internal read/emit of an existing field.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `images`: requirement "Round-trip preserves anchor" now also requires one-cell
  (`ext`) anchors to preserve their pixel size across read → write, and
  `getImages()` to return a valid `ext`-shaped anchor for one-cell images read
  from a file.

## Impact

- `src/reader/xlsx.rs`: `parse_drawing_xml` return type gains `ext_size`; caller
  assigns it instead of `None`.
- `src/writer/xlsx.rs`: `write_drawing_xml` emits `<xdr:ext>` unconditionally for
  `oneCellAnchor`.
- `src/model/image.rs`: `to_exceljs_shape` already returns the correct `ext` when
  `ext_size` is `Some`; no change needed beyond confirming the path.
- Tests: add a round-trip test for a one-cell anchor with explicit size read
  back from a written file.
