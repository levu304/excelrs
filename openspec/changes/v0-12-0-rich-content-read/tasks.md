## 1. Spike — confirm rich-text read path (Q1)

- [x] 1.1 Verify whether `calamine 0.35` exposes rich text as a distinct `Data` variant (e.g. `Data::Richtext`) or collapses it to `Data::String`. This decides whether RichText read uses `map_data` (D3 clean path) or direct `<is>` parsing (D3 fallback).

## 2. Diagonal border read

- [x] 2.1 In `src/reader/styles.rs::parse_style_table`, add `diagonal` to the `left|right|top|bottom` border-side match so the `<diagonal>` style is captured into `Border.diagonal` (same accumulator pattern as the other sides).
- [x] 2.2 Capture `diagonalUp` / `diagonalDown` boolean attributes off the `<border>` element into `Border.diagonal_up` / `Border.diagonal_down`.
- [x] 2.3 Add a unit test in `reader/styles.rs` round-tripping a border with `diagonal` + `diagonalUp`/`diagonalDown` through `parse_style_table` → `resolve_style`.

## 3. Gradient fill read

- [x] 3.1 Replace the `b"gradientFill" => { /* skip */ }` no-op in `parse_style_table` with a parser that reads `<gradientFill type="linear|path">` and its `<stop position color>` children into `Fill` (`kind="gradient"`, `gradient_type`, `gradient_stops`).
- [x] 3.2 For linear gradients read `degree` into `gradient_degree`; for path gradients read `left/right/top/bottom` geometry. Leave `gradient_angle` deprecated/unread (Decision D1 — mirror the writer).
- [x] 3.3 Add unit tests in `reader/styles.rs` for linear and path gradient fills through `parse_style_table` → `resolve_style`.

## 4. RichText read

- [x] (skip) 4.1 Q1 confirmed: calamine 0.35 has NO `Data::Richtext` variant — must use direct `<is>` parsing.
- [x] 4.2 Parse `xl/worksheets/sheetN.xml` `<c><is><r>` inline strings directly with `parse_inline_str_rich_text()` in xlsx.rs.
- [x] 4.3 Map each run's `<rPr>` font (name/size/bold/italic/underline/color) into `RichTextRun.font` using the existing `Font` model.
- [x] 4.4 Add a round-trip test: write rich text via `CellValue.rich_text`, read back, assert `value_type === "RichText"` and run count/text/font match.

## 5. Docs & parity matrix

- [x] 5.1 Update the `reader/styles.rs` "v0.3.0 limitations" doc comment to mark gradient fills and diagonal borders as resolved (v0.12.0).
- [x] 5.2 Advance `ROADMAP.md` parity matrix: `rich-text`, `gradient fill`, and `diagonal border` move `partial` → `shipped` (v0.12.0). The `exceljs-parity` spec delta is already captured in this change.

## 6. Round-trip validation

- [x] 6.1 Add write→read round-trip fixtures covering all three features (gradient fill, diagonal border, rich text).
- [x] 6.2 Run `cargo test` napi build; confirm no regression previously-skipped reader tests now pass.
