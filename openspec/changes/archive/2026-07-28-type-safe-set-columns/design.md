## Context

`Worksheet.setColumns` currently accepts `serde_json::Value` to allow ExcelJS-compatible plain-JS-object input. This bypasses napi-rs's type system — the parameter becomes `any` in the generated `.d.ts`, and the Rust side does unchecked serde deserialization at runtime.

The codebase already has a well-established pattern for typed input: `#[napi(object)]` structs like `AddWorksheetOptions`, `DataValidation`, `ConditionalFormat`, `PageSetup`, `SheetView`, etc. These generate proper TypeScript interfaces and accept plain JS objects natively through napi-rs's FFI.

The `Column` class itself is `#[napi]` (not `#[napi(object)]`) — it generates a TS class with constructor + getters/setters, which is correct for return values and programmatic mutation. But it can't double as the input type because `#[napi(object)]` requires `pub` fields while `#[napi]` class uses private fields with getter/setter methods.

## Goals / Non-Goals

**Goals:**

- Type-safe `setColumns` parameter on both TS and Rust sides
- Accept plain JS objects (ExcelJS compat) — no `new Column(...)` required
- Preserve all existing validation (col_num auto-assign, duplicate detection, style validation)
- Keep the `Column` class API unchanged
- Follow the existing `#[napi(object)]` pattern established in the codebase

**Non-Goals:**

- Changing the `Column` class API or internal representation
- Changing runtime behavior, error messages, or validation rules
- Adding new column features (those belong in separate changes)
- Refactoring the internal column storage

## Decisions

### D1 — Separate `ColumnInput` struct over modifying `Column`

**Decision:** Add a new `ColumnInput` struct with `#[napi(object)]` in `column.rs`.

**Alternatives considered:**

1. **Add `#[napi(object)]` to `Column`** — Rejected. `#[napi(object)]` requires `pub` fields, but `Column` uses private fields with getter/setter accessors (following the class pattern). A struct can't simultaneously be a `#[napi]` class and `#[napi(object)]` input type.

2. **Use `Vec<Column>` in the signature** — Rejected. napi-rs would require callers to pass `Column` instances (i.e., `new Column(...)` for each entry), breaking ExcelJS compat where callers pass plain objects.

3. **Keep `serde_json::Value` and add a hand-written TS override** — Rejected. This would create a maintainability burden (hand-typed declarations that drift from Rust implementation) and still leave the Rust side untyped.

**Rationale:** `ColumnInput` follows the exact pattern of `AddWorksheetOptions` — a `#[napi(object)]` input-only type that parallels a more complex class. napi-rs generates both the TS interface and the FFI deserialization automatically, giving type safety on both sides.

### D2 — All fields Option<>

**Decision:** Every field in `ColumnInput` SHALL be `Option<T>`, matching serde deserialization with `#[serde(default)]`.

```rust
#[napi(object)]
#[derive(Clone, Debug, Default)]
pub struct ColumnInput {
    pub col_num: Option<u32>,
    pub header: Option<String>,
    pub key: Option<String>,
    pub width: Option<f64>,
    pub hidden: Option<bool>,
    pub style: Option<Style>,
    pub outline_level: Option<u8>,
}
```

**Rationale:** napi-rs translates `Option<T>` to optional TS properties (`field?: T`). This matches the existing ExcelJS contract where column descriptors can omit fields and get defaults. The current serde path already treats all fields as optional (via `#[serde(default)]`).

### D3 — Inline conversion in set_columns

**Decision:** Convert `Vec<ColumnInput>` to `Vec<Column>` inside `set_columns`, reusing the same validation logic (auto-assign, duplicate check, style validation).

```rust
pub fn set_columns(&self, cols: Vec<ColumnInput>) -> napi::Result<()> {
    let columns = self.columns.lock().expect("...");
    let mut parsed: Vec<Column> = cols.into_iter().map(|c| Column {
        col_num: c.col_num.unwrap_or(0),
        header: c.header.unwrap_or_default(),
        key: c.key.unwrap_or_default(),
        width: c.width.unwrap_or(0.0),
        hidden: c.hidden.unwrap_or(false),
        style: c.style,
        outline_level: c.outline_level.unwrap_or(0).min(7) as u8,
    }).collect();

    // Same auto-assign, dedup, style validation as today
    // ...
}
```

**Rationale:** No new abstractions needed. The conversion is a mechanical field-map. Validation stays in `set_columns` where it lives today. A `From<ColumnInput> for Column` impl is possible but adds a file-scope dependency for a single call site.

### D4 — Style validation unchanged

**Decision:** The existing `apply_style` call chain in `set_columns` remains identical. `ColumnInput.style` is an `Option<Style>` which passes through naturally.

**Rationale:** Style validation is already extracted into `apply_style` and reused across Cell and Column setters. No changes needed.

## Risks / Trade-offs

- **[Duplicate struct definitions]** — `ColumnInput` mirrors some `Column` fields. If new fields are added to `Column`, they must also be added to `ColumnInput`. **Mitigation:** Both structs live in the same file (`column.rs`), making drift visible during code review.

- **[napi-rs object limit]** — napi-rs `#[napi(object)]` structs have a field count limit (≈64). `ColumnInput` has 7 fields — well within bounds. **Mitigation:** None needed.

- **[Breaking change for TypeScript callers using `any`]** — Any existing TypeScript code that passes a value of type `any` to `setColumns` will still work (TypeScript `any` is assignable to `ColumnInput[]`). Only code passing clearly incompatible types (e.g., `string`) will fail at compile time — which is the desired outcome.

## Migration Plan

1. Add `ColumnInput` struct to `column.rs`
2. Update `set_columns` signature in `worksheet.rs`
3. Update callers in tests (`xlsx.rs`, `worksheet.rs`) that pass `serde_json::Value` to use `Vec<ColumnInput>`
4. Rebuild — napi-rs auto-generates `native.d.ts` with new types
5. Verify `index.d.ts` reflects `ColumnInput[]` (apply-glue passthrough)

Rollback: Revert the two Rust files and rebuild.

## Open Questions

- None. All design decisions are resolved from existing patterns in the codebase.
