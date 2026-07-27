## 1. Reader parses `<xdr:ext>`

- [x] 1.1 In `parse_drawing_xml` (`src/reader/xlsx.rs`), add a `current_ext: Option<(u32,u32)>` accumulator, reset to `None` on `xdr:oneCellAnchor` / `xdr:twoCellAnchor` start.
- [x] 1.2 Add a `b"xdr:ext"` match arm in the `Start | Empty` handler that reads `cx`/`cy` attributes into `current_ext`.
- [x] 1.3 Change the function return type from `Vec<(String, ImageAnchor)>` to `Vec<(String, ImageAnchor, Option<(u32,u32)>)>` and push `current_ext` at `xdr:pic` end.

## 2. Reader assigns parsed ext

- [x] 2.1 In `parse_sheet_images`, destructure the third tuple element and assign it to `WorksheetImage.ext_size` (replace the hardcoded `ext_size: None`).

## 3. Writer always emits ext for one-cell

- [x] 3.1 In `write_drawing_xml` (`src/writer/xlsx.rs`), replace the `if let Some((cx, cy)) = img.ext_size` guard with a `OneCell` branch that emits `<xdr:ext cx="{cx}" cy="{cy}"/>` using `ext_size.unwrap_or((0,0))`. Keep the two-cell `<xdr:to>` branch unchanged.

## 4. Verify

- [x] 4.1 Add a Rust test in `src/xlsx/handle.rs`: write a one-cell anchor with `ext: { width: 120, height: 60 }`, re-read the workbook, assert `getImages()[0].anchor.ext` equals `{ width: 120, height: 60 }` (within 4 dp) and the emitted drawing XML contains `<xdr:ext cx="1143000" cy="571500"/>`.
- [x] 4.2 Confirm `cargo test` passes, `napi build` regenerates `native.d.ts` (no public type change), and `tsc --noEmit` stays clean.
- [x] 4.3 `openspec validate fix-image-ext-readback` passes.
