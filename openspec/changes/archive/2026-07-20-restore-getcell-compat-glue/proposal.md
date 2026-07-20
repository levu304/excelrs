## Why

PR #17 ("v1.0.0 drop-in ExcelJS compatibility") appended hand-written `getCell` overload glue to the napi-generated `index.js` / `index.d.ts` (delegating to the Rust `getCellBy*` APIs). The streaming-bridge work (PR #33 / `feature/streaming-node-bridge`) ran `napi build`, which **overwrites** those generated files wholesale — wiping the glue. CI now fails: every test calling `worksheet.getCell(...)` or `row.getCell(...)` errors with `TS2339: Property 'getCell' does not exist on type 'Worksheet'`. This breaks the documented ExcelJS drop-in compatibility promise.

## What Changes

- Restore the ExcelJS-compat `getCell` overloads on `Worksheet` and `Row` so they survive `napi build` (currently lost on every regenerate).
- Use napi-rs's supported `--pipe` post-build hook to re-inject the glue into the generated `index.js` / `index.d.ts` instead of hand-patching committed files.
- No Rust changes, no public API changes — pure build-time glue restoration.

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities

- `exceljs-parity`: the drop-in `getCell` compatibility requirement (Row/Worksheet overloads delegating to `getCellBy*`) is currently broken by the regenerate. This change restores it. Delta spec required.

## Impact

- **Build**: `package.json` `build` / `build:release` scripts gain a `--pipe` step; new `scripts/apply-glue.cjs` post-processes generated output.
- **Generated files**: `index.js` / `index.d.ts` will once again carry the `getCell` glue after build (and stop drifting on rebuilds).
- **Tests**: `reader.test.ts`, `style.test.ts`, `tables.test.ts`, `theme-color.test.ts`, `workbook_xlsx.test.ts`, `worksheet.test.ts` (all call `.getCell`) will typecheck again; CI Typecheck job passes.
- **Consumers**: ExcelJS-compat `getCell` callers keep working — no breaking change.
