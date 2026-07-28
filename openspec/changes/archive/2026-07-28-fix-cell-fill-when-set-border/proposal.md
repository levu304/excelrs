## Why

Cells that only set a `border` (without a `fill`) render with an unwanted visible fill in Excel. Root cause: the writer emits `patternType="solid"` instead of `patternType="none"` for the default/no-fill fill entry. This breaks visual output for any cell styled with border-only — they appear as "filled" with the default color.

This is a high-impact papercut: every border-only style in every workbook using excelrs produces corrupted output. Fix is one-line in writer logic.

## What Changes

- Fix `emit_fills` in `src/writer/styles.rs` to emit `patternType="none"` (not `"solid"`) when a fill has no foreground/background color
- Add tests that verify the default fill emits `patternType="none"`
- Add a round-trip test: write then read a workbook with border-only styles, assert fills are not present

## Capabilities

### New Capabilities

*(none — this is a bug fix, not a new capability)*

### Modified Capabilities

*(none — no spec-level requirement changes; this is an implementation bug with no behavioral contract change)*

## Impact

- **Single line fix** in `src/writer/styles.rs` line 538: `"solid"` → `"none"`
- Tests in `src/writer/styles.rs` need assertion for `patternType="none"` on the default fill
- A new integration test reads back a border-only workbook to verify no fill leaks
- No API surface change — `Fill` struct, `Style` struct, reader are unaffected
