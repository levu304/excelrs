## Why

When reading an `.xlsx` with rich-text cells, a run whose `<rPr>` contains
formatting (e.g. `<b/>`, `<sz/>`) but no explicit `<rFont>` element is read back
with `font.name === "Calibri"` — excelrs's default font — **injected**. The
reader seeds each run's font from `Font::default()` and only overrides `name`
when an `<rFont>` is present (`apply_rpr_child` in `src/reader/xlsx.rs`).

Consequence: on a read → write round trip, runs that had no font name in the
source gain a spurious `Calibri` font name, changing the rendered output — the
run should instead inherit the cell's default font (correct OOXML semantics).
This was introduced by commit 1f55235 (rich-text shared-string reader), which
seeds `Font::default()` at both the inline and shared-string parse paths.

Reported as: rich-text run font shows `Calibri` instead of inheriting the
cell's font after a read/write cycle.

## What Changes

- Reader seeds each rich-text run's font as **empty** (no default name/size)
  instead of `Font::default()` at run start in both parse functions.
- A run whose `<rPr>` has no `<rFont>` reads back with `font.name === null`
  (not `"Calibri"`), preserving "inherit cell default font" semantics.
- Writer is unchanged: it already emits `<rFont>`/`<sz>` only when the field is
  `Some`, so a `null`-name run is written correctly without an explicit font.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `rich-text`: clarify that `font.name` is `null` (not the default `"Calibri"`)
  when a run's `<rPr>` lacks an `<rFont>`. Delta added under
  `specs/rich-text/spec.md`.

## Impact

- No public API change. `RichTextRun.font` is already `Option<Font>`.
- Behavior change is read-side only: previously a no-`<rFont>` run leaked
  `Calibri`; now it inherits the cell default font.
- Blast radius is low: only `parse_inline_str_rich_text_with`
  (`src/reader/xlsx.rs:2447`) and `parse_shared_string_rich_text`
  (`src/reader/xlsx.rs:2554`) are touched. Both are private and used solely by
  the rich-text read path.
- One new regression test: read a run with `<b/>` only → assert
  `font.name === null`.
