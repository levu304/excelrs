## 1. Glue injection script

- [x] 1.1 Create `scripts/apply-glue.cjs` — for each generated output file path in `process.argv`, if it is `index.js` or `index.d.ts` and lacks the glue marker, append the ExcelJS-compat `getCell` overloads (prototype patches for `.js`; `export interface` declaration-merging blocks for `.d.ts`). Idempotent and file-scoped.
- [x] 1.2 Add `--pipe "node scripts/apply-glue.cjs"` to the `build` and `build:release` scripts in `package.json`.

## 2. Verify rebuild restores glue

- [x] 2.1 Dry-run the script to confirm how `--pipe` passes the file (CLI arg vs stdin→stdout); adjust the script to match napi-rs's actual mechanism.
- [x] 2.2 Run `npm run build`; confirm `index.js` ends with `Worksheet.prototype.getCell` / `Row.prototype.getCell` and `index.d.ts` ends with `export interface Worksheet { getCell... }` / `export interface Row { getCell... }`.
- [x] 2.3 Run `npm run typecheck`; confirm zero `TS2339: Property 'getCell' does not exist` errors.

## 3. Land on the streaming branch + CI

- [ ] 3.1 Apply the fix as a commit on `feature/streaming-node-bridge` (or a `fix/getcell-glue` branch off it), then push.
- [ ] 3.2 Confirm the CI Typecheck job passes on the pushed branch (PR #33 goes green).
- [ ] 3.3 Confirm the 6 affected test files (`reader.test.ts`, `style.test.ts`, `tables.test.ts`, `theme-color.test.ts`, `workbook_xlsx.test.ts`, `worksheet.test.ts`) typecheck.
