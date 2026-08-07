## Context

# 48 landed `CellType::from_tag` (cell.rs:50) and migrated `value_type()` /
`set_value()`. The remaining sites listed in issue #50 do raw
`cv.value_type == "Date"` / `match cv.value_type.as_str()` comparisons that
bypass the central table. See proposal.md — Why. `value_type()` (cell.rs:319)
already returns `CellType` via `from_tag`, so the enum is always available
without re-parsing the string.

## Goals / Non-Goals

Goals: single discriminant routing through `from_tag`; behavior-neutral.

Non-Goals: no new cell types; no API/ABI changes; no perf tuning beyond
removing redundant string matching.

## Decisions

1. **Write-path sites → `from_tag`.** They run once per export, so the small
   map lookup cost is irrelevant. Single-`==` sites become
   `matches!(CellType::from_tag(x), Some(CellType::Date))` (or an equivalent
   match arm); `match cv.value_type.as_str()` sites become `match cv.value_type()`.

2. **`value()` hot path → match the enum, not string.** `value_type()` already
   returns `CellType`, so matching on it is single dispatch. This is strictly
   better than the issue's suggested "wrap the string in `from_tag` then re-match
   the `Option<CellType>`" (double dispatch on every `.value` getter call).
   - `Null` vs `None`: `from_tag("Null") == Some(Null)`, so a `Some(CellType::Null)`
     arm is needed separately from genuinely-unknown `None`.
   - Variant-data types (Formula / RichText / Hyperlink / Error / Merge) →
     `env.to_js_value(cv)`, identical to today's `_` catch-all.

## Risks / Trade-offs

- [Unknown tag string] → Mitigation: keep the `_` arm falling through to
  `env.to_js_value(cv)`; no change from today's behavior.
- [from_tag map miss vs today's literal match] → Mitigation: both treat unknown
  tags as an opaque value today; outcome is identical.

## Migration Plan

N/A — internal refactor. Rollback: `git revert`.

## Open Questions

None.
