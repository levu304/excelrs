## 1. Seed empty font at inline rich-text run start

- [x] In `parse_inline_str_rich_text_with` (`src/reader/xlsx.rs:2447`), replace
      `current_font = Font::default();` with an empty-font seed: clear `name`
      and `size` (e.g. `Font::default()` then `current_font.name = None;
      current_font.size = None;`) so a run without `<rFont>` reads back
      `font.name === null`.

## 2. Seed empty font at shared-string rich-text run start

- [x] In `parse_shared_string_rich_text` (`src/reader/xlsx.rs:2554`), apply the
      same empty-font seed used in task 1.

## 3. Add regression test

- [x] Add a reader test (near `test_parse_inline_str_rich_text_run_font_name`)
      that parses an inline rich-text run with `<b/>` but no `<rFont>` and
      asserts `run.font.name === null` (was `"Calibri"`).

## 4. Verify

- [x] Run `cargo test -p excelrs --lib reader` (or `cargo test rich_text`) and
      confirm the new test passes and no existing rich-text reader tests
      regress.
