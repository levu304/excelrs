// Type-level regression test for the Cell.value setter soundness fix.
// NOT executed by vitest (typecheck mode off) — enforced by `npm run typecheck`
// (tsc --noEmit), which includes __test__/**/*.ts via tsconfig.json.
//
// `CellValueInput` is a union of optional-discriminant variant shapes, so
// excess-property checking rejects cross-variant field mixes while still
// accepting shape-inference objects and explicit variants.
import type { Cell } from '../index'

declare const cell: Cell

// Spec: cross-variant field combinations MUST NOT compile.
// @ts-expect-error valueType Number + string field from String variant
cell.value = { valueType: 'Number', string: 'leaked' }
// @ts-expect-error unknown discriminant must not compile
cell.value = { valueType: 'Banana', number: 5 }
// @ts-expect-error variant field required when discriminant present
cell.value = { valueType: 'Number' }

// Backward compat: omitting valueType (shape inference) MUST still compile.
cell.value = { number: 42 }
cell.value = { formula: 'SUM(A1:B1)' }
cell.value = { hyperlink: 'https://x', hyperlinkText: 'X' }
cell.value = { richText: [{ text: 'hi' }] }

// Explicit variants (round-trip from getter) MUST still compile.
cell.value = { valueType: 'Number', number: 5 }
cell.value = { valueType: 'RichText', richText: [] }
cell.value = { valueType: 'Hyperlink', hyperlink: 'https://x' }
cell.value = { valueType: 'Null' }
cell.value = { valueType: 'Merge' }