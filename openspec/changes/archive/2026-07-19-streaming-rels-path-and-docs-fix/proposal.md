## Why

Two findings from the PR #27 review (review #4730539157) are follow-ups:

- **Issue #28** — `parse_workbook_sheet_targets` resolves a sheet file with
  `format!("xl/{}", target)` where `target` comes from
  `xl/_rels/workbook.xml.rels`. When a rels `Target` uses the absolute form
  (leading `/`, e.g. `Target="/xl/worksheets/sheet1.xml"`), the result is
  `xl//xl/worksheets/sheet1.xml` and `archive.by_name` misses the entry,
  silently yielding an empty sheet. Excel always emits relative targets, so
  this only affects non-Excel producers — but it is a real silent-data-loss
  bug on the streaming read path.
- **Issue #30** — the doc comment "Max SAX events per sheet (anti-billion-row /
  entity-expansion guard)." is attached to `MAX_ENTRY_BYTES`'s `///` block
  (no blank line between), so `const MAX_EVENTS` ends up undocumented.

## What Changes

- Make sheet-path construction in `parse_workbook_sheet_targets` tolerant of
  absolute rels `Target` values (package-rooted paths), resolving them
  correctly instead of producing a doubled `xl/` prefix.
- Move the misplaced doc comment from `MAX_ENTRY_BYTES` to a `///` directly
  above `const MAX_EVENTS`.

Both are non-breaking, implementation-only fixes.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

(none — implementation fixes only; no requirement-level behavior change)

## Impact

- `src/stream.rs`: `parse_workbook_sheet_targets` (path build) and the
  `MAX_EVENTS` doc comment.
- No public API surface change, no FFI change, no dependency change.
