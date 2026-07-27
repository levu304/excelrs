## Context

excelrs's internal `ImageAnchor` is a faithful copy of the OOXML drawing XML:
`{ anchorType, col, row, x, y, col2, row2, x2, y2 }` where `x/y/x2/y2` are EMU
offsets and everything is `u32`. `write_drawing_xml` and `parse_drawing_xml`
read/write exactly that shape. The public TypeScript `AddImageOptions.anchor`
exposes this same struct verbatim, so users must hand-split a position like
`"col 5.5, row 2.2"` into `col=5, col2=..., x2=<EMU>, ...` and pick an
`anchorType` enum — work ExcelJS does for them behind `{ tl, br }`.

ExcelJS anchor model:

- `tl: { col: number, row: number }` — top-left, **fractional** col/row allowed.
- `br: { col, row }` → a **twoCell** anchor spanning tl→br.
- `ext: { width, height }` → a **oneCell** anchor sized in pixels.
- No `anchorType` — derived from `br` vs `ext`.

The spec already demands this shape; only the implementation + README diverged.

## Goals / Non-Goals

**Goals:**

- Public `anchor` matches ExcelJS: `tl` + (`br` | `ext`), fractional col/row,
  no `anchorType` on the public type.
- No behavioral change to the bytes written/read — OOXML stays identical.
- `getImages()` returns the same ExcelJS shape so round-trips are ergonomic.
- README example compiles and documents the inference + fractional rules.

**Non-Goals:**

- No `Workbook.addImage` / two-step image-ID registry (separate concern; README
  already documents the Worksheet-only placement choice).
- No change to internal `ImageAnchor` storage or the reader/writer XML parsing.
- No chart / background-image support.

## Decisions

- **Boundary conversion, not internal rewrite.** Keep Rust `ImageAnchor` (flat
  OOXML fields) as the storage/serialization form. Add a separate public input
  type the `#[napi(object)]` derives from, and convert in `add_image()` /
  `get_images()`. Writer/reader parsing code is untouched. This is the smallest
  diff and keeps the OOXML model honest.

  ```rust
  #[napi(object)]
  pub struct AnchorPoint { pub col: f64, pub row: f64 }
  #[napi(object)]
  pub struct ImageSize { pub width: f64, pub height: f64 }
  #[napi(object)]
  pub struct ImageAnchorInput {
      pub tl: AnchorPoint,
      pub br: Option<AnchorPoint>,
      pub ext: Option<ImageSize>,
  }
  ```

- **Fractional → EMU split** uses ExcelJS defaults so behavior matches a
  migration 1:1:
  - `EMU_PER_PX = 9525`
  - `DEFAULT_COL_WIDTH_PX = 64`, `DEFAULT_ROW_HEIGHT_PX = 20`
  - `col_off = fract(tl.col) * DEFAULT_COL_WIDTH_PX * EMU_PER_PX`
  - `row_off = fract(tl.row) * DEFAULT_ROW_HEIGHT_PX * EMU_PER_PX`
  - Same for `br` → `col2/row2/x2/y2`. Integer part → `col`/`row`.

- **Anchor type inferred:** `br` present → `AnchorType::TwoCell` (emit
  `<xdr:from>` + `<xdr:to>`); `ext` present → `AnchorType::OneCell` (emit
  `<xdr:from>` + `<xdr:ext>`). Exactly one of `br`/`ext` is required (validate).

- **oneCell size emission.** Current writer hardcodes `cx="0" cy="0"`. When
  `ext` is given, emit `<xdr:ext cx="{width*EMU_PER_PX}" cy="{height*EMU_PER_PX}"/>`.
  When `br` (twoCell), size is implied by from/to, so `cx/cy` stay 0 (unchanged).

- **Read-back reconstruction.** `get_images()` builds `ImageAnchorInput` from
  the stored flat `ImageAnchor`:
  - `tl = { col: col + x / (DEFAULT_COL_WIDTH_PX*EMU_PER_PX),
            row: row + y / (DEFAULT_ROW_HEIGHT_PX*EMU_PER_PX) }`
  - twoCell → `br` reconstructed from `col2/row2/x2/y2`; oneCell → `ext`
    reconstructed from stored `cx/cy` (added to `WorksheetImage` if not already
    present). Integer rounding on read-back is acceptable (positions were
    authored against default dimensions anyway).

- **Validation at FFI boundary.** Reject (return `Err`) options where neither
  `br` nor `ext` is set, or both are set. Keep the error message explicit so
  migrating users see the fix immediately.

## Risks / Trade-offs

- **EMU mapping is approximate when column widths differ from the default.**
  Real sheets have variable widths, so a fractional `col` placed in one sheet
  may shift slightly in another. ExcelJS has the same limitation (it doesn't
  know target column widths either). Mitigation: document as "positioned
  against default cell size; exact pixel placement needs explicit `ext`".
- **Read-back fractional noise.** Reconstructing floats from integer EMU may
  yield `5.4999` instead of `5.5`. Mitigation: round to a sane precision (e.g.
  4 dp) on reconstruction; acceptable for positioning.
- **Breaking change for current excelrs users** who adopted the flat struct.
  Likelihood low (v1.0.0 images API is recent, shape leaked an internal detail).
  Mitigation: CHANGELOG entry + README note; the flat shape was never in docs.
- **`napi` type regen churn.** Changing the `#[napi(object)]` fields regenerates
  `index.d.ts`/`native.d.ts`. Mitigation: only those generated files change;
  no hand-edits needed beyond README.
