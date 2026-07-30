## 1. Document the UTC convention

- [x] 1.1 Add a note to the v2.5.0 migration section in CHANGELOG.md: `cell.value` / `cell.date` for Date cells are UTC-anchored (matches ExcelJS 4.4); use `toISOString()` / `getUTC*` for exact values, or normalize for local display.
- [x] 1.2 Note that `cell.value = new Date(y, m, d)` (local constructor) is interpreted by UTC milliseconds; use `new Date(Date.UTC(...))` for exact calendar dates.

## 2. Lock the contract with value-asserting tests

- [x] 2.1 In `__test__/reader.test.ts`, replace the `a2.value instanceof Date` assertion (date cell) with a value assertion: `a2.value.toISOString()` equals the expected UTC instant for the known serial.
- [x] 2.2 In `__test__/cell.test.ts`, add a test that `cell.value = new Date(Date.UTC(2026, 0, 15))` round-trips to `toISOString() === '2026-01-15T00:00:00.000Z'`.
- [x] 2.3 Run the date-related test files and confirm all pass.

## 3. Reconcile the spec

- [x] 3.1 Confirm the `date-cell-value` delta spec (`specs/date-cell-value/spec.md`) is present and states the UTC-anchored requirement with tz-safe scenarios.
- [x] 3.2 Validate the change with `openspec validate date-value-utc-convention`.

## 4. Verify

- [x] 4.1 Run the full test suite (`pnpm test`) and confirm 0 failures.
- [x] 4.2 Review the diff is docs + tests + spec only (no `src/` changes).
