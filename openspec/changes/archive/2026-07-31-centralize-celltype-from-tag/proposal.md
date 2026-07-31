## Why

The `CellValue.value_type` discriminant string (one of ten literals: `"Null"`, `"Number"`, `"String"`, `"Boolean"`, `"Date"`, `"Formula"`, `"Error"`, `"Hyperlink"`, `"RichText"`, `"Merge"`) is hand-listed in **8 separate match/literal sites** across `cell.rs`, `csv.rs`, `table.rs`, and `writer/xlsx.rs`. Adding or renaming a variant requires coordinated edits in N places, risking silent misclassification: only `set_value()` validates and rejects unknown tags; all other sites silently fall through to a `_` catch-all, so a missed arm produces a `Null` cell rather than an error.

**Why now:** v2.5.1 established the `CellType` enum as the canonical JS-facing discriminant (`#[napi(string_enum)]`). There is no Rust-side `from_tag`/`from_str` — each site re-implements the same 10-arm table. Centralizing this is a small, behavior-neutral refactor that eliminates the drift surface before the enum grows again.

## What Changes

- Add a single `pub(crate) fn from_tag(tag: &str) -> Option<CellType>` method on `CellType` in `src/model/cell.rs` — one 10-arm match table.
- **Modified:** `Cell::value_type()` — replace 11-arm inline match with `CellType::from_tag(...).unwrap_or(CellType::Null)`.
- **Modified:** `Cell::set_value()` — replace the 10-arm `matches!(vt, ...)` validation list with `CellType::from_tag(vt).is_some()`.
- **Follow-up (lower priority, behavior-dispatch sites):** `csv.rs::cell_value_to_text`, `table.rs::cell_text`, `writer/xlsx.rs::build_shared_strings` and `write_cell_xml` — these are dispatch tables (not validation), so migration is optional and can be a separate change if desired.

Behavior-neutral: identical runtime behavior, same fallback semantics, no new dependencies. No breaking API changes.

## Capabilities

### New Capabilities

(none — pure refactoring, `skip_specs: true`)

### Modified Capabilities

(none — no spec-level behavior changes)

## Impact

- **Affected code:** `src/model/cell.rs` (CellType enum, `value()`, `value_type()`, `set_value()`). Optionally: `src/csv.rs`, `src/model/table.rs`, `src/writer/xlsx.rs` as follow-up.
- **API surface:** No changes to JS-facing TypeScript declarations (`index.d.ts`). `CellType` is a `string_enum` — `from_tag` is `pub(crate)`, not exported.
- **Dependencies:** None.
- **Tests:** Existing Rust unit tests and Vitest integration tests remain green unchanged (behavior-neutral refactor). No new tests needed beyond confirming the central table is used.
- **Risk level:** Low. Single-file change with mechanical replacements.
