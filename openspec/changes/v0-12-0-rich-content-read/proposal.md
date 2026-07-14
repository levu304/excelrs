## Why

`excelrs` already *emits* rich text, gradient fills, and diagonal borders on
write (since v0.5.0), but the reader silently drops all three on read
(`reader/styles.rs` skips `gradientFill` and diagonal attributes; `map_data`
collapses inline rich-text strings). A file produced by `excelrs` therefore
loses formatting when read back — a broken round-trip that undermines the
drop-in ExcelJS compatibility promise. v0.12.0 closes these three read/write
gaps with low-risk, same-shape parser work. **JS Date preservation is
deliberately deferred to v0.13.0** (it is a separate FFI type-bridging effort,
not a round-trip gap).

## What Changes

- **RichText read**: The reader SHALL parse inline (`<is>`) and shared-string
  (`<si>`) rich text into `CellValue.rich_text` (`Vec<RichTextRun>`), recovering
  per-run text and font. (Write already ships.)
- **Gradient fill read**: The styles reader SHALL parse `<gradientFill>` into
  the existing `Fill` gradient fields (`gradient_type`, `gradient_degree` /
  `gradient_angle`, `gradient_stops`, path geometry) instead of skipping it.
  (Write already ships.)
- **Diagonal border read**: The styles reader SHALL parse `<diagonal>` plus the
  `diagonalUp` / `diagonalDown` attributes on `<border>` into the existing
  `Border.diagonal*`, instead of skipping them. (Write already ships.)
- **Parity matrix**: `exceljs-parity` advances rich-text, gradient fill, and
  diagonal border from `partial` to `shipped`.

No breaking changes. All three model structs and writers already exist; this
change is read-side only.

## Capabilities

### New Capabilities

- `rich-text`: Rich-text cell content round-trip — `CellValue.rich_text` runs with per-run `Font`, parsed on read and emitted on write.
- `gradient-fill`: Gradient cell fill round-trip — `Fill` gradient fields parsed on read and emitted on write.
- `diagonal-border`: Diagonal cell border round-trip — `Border.diagonal*` parsed on read and emitted on write.

### Modified Capabilities

- `exceljs-parity`: Parity matrix status advances for `rich-text`, `gradient fill`, and `diagonal border` (each `partial` → `shipped`).

## Impact

- **Reader**: `src/reader/styles.rs` (gradient + diagonal parse branches), `src/reader/xlsx.rs` (`map_data` / cell-value builder for rich text).
- **Model**: `src/model/cell.rs` (`RichTextRun`, `CellValue.rich_text` — already present, now populated on read), `src/model/style.rs` (already complete; no change).
- **Writer**: `src/writer/styles.rs`, `src/writer/xlsx.rs` — no change; already emit these features.
- **Tests**: round-trip fixtures for each of the three; existing skip-comment tests in `reader/styles.rs` updated.
- **Out of scope**: JS `Date` value type, streaming, charts, pivot tables (deferred per roadmap).
