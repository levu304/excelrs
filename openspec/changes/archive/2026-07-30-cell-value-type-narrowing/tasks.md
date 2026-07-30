## 1. Rust Model — new accessors

- [x] 1.1 Add `#[napi(getter)] pub fn value_of(&self) -> CellValue` to `Cell` in `src/model/cell.rs` (clones `inner.value`, exposed as JS getter `cell.valueOf`).
- [x] 1.2 Add `#[napi(getter)] pub fn rich_text(&self) -> Option<Vec<RichTextRun>>` to `Cell` in `src/model/cell.rs` (clones `inner.value.rich_text`, exposed as JS getter `cell.richText`).
- [x] 1.3 `cargo build` passes ✅ — both getters use read-only `inner.lock()`, no mutation.

## 2. TypeScript declaration transform

- [x] 2.1 In `scripts/apply-glue.cjs`, extended the DTS-processing branch: added `CELLVALUE_UNION_TYPE` replacement that transforms `export interface CellValue { … }` into `export type CellValue = | { valueType: "Null" } | … | { valueType: "Merge" }`.
- [x] 2.2 Anchored replacement on `export interface CellValue {` + lazy `[^}]*` match to closing brace (does not disturb `RichTextRun`, `CellType`, or other interfaces).
- [x] 2.3 `CellValueResult` (`CellSimpleValue | Date | CellValue`) still resolves — `CellValue` is now a union but remains a valid member of the union.
- [x] 2.4 Guard added: the glue script checks `content.includes('export interface CellValue {')` before transforming; a CI `typecheck` assertion validates the transform applied.

## 3. Rebuild & verify generated types

- [x] 3.1 Run `pnpm build`; inspect `native.d.ts` + `index.d.ts`.
- [x] 3.2 Assert `CellValue` is `export type CellValue = …` (union), not `export interface CellValue`.
- [x] 3.3 Assert `Cell` exposes `get valueOf(): CellValue` and `get richText(): RichTextRun[] | null`.

## 4. Tests

- [x] 4.1 `__test__/rich-text.test.ts`: replace `const result = cell2.value as import('../index').CellValue` with `cell2.richText` getter.
- [x] 4.2 Add narrowing compile-check test: `const cv = cell2.valueOf; if (cv.valueType === "RichText") expect(cv.richText.length).toBe(2)` — no cast needed.
- [x] 4.3 `__test__/cell.test.ts`: add tests for `cell.valueOf` getter with narrowing by valueType, and `cell.richText` for rich/Number cells.
- [x] 4.4 Run `pnpm test` + `pnpm typecheck`; all green.

## 5. Spec updates

- [x] 5.1 `openspec/specs/cell-value-getter/spec.md`: delta spec reflects getter semantics (not method) for `cell.valueOf`; requires `CellValue` discriminated union narrowable on `valueType`; `cell.richText` returns `RichTextRun[] | null` without cast.
- [x] 5.2 `openspec/specs/rich-text/spec.md`: `Cell.richText` is a `#[napi(getter)]` returning parsed runs directly, no cast, consistent with `cell.formula`.
- [x] 5.3 `package.json` bumped to `v2.5.1`; CHANGELOG entry added.

## 6. Validation

- [x] 6.1 `openspec validate cell-value-type-narrowing` passes.
- [x] 6.2 `pnpm typecheck` passes ✓
- [x] 6.3 `pnpm test` passes ✓ (152 tests)
