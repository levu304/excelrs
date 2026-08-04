## Why <!-- Explain motivation for change. What problem this solve? Why now? -->

excelrs rich-text read resolves run fonts to `name/size/bold/italic/underline/color(rgb only)`. Real-world `.xlsx` files authored by Excel (not just ExcelJS) use theme colors (`<color theme="N" tint="T"/>`), indexed palette colors (`<color indexed="N"/>`), `<color auto/>`, and explicit off-flags (`<b val="0"/>`, `<u val="none"/>`). Today these degrade: theme/indexed colors silently yield `null`/`undefined`, and `val="0"` is misread as "on". PR #58 (`fix/shared-strings-rich-text-read`) landed the shared-strings read path but intentionally kept the rgb-only surface that the inline-string parser already had — consolidating both onto one `apply_rpr_child` helper. Now is the moment: the helper is single-chokepoint, theme1.xml resolution already exists in the tree, and the gap is a one-function edit.

## What Changes

- `apply_rpr_child` (single shared helper used by inline-string `parse_inline_str_rich_text` AND shared-string `parse_shared_string_rich_text`) gains `val`-aware bold/italic/underline AND theme/tint/indexed/`auto` color resolution.
- A `ThemeColorScheme` is loaded once in `workbook_inner_from_bytes` and passed into both rich-text parsers (replacing the duplicated per-run `Font::default()` drift with a shared resolver) so inline and shared-string runs resolve theme colors identically.
- `Font.color` now holds resolved ARGB (`FFRRGGBB`) for themed/indexed/auto colors, matching the existing public API (string) — no napi shape change.
- New tests cover `<b val="0"/>`, `<color theme="4" tint="0.5"/>`, `<color indexed="..."/>`, `<color auto/>`, and mixed runs, on both inline-string and shared-string paths.

## Capabilities

### New Capabilities

- none (no new capability; extends existing `rich-text` read surface).

### Modified Capabilities

- `rich-text`: run-level `Font` color resolution expands from rgb-only to also resolve `<color theme="N" tint="T"/>`, `indexed="N"`, `auto` to ARGB via the workbook theme scheme; `bold`/`italic`/`underline` now honor `val` (`0`/`false`/`none` ⇒ flag `Some(false)` instead of `Some(true)`).

## Impact

- **Code**: `src/reader/xlsx.rs` (`apply_rpr_child`, `parse_inline_str_rich_text`, `parse_shared_string_rich_text`, call sites in `workbook_inner_from_bytes`); reuses `ThemeColorScheme::resolve_theme`/`resolve_indexed` from `src/model/color.rs` (no new dependency).
- **API**: `Font.color` type unchanged (still `Option<String>` ARGB); no `index.d.ts`/`napi` surface change.
- **Behavior**: richer text on read for Excel-authored files; excelrs round-trip unaffected (writer still emits `rgb`/`<b/>` only when true).
- **Tests**: new Rust cases + one ExcelJS-authored themed fixture if available.
