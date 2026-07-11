## Why

The v0.4.0 roadmap (spec §9.2.1) shipped all six deferred style items by v0.6.0. The next roadmap capability (§9.3) with the highest value for exceljs drop-in compatibility is **defined names** (named ranges). ExcelJS users can create, read, and resolve named ranges via `workbook.definedNames`; excelrs currently ignores the `<definedNames>` element in `xl/workbook.xml`, silently dropping them on read and emitting no `<definedNames>` on write.

## What Changes

- New `DefinedName` model type (`name: String`, `value: String`, optional `sheet: String` for sheet-scoped names).
- **Reader**: parse `xl/workbook.xml`'s `<definedNames>` element, extracting `<definedName name="..." [localSheetId="N"]>text</definedName>` entries. Map `localSheetId` to sheet name via the workbook's sheet order.
- **Writer**: emit `<definedNames>` after `<sheets>` in `xl/workbook.xml`. Omit if no names defined.
- **NAPI**: `Workbook.definedNames` getter, `Workbook.addDefinedName(name, value, sheet?)`, `Workbook.removeDefinedName(name, sheet?)`, `Workbook.getDefinedName(name, sheet?)`.
- **index.d.ts**: New `DefinedName` interface, methods on `Workbook`.
- **No breaking changes** — all additions are additive.

## Capabilities

### New Capabilities

- `defined-names`: Reading and writing workbook-level and sheet-scoped defined names (named ranges). Covers parsing `<definedNames>` from `xl/workbook.xml`, exposing them via the Workbook JS API, and emitting `<definedNames>` in the written OOXML archive.

### Modified Capabilities
<!-- No existing capabilities change requirement-wise. -->

## Impact

- **Code**: New `src/model/defined_name.rs` (`DefinedName` struct). `WorkbookInner` gains `defined_names: Vec<DefinedName>`. Reader adds `parse_defined_names(data, sheet_names)` in `src/reader/` (reads `xl/workbook.xml` from zip). Writer `write_workbook_xml` emits `<definedNames>`. `Workbook` (napi) gains 4 methods. `index.d.ts` adds `DefinedName` interface.
- **API**: Additive. No existing signatures change.
- **Dependencies**: None (uses existing `quick_xml` and `zip`).
- **Tests**: ~15 new Rust (unit: parse, round-trip; edge: empty, sheet-scope, dupes) + ~6 new JS (`__test__/defined-names.test.ts`). Baseline 173→188 Rust, 69→75 JS.
- **Spec**: `docs/spec.md` §6 (new sub-section), §9.3 mark defined-names shipped, §1 version note.
