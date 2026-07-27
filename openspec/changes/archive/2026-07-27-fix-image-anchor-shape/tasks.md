## 1. Public anchor types (Rust model)

- [x] 1.1 Add `#[napi(object)]` `AnchorPoint { col: f64, row: f64 }` and `ImageSize { width: f64, height: f64 }`.
- [x] 1.2 Add `#[napi(object)]` `ImageAnchorInput { tl: AnchorPoint, br: Option<AnchorPoint>, ext: Option<ImageSize> }`.
- [x] 1.3 Retype `AddImageOptions.anchor` to `ImageAnchorInput`; retype `ImageInfo.anchor` to `ImageAnchorInput`.
- [x] 1.4 Add `WorksheetImage.ext_size: Option<(u32, u32)>` for one-cell size.

## 2. Boundary conversion

- [x] 2.1 Add `ImageAnchorInput.to_internal()` — infers AnchorType, splits fractional col/row.
- [x] 2.2 In `add_image()`, convert via `to_internal()` before storing.
- [x] 2.3 Add `ImageAnchor.to_exceljs_shape()` for `get_images()` (reverse conversion).
- [x] 2.4 Clear `Err` when neither/both `br`/`ext` present.

## 3. Writer one-cell size emission

- [x] 3.1 Emit `<xdr:ext cx=".." cy=".."/>` for oneCell with explicit ext_size.
- [x] 3.2 Keep `cx="0" cy="0"` for twoCell (unchanged).

## 4. Regenerate types + README

- [x] 4.1 Run `napi build` — `native.d.ts` generated with correct types.
- [x] 4.2 Fix `README.md` with compiling ExcelJS-shaped example + anchor-shape note.

## 5. Verify

- [x] 5.1 TS + Rust tests adjusted (existing images tests updated).
- [x] 5.2 `cargo test` passes (384 passed).
- [x] 5.3 `openspec validate` passes.
