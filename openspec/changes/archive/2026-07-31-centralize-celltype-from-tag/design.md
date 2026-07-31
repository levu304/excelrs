## Context

`src/model/cell.rs` defines `CellType` as a `#[napi(string_enum)]` enum with 10 variants. The `value_type` discriminant string on `CellValue` is matched in three inline sites within `cell.rs` alone:

- `Cell::value()` (~line 253) — dispatches on `cv.value_type.as_str()` to choose the JS return type
- `Cell::value_type()` (~line 275) — converts the tag string back to a `CellType` variant
- `Cell::set_value()` (~line 460) — `matches!(vt, "Number" | "String" | ...)` validation that rejects unknown tags

Five additional sites in `csv.rs`, `table.rs`, and `writer/xlsx.rs` re-list the same literals for behavior dispatch (not validation).

The `#[napi(string_enum)]` attribute generates JS→Rust and Rust→JS string conversion for the N-API bridge, but does **not** expose a Rust-side `from_str` or `from_tag` that internal code can call. The issue author explicitly ruled out `strum::EnumString` ("not worth 10 lines").

## Goals

- Single source of truth for the 10-arm tag→`CellType` mapping.
- Eliminate silent misclassification risk: all three `cell.rs` sites route through one table.
- Behavior-neutral: identical fallback semantics (`_ → Null` for `value_type()`, `matches!` validation for `set_value()`).
- No new dependencies; `pub(crate)` visibility only.

## Non-Goals

- Migrating the five behavior-dispatch sites in `csv.rs`, `table.rs`, `writer/xlsx.rs` — these are dispatch tables, not validation. Left as a separate follow-up change if deemed worthwhile.
- Renaming or restructuring the `CellType` enum itself.
- Any change to the `#[napi(string_enum)]` attribute or JS-facing DTS.

## Decisions

### D1: `from_tag` returns `Option<CellType>`

```rust
impl CellType {
    pub(crate) fn from_tag(tag: &str) -> Option<CellType> {
        Some(match tag {
            "Null" => CellType::Null,
            "Number" => CellType::Number,
            "String" => CellType::String,
            "Boolean" => CellType::Boolean,
            "Date" => CellType::Date,
            "Formula" => CellType::Formula,
            "Error" => CellType::Error,
            "Hyperlink" => CellType::Hyperlink,
            "RichText" => CellType::RichText,
            "Merge" => CellType::Merge,
            _ => return None,
        })
    }
}
```

**Rationale:** `Option` return lets each call site choose its own fallback (e.g. `unwrap_or(CellType::Null)` vs `.is_some()` vs `.ok_or(...)`). A `Result` would force error construction on every unknown tag, which is noise for the hot path.

### D2: `value_type()` uses `unwrap_or(CellType::Null)`

```rust
CellType::from_tag(&inner.value.value_type).unwrap_or(CellType::Null)
```

Preserves the existing `_ => CellType::Null` fallback exactly.

### D3: `set_value()` uses `.is_some()`

```rust
if !CellType::from_tag(vt).is_some() {
    return Err(napi::Error::from_reason(format!(
        "Unknown valueType discriminant: '{vt}'. Expected one of: ..."
    )));
}
```

Preserves the exact same error message and behavior — any tag that wasn't `Null`…`Merge` already triggers this path.

### D4: `value()` dispatch table — NOT migrated

`value()` dispatches `value_type` → JS return type (e.g. `"Date"` → `JsDate`, `"Number"` → `f64`). This is a **behavior dispatch**, not a tag-validation. The `_ =>` arm returns `env.to_js_value(cv)` (raw `CellValue` object) for Formula/RichText/Hyperlink/Error/Merge. Migrating to `from_tag` here would require mapping `CellType → JS return`, which is a different abstraction. **Left as-is** per Non-Goals.

## Risks / Trade-offs

| Risk | Mitigation |
| --- | --- |
| Match exhaustiveness: future variant added but missed in `from_tag` | Rust exhaustiveness checking on `match tag` inside `from_tag` — compiler will warn on new enum variant if arms don't match. This is the **primary safety gain**: the compiler forces the table to stay in sync with the enum. |
| `set_value()` error message lists 10 names manually | The message is a static string, not generated from the enum. Low risk: new variant would need manual message update anyway. |
| `value()` still has inline match (D4) | Documented as Non-Goal. The critical validation path (`set_value`) is covered.
