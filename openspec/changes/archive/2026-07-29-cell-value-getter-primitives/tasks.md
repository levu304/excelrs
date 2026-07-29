## 1. Rust Model Changes

- [x] 1.1 Add `#[napi(string_enum)] pub enum CellType { Null, Number, String, Boolean, Date, Formula, Error, Hyperlink, RichText, Merge }` to `src/model/cell.rs`.
- [x] 1.2 Change `value()` getter signature to `pub fn value(&self, env: Env) -> napi::Result<Unknown<'_>>` and add `#[napi(getter, ts_return_type = "CellValueResult")]`.
- [x] 1.3 Implement the per-variant match: Number→`env.to_js_value(&cv.number.unwrap_or(f64::NAN))`; String→`env.to_js_value(&cv.string.as_deref().unwrap_or(""))`; Boolean→`env.to_js_value(&cv.boolean.unwrap_or(false))`; Date→`env.create_date(ms)` then `JsDate::to_unknown()` (with the same `transmute` to `'static` used by the `date` getter); Null→`env.create_null()` then `to_unknown()`; `_`→`env.to_js_value(cv)`.
- [x] 1.4 Add `#[napi(getter, js_name = "type")] pub fn value_type(&self) -> CellType` returning the `CellType` variant matching `inner.value.value_type`.
- [x] 1.5 Narrow the `value_type` field on the `CellValue` struct with `#[napi(ts_type = "\"Null\" | \"Number\" | ... ")]` so the generated interface discriminant is a string literal union.

## 2. TypeScript Declaration Header

- [x] 2.1 Create `dts-header.d.ts` containing: `CellSimpleValue` (`number | string | boolean | null`), `CellValueResult` (`CellSimpleValue | Date | CellValue`), and a `Cell` interface declaration merge with setter narrow + `get date()` deprecation.
- [x] 2.2 Add `"dtsHeaderFile": "./dts-header.d.ts"` to the `napi` config in `package.json`.
- [x] 2.3 Remove the `Cell` setter merge block from `scripts/apply-glue.cjs` (now provided by the header) so the `set value(...)` signature lives in one place; keep the `getCell` glue.

## 3. Build & Verify Generated Types

- [x] 3.1 Run `pnpm build` and confirm `native.d.ts`/`index.d.ts` regenerate with `get value(): CellValueResult`, `get type(): CellType`, and the `CellType` enum.
- [x] 3.2 Run `pnpm typecheck` and confirm no TS errors in the generated declarations and the package entrypoint.

## 4. Test Updates

- [x] 4.1 Update `__test__/cell.test.ts`: replace `cell.value.valueType` / `cell.value.number` / `.string` / `.boolean` assertions with `cell.type` / `cell.value` primitive checks; keep the unknown-`valueType` error test and the rich-text/hyperlink/formula object tests unchanged.
- [x] 4.2 Update `__test__/xlsx-async-contract.test.ts`: change `cellVal.valueType` checks (Number/Date) to `cell.type` and `cell.value` primitive / `cell.value` Date instance; keep `cell.date` assertions.
- [x] 4.3 Run `pnpm test` (vitest) and confirm all cell/value/rich-text/date tests pass.

## 5. Release

- [x] 5.1 Bump `version` to `2.5.0` in `package.json`; add a CHANGELOG entry describing the breaking `cell.value` getter change, the new `cell.type` accessor, and the migration note (`cell.value.valueType` / `.number` → `cell.type` / `cell.value`).
- [x] 5.2 Add `@deprecated` JSDoc to the `get date()` declaration in `dts-header.d.ts` noting removal in v3.
