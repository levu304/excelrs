## Why

`excelrs`' rich-text spec (`openspec/specs/rich-text/spec.md`) requires the reader to parse rich-text runs from **both** inline strings (`<is><r>`) **and** shared strings (`<si><r>`). Only inline strings are implemented — `parse_inline_str_rich_text_with()` scans `<c t="inlineStr"><is>` in worksheet XML and never touches `xl/sharedStrings.xml`.

Excel (the application) and other XLSX producers store rich text as shared strings (`<si><r><rPr>...</rPr><t>...</t></r></si>`), not inline strings. Calamine 0.35 — excelrs's read engine — has **no `Data::Richtext` variant**; it collapses shared strings to `Data::String(plain)`, losing all run-level font formatting. So opening an Excel-generated file with rich-text cells yields `value_type === "String"` with **no font information**, and writing it back strips the fonts entirely.

Commit `5233f6e` fixed the in-memory **writer** rPr ordering + `<u/>`, and round-trip tests pass for excelrs→XLSX→excelrs (which uses inline strings on both sides). But the reader cannot ingest shared-strings rich text from Excel files, so the font "is still not changed" on real-world files.

## What Changes

- Add shared-strings rich-text parsing: read `<si><r><rPr>...</rPr><t>...</t></r></si>` entries from `xl/sharedStrings.xml`, producing `Vec<RichTextRun>` per string-index.
- Extend the cell-resolution step so that when calamine returns a cell as `Data::String` (shared string index resolved), the reader checks the pre-parsed shared-strings rich-text table for that index and, if present, upgrades the `CellValue` to `value_type === "RichText"` with the runs + per-run `Font`.
- Reuse the existing `Font` model and `RichTextRun` struct (no new types) — the same `<rPr>` → `Font` mapping already used by the inline-string parser.
- No public-API changes; the gap is purely read-side fidelity for Excel-generated files.

## Non-Goals

- Streaming-path rich text support (`StreamValue`/`JsStreamValue` lack a rich-text variant) — out of scope; belongs to a streaming-capability change.
- Writer-side changes — the in-memory writer (commit `5233f6e`) is correct for inline strings.
- `Font::default()` injection (Calibri/11 leaking into runs) — a separate, minor read-side quality issue.
- ExcelJS round-trip — ExcelJS writes rich text as inline strings, which already round-trips correctly.

## Capabilities

### New Capabilities
<!-- No new capability introduced; this fixes a gap in an existing capability. -->

### Modified Capabilities

- `rich-text` — advances the spec requirement "reader SHALL parse rich-text … shared-string `<si><r>` runs" from **unimplemented** to **shipped** for the reader. No spec amendment needed; the requirement was always there.

## Impact

- **Code**: `src/reader/xlsx.rs` — add shared-strings rich-text parse + merge into cell resolution. ~80 LOC.
- **Tests**: new Rust unit test for shared-strings `<si><r>` parsing; new TS round-trip fixture reading an ExcelJS-shared-strings rich-text file.
- **Public API**: none changed.
- **Build/deps**: none.
