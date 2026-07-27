## Why

Users migrating from ExcelJS hit `Property 'addImage' does not exist on type 'Workbook'` because excelrs places `addImage` on **Worksheet** (one-step opts) while ExcelJS places it on **Workbook** (two-step: add image → get ID → position). This is a deliberate API simplification, but it's undocumented — no README note tells users how to adapt their code.

## What Changes

- **README patch**: Add a compatibility note in the "API Surface" / Quick Start section documenting the `addImage` API divergence.
- No Rust code, no TypeScript type changes, no new methods.

## Capabilities

No new capabilities. No spec-level requirement changes. This is a documentation-only change — the images spec already describes the excelrs API correctly. The gap is in README, not in the spec.

### New Capabilities

- *(none — this is a docs-only change)*

### Modified Capabilities

- *(none — no spec-level behavior changes)*

## Impact

- `README.md` — one new paragraph documenting the `ws.addImage` vs `wb.addImage` divergence
- No API surface changes, no test changes, no build changes
