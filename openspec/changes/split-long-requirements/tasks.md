# Tasks

## 1. Baseline and invariants

- [x] 1.1 Record the pre-change validator baseline: run `npx --yes @fission-ai/openspec@1.14.0 validate --strict --specs --no-interactive` and confirm it exits zero with exactly 22 INFO long-requirement findings across 12 capabilities, and confirm `npx --yes @fission-ai/openspec@1.14.0 list --specs` reports 41 capabilities
- [x] 1.2 Extract the full text of all 22 flagged requirements (including their scenarios) from `openspec/specs/` into a scratch reference outside the repo, to be used as the comparison baseline for every split
- [x] 1.3 Verify the target delta spec files are byte-identical in intent to what this change specifies: `openspec validate "split-long-requirements" --strict` exits zero, and confirm no delta uses `MODIFIED Requirements` (design.md D1)

## 2. Non-streaming single-capability splits

Each task in this group syncs one capability's delta into `openspec/specs/` and verifies the
capability's own findings dropped and its inventory count rose as expected.

- [x] 2.1 Sync the `cached-formula-value` delta and verify the two flagged requirements are replaced by five single-behavior requirements, that the `date_serial` branch and the `number → string → boolean → error_value → date_serial` priority order survive verbatim, and that the internal `recalc_only` flag no longer appears in observable prose (per `spec-integrity`)
- [x] 2.2 Sync the `date-cell-value` delta and verify the UTC mapping `ms = (serial - 25569) * 86400000` and its inverse appear verbatim, and that the accepted local-display-shift caveat and the `getTime()` assignment rule are each preserved
- [x] 2.3 Sync the `exceljs-parity` delta and verify all 19 feature areas and all 7 v2.0.0 exclusions (charts, pivot tables, formula evaluation, themes-write, sheet state, tab color, default worksheet properties) survive with none dropped, and that the matrix-is-authoritative status rule is retained
- [x] 2.4 Sync the `release-verification` delta and verify all seven package names appear verbatim and the "no stored credential / no `.npmrc`" rule is retained
- [x] 2.5 Sync the `tables` delta and verify all nine option fields (name, displayName, ref, headerRow with `true` default, totalsRow with `false` default, columns, rows, style) and the per-worksheet name-uniqueness rule survive
- [x] 2.6 After 2.1–2.5, run `openspec validate --strict --specs --no-interactive` and verify these five capabilities report zero long-requirement findings

## 3. Rich text and images splits

- [x] 3.1 Sync the `rich-text` delta and verify all four `Font.color` resolution paths survive verbatim (`rgb` direct, `theme="N"` via `xl/theme/theme1.xml` scheme with `tint`, `indexed="N"` via workbook palette, `auto` ⇒ `"FF000000"`), that `font.name` defaults to `null` and not `"Calibri"`, that `font.size` defaults to `null`, and that the `<u val="double"/>` ⇒ `Some(true)` rule is preserved
- [x] 3.2 Verify the `rich-text` split retains the shared-strings serialization contract (`<si><r><rPr>…</rPr><t>…</t></r></si>`, cell as `t="s"` with index in `<v>`), the no-`inlineStr` rule, and all three verification mechanisms (golden-file test, OOXML conformance smoke test, `scripts/rich-text-repro.cjs` manual step)
- [x] 3.3 Sync the `images` delta and verify the anchor contract survives verbatim: exactly one of `br` or `ext`, no public `anchorType` enum, `col`/`row` as `number` with floats allowed, and the EMU formulas `colOff = fract(col) * 64px * 9525` and `rowOff = fract(row) * 20px * 9525`
- [x] 3.4 Verify the `images` split retains the `Buffer` typing contract (declared as `Buffer` in TS, runtime accepts `Buffer` and `Array<number>`, `getImages()` returns a real `Buffer` with matching bytes) and the ECMA-376 20.5.2.27 `oneCellAnchor` `<xdr:ext>` / `twoCellAnchor` `<xdr:to>` shape rules
- [x] 3.5 After 3.1–3.4, run `openspec validate --strict --specs --no-interactive` and verify `rich-text` and `images` report zero long-requirement findings

## 4. Streaming cluster — consolidate the two-phase model

Design.md D5: state the input/output phase model once in `streaming-xlsx`; the other two
finalize specs cross-reference it.

- [x] 4.1 Sync the `streaming-xlsx` delta and verify the renamed requirement "Streaming writer buffers input sheets and streams the output phase" is the sole normative statement of the two-phase model, retaining `sheets: Vec<StreamSheet>`, O(all sheets) input peak, constant output peak, the cap-16 bounded mpsc channel, and the metadata-emitted-after-sheet-XML ordering
- [x] 4.2 Verify the `streaming-xlsx` split preserves `MAX_ENTRY_BYTES` (16 MiB) as a cap on *actual decompressed* bytes rather than the declared central-directory size, `MAX_EVENTS` (5,000,000) for per-sheet SAX events, and that the per-part cap coverage list (`xl/workbook.xml`, the workbook rels, each `sheetN.xml`, `xl/sharedStrings.xml`) is intact
- [x] 4.3 Verify the `streaming-xlsx` split preserves shared-formula resolution: per-sheet `si`-keyed collection from streaming master cells, member translation by position offset, preservation of `$A$1` and `A$1`, the bare column/row reference shifting rule, the non-reference-token preservation rule (function names, quoted strings), and the bounded-table/no-whole-sheet-materialization constraint
- [x] 4.4 Sync the `streaming-write-to-file` delta and verify its `finalizeToFile` requirement now cross-references the `streaming-xlsx` two-phase model, still states its own operative consequence (output phase holds one sheet's XML plus accumulators; input peak O(all sheets)), and retains the deferred-`writeSheet()` pointer to `streaming-write-incremental` and `docs/adr/005-streaming-write-buffering.md`
- [x] 4.5 Sync the `streaming-write-to-readable` delta and verify its `finalizeToReadable` requirement cross-references the same model, still states cap-16 backpressure and O(all sheets) input peak, and retains both deferral pointers
- [x] 4.6 Confirm the deferred input/output-phase disclaimer no longer appears as prose in `streaming-write-to-file` or `streaming-write-to-readable`, appearing only as a cross-reference (design.md D5)
- [x] 4.7 After 4.1–4.6, run `openspec validate --strict --specs --no-interactive` and verify `streaming-xlsx` (5 findings), `streaming-write-to-file` (1), and `streaming-write-to-readable` (1) all report zero

## 5. Conditional formatting and streaming-write-incremental splits

- [x] 5.1 Sync the `conditional-formatting` delta and verify all 14 `CfRule` types survive with their type-specific fields (`cellIs`, `expression`, `colorScale`, `dataBar`, `iconSet`, `top10`, `unique`, `duplicate`, `containsText`, `timePeriod`, blank/error pairs), the worksheet-global unique `priority` rule, and the `dxfId` requirement excluding `colorScale` / `dataBar` / `iconSet`
- [x] 5.2 Verify the `conditional-formatting` split retains the writer rules: `<conditionalFormatting sqref="…">` after `<sheetData>`, `<dxfs>` in `xl/styles.xml` after `cellXfs` before `tableStyles` with `count` equal to the number of `dxfs`, and the no-formats negative case
- [x] 5.3 Sync the `streaming-write-incremental` delta and verify the capability stays in `openspec/specs/` (design.md D6), retaining the ≤2 s prompt-termination guarantee, release of `ZipWriter` / `StreamSession` / bounded channel on explicit cancel and drop-without-release, exact-once in-order chunk delivery for a live consumer with cap-16 backpressure, the no-write-error / no-zip-corruption guarantee, and the `writeToWritable` `finally` teardown with its explicit "does NOT implement true incremental `writeSheet`" scope limit

## 6. Corpus-wide verification

- [x] 6.1 Run `openspec validate --strict --specs --changes --no-interactive` and verify it exits zero with no ERROR or WARNING findings and zero long-requirement INFO findings across the 41 capabilities, down from 22
- [x] 6.2 Confirm the capability inventory is still 41 via `openspec list --specs`, that no capability was added, renamed, or moved, and that `spec-integrity` is byte-identical to its pre-change content (design.md D8)
- [x] 6.3 Perform the no-behavior-loss audit (design.md Risks): for each of the 12 capabilities, diff the union of the post-split requirement texts and scenarios against the pre-change baseline from task 1.2, and confirm every behavior, enumerated item, and numeric threshold (`16 MiB`, `5,000,000`, cap 16, the seven package names, the EMU multipliers `64px`/`20px`/`9525`) is preserved with no addition or contradiction
- [x] 6.4 Confirm no file outside `openspec/specs/` was modified, and record any enumerated requirement that necessarily remains over 500 characters as a documented allowance rather than forcing a split that would lose meaning (design.md Risks)
- [ ] 6.5 Confirm the existing CI gate in `.github/workflows/ci.yml` still passes on all three runners with the rewritten corpus