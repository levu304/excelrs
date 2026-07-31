## 1. Add CellValueInput type constant

- [x] 1.1 Add `CELLVALUE_INPUT_TYPE` constant to `scripts/apply-glue.cjs` (after `CELLVALUE_UNION_TYPE`)
- [x] 1.2 Update `REFINED_CELL_VALUE_SETTER` to reference `CellValueInput` instead of `Partial<CellValue>`

## 2. Inject CellValueInput into generated DTS

- [x] 2.1 In `apply-glue.cjs`, after the `CellValue` union replacement, append `CELLVALUE_INPUT_TYPE` to the DTS output (both `native.d.ts` and `index.d.ts` branches)

## 3. Regenerate DTS

- [x] 3.1 Run `napi build --pipe "node scripts/apply-glue.cjs"` and verify `index.d.ts` contains `CellValueInput` and the updated setter signature

## 4. Add missing null-path tests

- [x] 4.1 Add test: `cell.richText` returns `null` for a Number cell (`__test__/cell.test.ts`)
- [x] 4.2 Add test: `cell.valueOf` returns `{ valueType: "Null" }` for a freshly constructed cell (`__test__/cell.test.ts`)

## 5. Verify

- [x] 5.1 Run `npm test` — all tests pass (including new ones)
- [x] 5.2 Run `npx tsc --noEmit` — no type errors
- [x] 5.3 Run `node -e "require('./index.js'); console.log('native load OK')"` — native module loads
