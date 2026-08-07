## 1. Migrate `writer/xlsx.rs` sites

- [x] Replace the 6 `value_type` string matches with `from_tag` / enum routing:
  - `writer/xlsx.rs:154` `if cv.value_type == "Date"` → `matches!(CellType::from_tag(&cv.value_type), Some(CellType::Date))`
  - `writer/xlsx.rs:382` `match cv.value_type.as_str()` → `match CellType::from_tag(&cv.value_type)`
  - `writer/xlsx.rs:429` `if cv.value_type == "Hyperlink"` → `matches!(..., Some(CellType::Hyperlink))`
  - `writer/xlsx.rs:1800` `match cv.value_type.as_str()` → `match CellType::from_tag(&cv.value_type)`
  - `writer/xlsx.rs:1840` `match cv.value_type.as_str()` → `match CellType::from_tag(&cv.value_type)`
  - Always keep a `_` arm falling through to the existing behavior.

## 2. Migrate `csv.rs` and `table.rs` sites

- [x] `csv.rs:261` `cell_value_to_text` `value.value_type.as_str()` → `CellType::from_tag(...)`.
- [x] `table.rs:87` `cell_text` `cv.value_type.as_str()` → `CellType::from_tag(...)`.

## 3. Migrate `formula/bridge.rs` and `worksheet.rs` sites

- [x] `bridge.rs:66` `match cv.value_type.as_str()` → enum match.
- [x] `bridge.rs:114-138` internal `cv.value_type = "X"` set sites → use `CellType` variant
      constructors if available, else leave as string assignment (document why).
- [x] `bridge.rs:337` `if cv.value_type == "Formula"` → `matches!(..., Some(CellType::Formula))`.
- [x] `worksheet.rs:195` and `worksheet.rs:1074` `value_type` comparisons → `from_tag` / enum match.

## 4. Migrate `cell.rs` sites (incl. `value()` hot path)

- [x] Non-hot sites:
  - `cell.rs:331` `if cv.value_type == "Date"` → enum match.
  - `cell.rs:450` `value.value_type == "Formula"` → enum match.
  - `cell.rs:496` `cv.value_type != "Formula"` → enum match.
  - `cell.rs:662` `== "Null"` → `matches!(CellType::from_tag(...), Some(CellType::Null))`.
- [x] `value()` hot path `cell.rs:277` `match cv.value_type.as_str()` →
      `match self.value_type()` (enum returned by `value_type()`).
      Add a `Some(CellType::Null)` arm distinct from `None`; route
      Formula/RichText/Hyperlink/Error/Merge to `env.to_js_value(cv)` via `_`.

## 5. Verify behavior-neutral

- [x] `cargo test --features formula-eval` passes.
- [x] `napi build` (or `npm run build`) succeeds; call site types unchanged.
- [x] Run the 154 JS test suite; confirm no `value_type` round-trip / `value()`
      drifts (formulas, rich text, hyperlinks, dates, errors all unaffected).
- [x] Grep confirms no remaining `value_type == "<Lit>"` / `value_type.as_str()`
      outside tests and `bridge.rs` intentionally-kept set sites.
