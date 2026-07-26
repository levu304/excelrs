# Design: Style Setter Type Safety

## Context

`excelrs` is a Rust (napi-rs) + Node native addon. `#[napi(setter)]` methods
that take `serde_json::Value` generate `any` in TypeScript. Four style setters
have this issue (Cell, Row, Column, Worksheet.setCellStyle), all using identical
`serde_json::Value → serde_json::from_value<Style>() → validate()` logic.

The getters already return `Option<Style>`, which napi-rs correctly translates to
`Style | null`. Changing the setters from `serde_json::Value` to `Option<Style>`
closes the asymmetry and gives TypeScript callers compile-time checking for style
assignments — the most common setter in the API.

This change is **restrictive** — no new capabilities, no API surface growth.
The only user-visible change is the narrowed TypeScript type.

## Approach

### Rust side: change parameter type

All four setters follow the same body pattern:

```rust
// before
#[napi(setter)]
pub fn set_style(&mut self, val: serde_json::Value) -> napi::Result<()> {
    if val.is_null() {
        *self.style = None;
        return Ok(());
    }
    let style: Style = serde_json::from_value(val).map_err(|e| napi::Error::from_reason(...))?;
    if style.is_empty() {
        *self.style = None;
        return Ok(());
    }
    *self.style = Some(style.validate().map_err(|e| napi::Error::from_reason(...))?);
    Ok(())
}
```

→

```rust
// after
#[napi(setter)]
pub fn set_style(&mut self, val: Option<Style>) -> napi::Result<()> {
    match val {
        None => {
            *self.style = None;
        }
        Some(ref s) if s.is_empty() => {
            *self.style = None;
        }
        Some(s) => {
            *self.style = Some(s.validate().map_err(|e| napi::Error::from_reason(e.to_string()))?);
        }
    }
    Ok(())
}
```

Key points:

- `Option<Style>` napi-rs maps to `Style | null | undefined` in TS
- `serde_json::Value` → `Option<Style>` removes the `serde_json::from_value` call
- Validation (`Style::validate()`) and `is_empty()` normalization stay identical
- `null` and `undefined` both deserialize to `None` (napi-rs handles this)
- Empty JS object `{}` deserializes to `Some(Style)` with all `None` fields → caught by `is_empty()`

### Test callers

Four internal test callers pass `serde_json::json!({...})` literals directly:

1. `worksheet.rs:1364` — `cell.set_style(serde_json::json!({...}))`
2. `worksheet.rs:1408` — `cell.set_style_raw(Some(Style {...}))` (already uses raw)
3. `worksheet.rs:1477` — `.set_style(serde_json::json!({...}))`
4. See line audit for remaining

These must switch to constructing `Style` structs directly or using `set_style_raw()`.

### Generated TypeScript

Before:

```typescript
get style(): Style | null
set style(val: any)
```

After:

```typescript
get style(): Style | null
set style(val: Style | null | undefined)
```

## Risk Assessment

| Risk | Likelihood | Mitigation |
| ---- | ---------- | ---------- |
| `{}` empty-object reset broken | Low | `Option<Style>` + `is_empty()` guard handles same path |
| napi-rs `Option<T>` for objects not supported in pinned version | Low | Validated against napi-rs 3 — `Option<Style>` is a standard pattern |
| test callers missed | Low | Compile error on every direct `serde_json::json!` → `set_style` call; easy to catch |
| Error message quality regresses for non-object values | Low | napi-rs produces earlier, less specific error for primitives — these were always rejected at serde level first; net effect is same: caller gets an error |

## Internal changes: test callers

The four `set_style(serde_json::json!(...))` calls in worksheet tests need to
construct `Style` structs directly:

| Location | Current | Replace with |
| -------- | ------- | ------------ |
| `worksheet.rs:1362-1380` | `cell.set_style(json!({numFmt: "yyyy-mm-dd"...}))` | Build `Style` struct directly |
| `worksheet.rs:1475-1485` | `.set_style(json!({font: {bold: true}}))` | Build `Style` struct directly or use `set_style_raw` |
