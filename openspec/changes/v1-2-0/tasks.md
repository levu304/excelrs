# Tasks: v1.2.0 — Conditional formatting

## 1. dxfs model + styles read/write

- [x] 1.1 Add `Dxf` model (optional `font` / `fill` / `border` / `num_fmt` differential subset) and `dxfs: Vec<Dxf>` to the styles model in `src/model/style.rs`
- [x] 1.2 Reader: remove `dxfs` from the skip list in `src/reader/styles.rs:577` and parse `<dxfs><dxf>…</dxf></dxfs>` into the `dxfs` collection (retain raw inner XML for unmodeled `dxf` children so foreign dxfs survive)
- [x] 1.3 Writer: emit `<dxfs count="N">` in `emit_styles_xml` (`src/writer/styles.rs`) after `cellXfs` / before `tableStyles`; omit entirely when `dxfs` is empty

## 2. Conditional formatting model

- [x] 2.1 Add `ConditionalFormat { sqref, rules: Vec<CfRule> }` and `CfRule` napi object (`type`, `priority`, `dxf_id`, `operator`, `formula`, `text`, `time_period`, `rank`, `percent`, `bottom`, `style`, `cfvo`, `color`, `data_bar_color`, `icon_set`) in `src/model/conditional_formatting.rs`
- [x] 2.2 Add `Cfvo` (`type`, `value`) and `CfColor` (`argb` / `theme` / `indexed` + optional `tint`) model types
- [x] 2.3 Store conditional formats on `Worksheet` (e.g. `Vec<ConditionalFormat>`) and validate `sqref` non-empty + unique `priority` per sheet

## 3. napi bridge — add/get

- [x] 3.1 Add `Worksheet.addConditionalFormatting(opts)` in `src/model/worksheet.rs` near `add_data_validation` (L389): append `ConditionalFormat`, assign order-based `priority` when absent, and register a `Dxf` (content-deduped) for any rule `style` → set `dxfId`
- [x] 3.2 Add `Worksheet.getConditionalFormatting()` returning `ConditionalFormat[]` (grouped by `sqref`)
- [x] 3.3 Expose `ConditionalFormat` / `CfRule` / `Cfvo` / `CfColor` JS types and `addConditionalFormatting` / `getConditionalFormatting` in `src/lib.rs` + `index.d.ts`

## 4. Reader — worksheet conditionalFormatting + dxfs resolution

- [x] 4.1 Reader: add `parse_sheet_conditional_formattings` in `src/reader/xlsx.rs` (mirror `parse_sheet_data_validations`) parsing `<conditionalFormatting sqref>` → `CfRule`s (`type`, `operator`, `priority`, `dxfId`, `formula`, `cfvo`, `color`, `iconSet`, `text`, `timePeriod`, `rank`, `percent`, `bottom`)
- [x] 4.2 Resolve each rule's `dxfId` → `Dxf` in the styles model into the rule's `style`; leave list empty when the sheet has no `<conditionalFormatting>`

## 5. Writer — worksheet conditionalFormatting + dxfs emit

- [x] 5.1 Writer: emit `<conditionalFormatting sqref="…">` (after `<sheetData>`, beside data validations) in `src/writer/xlsx.rs` with one `<cfRule>` per rule (`type`, `operator`, `priority`, `dxfId`) and type-specific children (`<formula>`, `<colorScale>`, `<dataBar>`, `<iconSet>`, `<cfvo>`, `<color>`)
- [x] 5.2 Omit `<conditionalFormatting>` for sheets with no rules; omit `<dxfs>` when empty; ensure correct worksheet + styles element ordering per D7

## 6. Round-trip fixtures

- [x] 6.1 Add ExcelJS-authored round-trip fixture: `addConditionalFormatting` across every rule type (`cellIs`, `expression`, `colorScale`, `dataBar`, `iconSet`, `top10`, `unique`, `duplicate`, `containsText`, `timePeriod`, blanks/errors/nonBlanks) → write → read; assert `sqref`, `type`, formulas/cfvo, and resolved `style` match
- [x] 6.2 Add Excel-authored `.xlsx` round-trip fixture with mixed rule types; assert all survive read→write→read (priorities + dxf styles preserved)
- [x] 6.3 Add foreign-dxf preservation fixture: a source with non-cf `dxfs` (e.g. pivot dxfs) → assert `dxfs` count/content unchanged on write

## 7. Release & parity bookkeeping

- [x] 7.1 Bump `package.json` `1.1.0` → `1.2.0`; update `CHANGELOG.md` and `ROADMAP.md` parity matrix (`conditional formatting` → `shipped`)
- [x] 7.2 Sync the `exceljs-parity` spec delta and archive the change when all fixtures pass
