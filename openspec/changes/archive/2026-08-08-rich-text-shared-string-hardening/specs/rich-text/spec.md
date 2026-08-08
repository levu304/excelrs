## ADDED Requirements

### Requirement: Rich-text dedup key uses rendered fields only

When computing the shared-string dedup key for a rich-text cell, the writer
SHALL derive the key from the *rendered* run projection only — the fields
`write_rich_run_xml` actually emits: `name`, `size`, `bold`, `italic`,
`underline`, `color`. The writer SHALL NOT include unrendered fields
(`color_theme`, `color_tint`) in the key. Two runs that render identically SHALL
share one shared-string entry even if they differ only in unrendered fields.

#### Scenario: Theme/tint-only differences collapse to one entry

- **WHEN** two rich-text runs have identical rendered fields but differ only in
  `color_theme` / `color_tint` (e.g. both render as `Arial`/`FF0000FF`, one
  carries an internal theme link the other does not)
- **THEN** the writer SHALL assign them the same shared-string index (one
  `<si>` entry), not two.

#### Scenario: Dedup key is independent of validation

- **WHEN** the dedup key is built
- **THEN** its correctness SHALL NOT depend on `Font::validate` covering every
  keyed field; only emitted fields participate in the key.

### Requirement: Streaming writer emits rich text via shared strings

When the streaming writer supports rich-text write (a future `RichText` value
on the streaming path), it SHALL serialize runs into the shared-string table
as `<si><r><rPr>…</rPr><t>…</t></r></si>` and emit the cell as `t="s"` with the
shared-string index in `<v>…</v>`, reusing the same render/key logic as the
non-streaming writer. The streaming writer SHALL NOT emit rich text as inline
strings (`t="inlineStr"`).

#### Scenario: Streaming rich text uses shared strings, not inline

- **WHEN** a streaming rich-text cell is written
- **THEN** the cell element SHALL use `t="s"` with `<v>idx</v>` (not
  `t="inlineStr"`), and `xl/sharedStrings.xml` SHALL contain the run's `<r>`
  with its `<rPr>` and `<t>`.

#### Scenario: Streaming and non-streaming paths stay consistent

- **WHEN** the same rich-text content is written via the streaming and
  non-streaming writers
- **THEN** both SHALL produce shared-string rich text with identical rendering
  (no path emits inlineStr).

### Requirement: Writer output is verifiable for cross-app compatibility

The rich-text shared-string writer SHALL be covered by automated confidence
checks that catch regressions strict consumers (e.g. Apple Numbers) would hit:
(a) a golden-file test asserting the exact emitted `xl/sharedStrings.xml` and
`xl/worksheets/sheetN.xml` for a known rich-text cell (run `<rPr>` contents,
`xml:space="preserve"`, `t="s"` + `<v>`); and (b) an OOXML conformance smoke
test (validating the generated workbook against the OOXML schema and/or opening
it headless in LibreOffice). A manual open in Apple Numbers (via
`scripts/rich-text-repro.cjs`) SHALL remain a documented verification step.

#### Scenario: Golden-file asserts exact shared-string output

- **WHEN** a known rich-text workbook is written
- **THEN** the golden-file test SHALL assert the exact `sharedStrings.xml` and
  sheet XML, so a revert to `inlineStr` or a change to run properties fails CI.

#### Scenario: Conformance smoke test passes

- **WHEN** the generated workbook is checked for OOXML conformance
- **THEN** it SHALL pass schema/XSD validation and/or open without error in
  headless LibreOffice.
