## Context

`Cell` wraps `Arc<Mutex<CellInner>>`; the `value` getter/setter bridge the FFI boundary. Today `set_value(&mut self, val: napi::Unknown)` does: (Path 1) a raw JS `Date` → Excel serial; (Path 2) `serde_json::Value::from_napi_value` then `match` on Number/String/Bool, with **every object falling through to `CellValue::default()` (Null)**. Rich text, hyperlink and formula are only constructible via the internal `set_value_raw` (Rust-only), so JS consumers can never create them. The writer (`write_cell_xml`) already emits correct XML for RichText (`t="inlineStr"`, `<is><r><rPr>…</rPr><t>…</t></r>…</is>`), and the reader already parses inline rich text into `RichTextRun`. ExcelJS@4.4.0 infers the value type by object shape (`value.richText` → RichText, etc.) and treats `cell.value = { richText: [...] }` as canonical — exactly the user's reproduction. excelrs instead uses an explicit flat-union discriminant (`value_type`); its `value` getter already returns a typed `CellValue`. The mismatch is purely at the setter's JS→Rust bridge, plus the `unknown` TS type.

## Goals / Non-Goals

**Goals:**

- Make the public setter accept object values for RichText, Hyperlink, Formula.
- Honor an explicit `valueType` so read-back objects round-trip (`cell.value = cell.value`).
- Type the setter as a union instead of `unknown`.

**Non-Goals:**

- Changing the writer emission XML or the reader (both already correct).
- Adding an ExcelJS-style derived `cell.type` getter (separate concern).
- Emitting `<u>` (underline) in the rich-text writer — read supports it, writer does not; tracked separately.

## Decisions

- **D1 — Shape inference + explicit discriminant.** Inspect the JS object in priority order: (1) raw `Date` → serial; (2) `richText` present → `CellValue::rich_text`; (3) `hyperlink` present → `CellValue::hyperlink`; (4) `formula` present → `CellValue::formula`; (5) explicit `valueType` field present → build that variant from the object's fields; (6) `typeof` primitive → Number/String/Boolean/Null. *Rationale:* matches ExcelJS idiom (user's code omits `valueType`) and excelrs's own spec, while the explicit-discriminant branch preserves round-trip. *Alternatives considered:* (a) require explicit `valueType` only — rejected, breaks the documented idiom; (b) add a separate `setRichText(runs)` method — rejected, leaves `cell.value = { richText }` broken and the spec scenario failing.
- **D2 — camelCase keys, direct mapping.** The JS object uses camelCase (`richText`, `hyperlink`, `hyperlinkText`, `formula`, `valueType`, `dateSerial`, `errorValue`) — identical to the napi/TS `CellValue` shape. Build each `RichTextRun` straight from `{ text, font }`; map font fields `name`, `size`, `bold`, `italic`, `color` (already emitted by the writer).
- **D3 — d.ts fix via glue, not Rust signature.** Keep the Rust parameter `napi::Unknown` (napi cannot express a primitive+object union cleanly) and retype the setter in `scripts/apply-glue.cjs` by declaration-merging `export interface Cell { set value(val: CellValue | string | number | boolean | Date | null): void }`. This reuses the existing post-build patch mechanism.

## Risks / Trade-offs

- **[Risk]** Ambiguous objects. → *Mitigation:* deterministic priority order (Date → richText → hyperlink → formula → valueType → primitive); a plain string/number never has those keys.
- **[Risk]** Unknown/extra keys are ignored. → *Mitigation:* only meaningful keys are consumed; harmless extras ignored (same as ExcelJS).
- **[Risk]** Per-assignment shape inspection overhead. → *Mitigation:* negligible — assignments are far rarer than reads, and it only runs on object/Date values.
- **[Risk]** `unknown` → union could surface a previously-accepted invalid assignment as a type error. → *Mitigation:* union is a superset that still accepts `unknown`-compatible values; no existing call site breaks.

## Migration Plan

- Internal `set_value_raw` and all existing Rust tests (e.g. `test_rich_text_roundtrip`) stay unchanged.
- JS consumers gain working behavior; nothing is removed.
- Ship as a minor/patch release (no breaking change). Gate with the new JS round-trip test plus the existing Rust rich-text test.

## Open Questions

- Include Hyperlink/Formula/Date-round-trip in this change (same code path) or defer? *Recommend: include* — excluding them leaves the identical bug for those variants.
- Underline emission in the rich-text writer (`<u>`) is a separate known gap (read supports it) — file a follow-up rather than widen this change.
