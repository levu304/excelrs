# Proposal: Fix CellValue setter type soundness

## What

Replace `Partial<CellValue>` in the `Cell.value` setter with a flat `CellValueInput` interface that preserves backward compatibility without the cross-variant field leakage introduced by the discriminated union.

## Why

PR #47 (`cell-value-type-narrowing`) replaced the old flat `CellValue` interface with a discriminated union — a major type-safety improvement for getters. But the setter retained `Partial<CellValue>` from the old API. With the new union, `Partial` distributes over each member, creating a type hole:

```typescript
// This compiles but is nonsense — cross-variant field mix:
cell.value = { valueType: "Number", string: "leaked from String variant" }
// This also compiles — cross-variant without valueType:
cell.value = { number: 5, string: "hi" }
```

Runtime is safe (`set_value` dispatches on `valueType` and ignores spurious fields), but the type system is now weaker than the old flat interface on the setter path. The PR that improved types on the getter path *regressed* them on the setter path.

## Scope

- **In scope**: New `CellValueInput` type in `dts-header.d.ts` / `apply-glue.cjs`, updated setter signature, two missing null-path tests
- **Out of scope**: Changing `valueOf` / `richText` getter signatures, Rust-side changes

## Impact

- **Non-breaking**. `cell.value = { number: 42 }` still works. Old code unaffected.
- **`index.d.ts` only** — no Rust compile.
