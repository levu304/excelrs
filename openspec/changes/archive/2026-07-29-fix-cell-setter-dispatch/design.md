## Context

PR #45 taught the `Cell.value` setter (`src/model/cell.rs::set_value`) to accept
object-shaped values (`{ richText }`, `{ hyperlink }`, `{ formula }`,
`{ valueType }`). The writer emits a `<f>` formula element by reading
`CellInner.formula` (`xlsx.rs:1794`, `cell.rs:406-407`) — **not** the
`CellValue.formula` field the object setter populates. So `cell.value = { formula }`
produces a cell with no `<f>` and no cached `<v>` → the formula is silently gone
after `wb.xlsx.write()`.

The same dispatch arm also has unreachable parsing (dead code) and accepts any
string as `valueType`, which the writer's `_ => {}` arm turns into an empty cell.

## Goals / Non-Goals

**Goals:**

- Formulas set via the object setter persist to XLSX (emit `<f>`).
- Remove the dead parsing branches in the `valueType` arm.
- Reject unknown `valueType` discriminants instead of silently emptying the cell.
- Clear stale `inner.formula` when a non-formula value replaces a formula cell.

**Non-Goals:**

- Changing the `set_formula` / `insert_cell_formula` native path (already correct).
- Changing the read (calamine) path; write-only variants stay Null on read.
- Reworking the two-formula-field model into a single source (larger refactor).

## Decisions

**D1 — Populate `inner.formula`, mirror `insert_cell_formula`.**
The existing native formula path sets `CellInner.formula` and lets the writer emit
`<f>`. The object setter will do the same. After building `inner.value`, set:

```rust
inner.formula = if inner.value.value_type == "Formula" {
    inner.value.formula.clone()
} else {
    None
};
```

This piggybacks on the already-correct `value_type == "Formula"` produced by the
`{ formula }` key branch and the `valueType: "Formula"` arm, and also clears any
stale formula when a plain value is assigned. Alternatives considered:

- *Make the writer read `CellValue.formula` instead of `CellInner.formula* —
  larger blast radius (changes`cell.formula()` semantics used by reader + native
  `set_formula`); rejected as out of scope.
- *Set `inner.formula` only inside the `{ formula }` branch* — misses the
  `valueType: "Formula"` arm and leaves stale formulas on reassignment; rejected.

**D2 — Remove dead reads in the `valueType` arm.**
Drop `formula`/`hyperlink`/`hyperlink_text`/`rich_text` parsing at `cell.rs:336/338/341`.
These are unreachable: the `richText`/`hyperlink`/`formula` key branches
(`cell.rs:301/327/330`) consume those keys first, so by the time the `valueType`
arm runs (`:332`) they are always absent. The arm keeps only
`number`/`string`/`boolean`/`error_value`/`date_serial`.

**D3 — Validate `valueType`.**
The `valueType` arm must reject unknown discriminants. Accept only
`Number | String | Boolean | Formula | Error | Hyperlink | RichText | Date | Null | Merge`.
Unknown → `Err(ExcelrsError)` (e.g. `ExcelrsError::from_reason(...)`), surfacing
the mistake instead of writing an empty `<c>`. This is a behavior change
(silent-drop → throw) and the reason `cell-value-dispatch` is a MODIFIED capability.

**D4 — Keep silent-`Null` fallback, document it.**
Objects with no recognized key (e.g. `{ number: 5 }` without `valueType`, or a
typo'd key) still become `Null`. This is intentional inference behavior; the
proposal documents it rather than changing it.

## Risks / Trade-offs

- [Risk] Throwing on unknown `valueType` breaks any caller relying on silent drop.
  → Mitigation: only the object-setter path is affected; known discriminants
  unchanged; cover with a test asserting `Err`.
- [Risk] Clearing `inner.formula` on reassignment could drop a formula the caller
  intended to keep when setting only `value`. → Mitigation: matches ExcelJS
  semantics (`cell.value = 5` replaces the cell); `set_formula` is the dedicated API.
- [Risk] `CellValue.formula` field becomes effectively write-only metadata (writer
  ignores it). → Mitigation: D1 keeps it in sync with `inner.formula` for Formula
  cells so `cell.value.formula` reflects what was set.

## Migration Plan

Single commit touching `set_value`. No data migration, no API signature change.
Rollback = revert the setter. Add tests for formula round-trip and `valueType`
validation before merge.

## Open Questions

- None blocking. `richText` present-but-not-an-array still yields an empty run set
  (`as_array()` → `None` → `unwrap_or_default()`); left as-is (minor, documented
  in review finding #4).
