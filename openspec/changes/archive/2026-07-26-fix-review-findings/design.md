## Context

The `style-setter-type-safety` change (archived) converted `Cell::set_style`, `Row::set_style`, `Column::set_style`, and `Worksheet::set_cell_style` from `serde_json::Value` to `Option<Style>`. A post-archive code review found four clean-up items:

1. **Redundant `map_err`**: The match arm `Some(s) => inner.style = Some(s.validate().map_err(...)?)` adds a `.map_err` wrapper when `From<ExcelrsError> for napi::Error` (defined in `src/error.rs:53`) makes bare `?` sufficient. The pattern appears in 5 places across 4 model files.

2. **DRY match body**: The 3-arm match (`None` / `Some(empty)` / `Some(valid)`) is copy-pasted across `cell.rs`, `row.rs`, `column.rs`, plus a fourth variant in `worksheet.rs`'s `set_columns` loop. All do identical normalization.

3. **Missing edge-case tests**: `test_set_style_empty_object` exists only for Cell. Row and Column don't test the `is_empty()` guard. No entity tests `set_style(Some(invalid))` → `Err`.

No spec-level behavioral changes. The JS/TS API surface is unaffected.

## Goals / Non-Goals

**Goals:**

- Eliminate redundant `.map_err` wrapper on `validate()` calls in `src/model/` (5 locations)
- Extract a shared `apply_style` normalization helper so all style-setter paths use one function
- Add edge-case test coverage for empty-style normalization on Row and Column
- Add validation-error-path tests for Cell, Row, and Column

**Non-Goals:**

- Fixing `map_err` patterns outside `src/model/` (80+ occurrences, many use non-ExcelrsError types — separate concern)
- Changing TS types or the napi-rs API surface
- Adding new style capabilities (no new specs)

## Decisions

### 1. Extract `apply_style` helper vs. keep inline

**Decision**: Extract `pub(crate) fn apply_style` in `src/model/style.rs`.

**Rationale**: 4 call sites (3 setters + `set_columns` loop body) all implement the same normalization. The `set_columns` loop uses a different code shape (if-let + `take()` + manual `is_empty` check) than the setters (match). A shared helper enforces one correct implementation and prevents future setter-like code from re-implementing or subtly diverging.

**Trade-off**: Each setter becomes a 1-line call + trailing `Ok(())`. The reader must jump to `style.rs` to see what `apply_style` does. However, the function is small (5 lines) and lives alongside `Style::validate()` and `Style::is_empty()` which it calls.

```rust
/// Apply an `Option<Style>` to a style slot, normalizing `None`/empty → `None`.
/// Shared by Cell, Column, Row, and Worksheet::set_columns.
pub(crate) fn apply_style(
    dest: &mut Option<Style>,
    val: Option<Style>,
) -> napi::Result<()> {
    match val {
        None | Some(ref s) if s.is_empty() => *dest = None,
        Some(s) => *dest = Some(s.validate()?),
    }
    Ok(())
}
```

Combines `None` and `Some(empty)` arms into a single pattern (`None | Some(ref s) if s.is_empty()`) to express "absent or empty → clear" as one concept.

### 2. Bare `?` vs. explicit `map_err`

**Decision**: Use bare `?` inside `apply_style` (removing the `.map_err` wrapper).

**Rationale**: `Style::validate()` returns `Result<Style, ExcelrsError>`. `apply_style` returns `napi::Result<()>`. The `From<ExcelrsError> for napi::Error` impl exists and does exactly what the manual `map_err` does — `napi::Error::from_reason(err.to_string())`. The explicit wrapper is dead code.

This applies to all 5 occurrences: 3 inside the setters (now centralized in `apply_style`), 1 in `set_columns` (calls `apply_style`), and 1 in `add_data_validation` (direct `?` fix).

### 3. Test scope for validation-error path

**Decision**: Add one test per entity (Cell, Row, Column) that passes an invalid Style and asserts `Err`.

**Rationale**: `Style::validate()` is well-tested in `style.rs`. The integration path from setter through `validate()` is the untested layer. A test like `set_style(Some(Style { num_fmt: Some("".into()), ..Default::default() }))` → `is_err()` verifies that the error propagates correctly. Minimal maintenance cost.

## Risks / Trade-offs

- **Risk**: `apply_style` adds abstraction for a 5-line pattern. **Mitigation**: It's `pub(crate)` and sits next to `Style::validate()` — discoverable, no external API surface. If the pattern ever needs a third variant (e.g., partial-merge semantics), the single function is the right place.
- **Risk**: 3 new test files increase compilation surface for test-only changes. **Mitigation**: ~10 lines per test module. Negligible impact.
- **No risk**: Runtime behavior is unchanged. The refactored code paths produce identical results. Tests prove it.
