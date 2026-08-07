## Why

# 48 introduced `CellType::from_tag` (cell.rs:50) as the single discriminant-tag
table and migrated `value_type()` + `set_value()`. Eleven `value_type`
string-literal matches remain scattered across write paths
(`writer/xlsx.rs`, `csv.rs`, `table.rs`, `bridge.rs`) and one per-cell read
hot path (`value()` in cell.rs). These duplicate discriminant logic and bypass
the central table, so a tag added to `from_tag` is not reflected at these sites.
This closes the #48 tail (issue #50) and is the prerequisite for deleting the
old string-match branches.

## What Changes

- Migrate the remaining `value_type` string comparisons to `CellType::from_tag`
  at all write-path / one-shot sites: `writer/xlsx.rs` (6 sites), `csv.rs:261`,
  `table.rs:87`, `bridge.rs` (set + read sites), `cell.rs` (non-hot sites),
  `worksheet.rs` (2 sites).
- `value()` hot path (cell.rs:277): match on the `CellType` enum returned by the
  existing `value_type()` getter (single dispatch), not a
  string → `from_tag` → `Option<CellType>` re-match (avoids double dispatch).
- Guard the `_` arm so unknown tag strings still fall through to
  `env.to_js_value(cv)` (behavior-neutral).
- No public API, ABI, or behavior change. No `CellType` variants added/removed.

## Capabilities

Pure refactor — no spec-level behavior changes. `skip_specs: true` is declared
in `.openspec.yaml`; no delta specs are created.

### New Capabilities

(none)

### Modified Capabilities

(none)

## Impact

- Affected: `src/model/cell.rs`, `src/model/table.rs`, `src/model/worksheet.rs`,
  `src/writer/xlsx.rs`, `src/csv.rs`, `src/formula/bridge.rs`.
- No dependency, public API, or feature-flag (`formula-eval`) changes.
- Risk: low. All sites are behavior-neutral string-tag reroutes through the
  existing `from_tag` table.
