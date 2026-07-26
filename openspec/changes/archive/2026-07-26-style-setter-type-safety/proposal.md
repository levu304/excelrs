# Proposal: Style Setter Type Safety

## Why

`cell.style`, `row.style`, `column.style`, and `worksheet.setCellStyle()` all
accept `any` on the TypeScript side because the Rust `#[napi(setter)]` methods
use `serde_json::Value`, which napi-rs translates to `any` in the generated
`index.d.ts`.

This silently bypasses TypeScript's compile-time type checking. A user writing

```typescript
cell.style = { font: { bold: "not-a-boolean" } }
```

gets no compiler error — only a late runtime `InvalidStyle` error from the Rust
validation. Similarly, misspelled fields, wrong enum values, and bad color hex
strings are all invisible until runtime.

The getter is correctly typed (`Style | null`), creating an asymmetry that
signals the wrong intent: "the property is read-typed but write-any."

## What Changes

Change the Rust setter parameter type from `serde_json::Value` to `Option<Style>`
in all four locations, so napi-rs generates `Style | null | undefined` instead of
`any`.

### Scope

| File | Method | Current Type | Target Type |
| ---- | ------ | ------------ | ----------- |
| `src/model/cell.rs` | `Cell.set_style` | `serde_json::Value` | `Option<Style>` |
| `src/model/row.rs` | `Row.set_style` | `serde_json::Value` | `Option<Style>` |
| `src/model/column.rs` | `Column.set_style` | `serde_json::Value` | `Option<Style>` |
| `src/model/worksheet.rs` | `Worksheet.set_cell_style` | `serde_json::Value` | `Option<Style>` |

### Test calls in worksheet.rs

Four test sites currently pass `serde_json::json!({...})` literals directly to
`set_style()` / `set_cell_style()`. These must be converted to use `Style`
struct instances (or `set_style_raw()`):

- `set_cell_style(2, 2, serde_json::json!({...}))` ×2
- `cell.set_style(serde_json::json!({...}))` ×2

## Impact

- **Generated `.d.ts`**: `set style(val: Style | null | undefined)` instead of
  `set style(val: any)` — full TypeScript type safety.
- **Runtime behavior**: identical. `null` / `undefined` still resets to Normal.
  `{}` deserializes to an all-`None` `Style`, which `is_empty()` catches and
  normalizes to `None` — same as today.
- **Error messages**: marginally less detailed for non-object / non-null JS
  values (napi-rs handles conversion before our code runs), but the value class
  that triggered it was never valid — earlier rejection is better.
- **No breaking changes** to the JS/TS API surface. Setter works the same. Only
  the TypeScript type narrows.
- **Package version**: patch bump (`0.13.0` → `0.13.1`) since this is a type-
  only tightening with no behavioral change.
