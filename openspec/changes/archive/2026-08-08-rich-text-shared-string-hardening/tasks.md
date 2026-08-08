## 1. Rendered-run dedup key

- [x] Add `RenderedRunKey` struct in `src/writer/xlsx.rs` with only the rendered
  fields (`name`, `size`, `bold`, `italic`, `underline`, `color`), deriving
  `PartialEq`, `Eq`, `Hash` (the `Hash` impl normalizes `size` signed zero so
  `+0.0` and `-0.0` hash equally).
- [x] Change `SharedString::Rich(Vec<RichTextRun>)` to
  `SharedString::Rich(Vec<RenderedRunKey>)`; project each `RichTextRun` into a
  `RenderedRunKey` using exactly the field set `write_rich_run_xml` emits
  (single source of truth).
- [x] Update `build_shared_strings` and `write_cell_xml` to build/lookup the
  projected key.
- [x] Remove the hand-rolled `Font: Hash` (`to_bits`) and manual `Eq`, and the
  `PartialEq` derive added in PR #64 from `src/model/style.rs`; keep
  `Font::validate` unchanged.
- [x] Delete the now-inaccurate "no NaN keys" comment in `src/model/style.rs`.

## 2. Streaming-writer guardrail

- [x] In `src/stream.rs`, document on `StreamValue` / the streaming writer that
  rich-text write, when added, MUST route through shared strings (`t="s"`)
  reusing `write_rich_run_xml` / `SharedString`, never `inlineStr`.
- [x] Make the streaming `write_shared_strings` `SharedString`-aware (share the
  same type and `RenderedRunKey` logic as the non-streaming writer) so the two
  paths cannot diverge.
- [x] Add a forward-looking unit test asserting `t="s"` (not `t="inlineStr"`)
  for streaming rich text, gated on the future `RichText` variant.

## 3. Confidence tests (Numbers compatibility)

- [x] Golden-file test: emit a known rich-text workbook; assert the exact
  `xl/sharedStrings.xml` and `xl/worksheets/sheetN.xml` (run `<rPr>` contents,
  `xml:space="preserve"`, `t="s"` + `<v>`).
- [x] Conformance smoke test: validate the generated workbook against the OOXML
  XSD and/or open it headless via `soffice --headless --convert-to`. Make it
  feature-gated / optional so CI does not hard-fail where LibreOffice is
  unavailable; document XSD validation as the fallback.
- [x] Keep `scripts/rich-text-repro.cjs` as the documented manual Apple Numbers
  verification step; reference it from the change docs.

## 4. Docs / cleanup

- [x] Update CHANGELOG / change summary to note the dedup-key hardening and the
  streaming guardrail.
- [x] Confirm the two corrected PR #64 review comments (color_tint reachability;
  rPr element-order false positive) are consistent with the final
  implementation.
