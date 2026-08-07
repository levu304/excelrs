## 1. Extract shared run-collection core

- [ ] Add `fn collect_runs<R: std::io::Read>(reader: &mut Reader<R>, scheme: &ThemeColorScheme, end_tag: &[u8]) -> Vec<RichTextRun>` in `src/reader/xlsx.rs`. It consumes events from the reader positioned at a run-container start, seeds an empty font (name/size `None`), honors `events > max_events`, and returns runs until the matching container end tag (`</is>` / `</si>`).

## 2. Rewire inline parser

- [ ] In `parse_inline_str_rich_text_with`, replace the inline `<r>` run state machine with a call to `collect_runs` per `<is>` container; delete the duplicate empty-font seed block + comment (`current_font = Font::default(); current_font.name = None; current_font.size = None;`).

## 3. Rewire shared-string parser

- [ ] In `parse_shared_string_rich_text`, replace the `<si>` run state machine with a call to `collect_runs` per `<si>`; delete the duplicate empty-font seed block + comment.

## 4. Add shared-string regression test

- [ ] Add `test_parse_shared_string_rich_text_no_rfont_no_calibri_leak` mirroring the inline one: parse a shared-string run with `<b/>` / `<sz>` but no `<rFont>` and assert `font.name === null` (and `font.size === null` when no `<sz>`). Closes the #63 review gap.

## 5. Verify

- [ ] Run `cargo test -p excelrs --lib reader` (or `cargo test rich_text`) and confirm all rich-text reader tests pass with no regressions.
