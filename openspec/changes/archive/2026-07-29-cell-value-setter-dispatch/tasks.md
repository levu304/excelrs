## 1. Setter dispatch (Rust)

- [x] 1.1 In `src/model/cell.rs` `set_value`, add object-shape inspection after the Date path: `richText` → `CellValue::rich_text(runs)`, `hyperlink` → `CellValue::hyperlink(url, text)`, `formula` → `CellValue::formula(s)`.
- [x] 1.2 Add an explicit-discriminant branch: when the object has `valueType`, build the matching variant from its fields (Number/String/Boolean/Date/Hyperlink/RichText/Formula) instead of dropping to Null.
- [x] 1.3 Preserve the primitive fallback (`typeof` → Number/String/Boolean/Null) for non-object values.
- [x] 1.4 Map camelCase JS keys to the Rust `CellValue` fields (`richText`→`rich_text`, `hyperlinkText`→`hyperlink_text`, `valueType`→`value_type`, `dateSerial`→`date_serial`, `errorValue`→`error_value`) and build `RichTextRun { text, font }` per run.
- [x] 1.5 Reuse existing `CellValue::validate` for rich-text font validation (no new validation logic).

## 2. Type surface (d.ts)

- [x] 2.1 In `scripts/apply-glue.cjs`, declaration-merge `export interface Cell { set value(val: CellValue | string | number | boolean | Date | null): void }` into the generated `index.d.ts` (reuse the existing `DTS_GLUE` patch mechanism).
- [x] 2.2 Rebuild (`pnpm build` / `napi build --pipe`) and confirm `index.d.ts` no longer types `value` as `unknown`.

## 3. Tests

- [x] 3.1 Add a JS test: assign `ws.getCell('A1').value = { richText: [{ text: "B: …", font: { name: "Times New Roman", size: 8 } }, { text: "S: …", font: { name: "Times New Roman", size: 8 } }] }`, write, read back, assert `value_type === "RichText"` and runs/text/font match.
- [x] 3.2 Add JS tests for hyperlink and formula object assignment (same dispatch path).
- [x] 3.3 Run `cargo test test_rich_text_roundtrip` and the full Rust + JS suites; confirm no regression.

## 4. Verification

- [x] 4.1 Run `openspec validate cell-value-setter-dispatch` and confirm all artifacts pass.
- [x] 4.2 Manual: run the user's exact reproduction snippet and confirm rich text renders in the output XLSX.
