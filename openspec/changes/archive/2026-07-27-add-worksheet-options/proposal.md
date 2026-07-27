## Why

`workbook.addWorksheet(name)` only accepts a sheet name. Callers who want to
set page setup, views (freeze panes, grid lines), header/footer, protection,
or auto-filter at creation time must make a second write-after-create call:

```typescript
const ws = workbook.addWorksheet("Sheet1");
ws.pageSetup = { paperSize: 9, orientation: "landscape", ... };
ws.views = [{ showGridLines: false }];
```

This is a papercut vs. ExcelJS, where `addWorksheet(name, options)` accepts
an `AddWorksheetOptions` object. The two-step pattern is easy to forget and
forces extra napi-rs round-trips. A single-call API is simpler, matches
user expectations, and is the idiomatic JS pattern.

## Changes

- **New `AddWorksheetOptions` struct** exposed as a JS object parameter on
  `addWorksheet(name, options?)`. Fields mirror worksheet creation-time
  properties: `pageSetup`, `views`, `headerFooter`, `protection`,
  `autoFilter`.
- **`SheetView` gains `showGridLines`** — the OOXML `<sheetView>` attribute
  exists on the model, parseable from files, emitted on write.
- **`addWorksheet` overload** — the existing `(name: string)` signature is
  extended to `(name: string, options?: AddWorksheetOptions)`. No breaking
  change; existing single-arg callers continue to work unchanged.

## Capabilities

### New Capabilities

- `add-worksheet-options`: Pass page setup, views, header/footer, protection,
  and auto-filter at worksheet creation time via a single `addWorksheet` call
  with an optional second argument.

### Modified Capabilities

- `worksheet-views`: Add `showGridLines` boolean field to `SheetView` so it
  can be read, written, and round-tripped through the OOXML reader/writer.

## Impact

| Area | Impact |
| ------ | -------- |
| `src/model/workbook.rs` | Add `AddWorksheetOptions` struct, modify `add_worksheet` to accept `Option<AddWorksheetOptions>` |
| `src/model/workbook_inner.rs` | Modify `add_worksheet` to accept and apply options |
| `src/model/sheet_view.rs` | Add `show_grid_lines: Option<bool>` field |
| `src/model/worksheet.rs` | No direct changes (setters already exist) |
| `src/reader/xlsx.rs` | Parse `showGridLines` attribute in `parse_views_from_xml` |
| `src/writer/xlsx.rs` | Emit `showGridLines` attribute in `emit_sheet_views` |
| `index.d.ts` / `native.d.ts` | Add `AddWorksheetOptions` interface, update `addWorksheet` signature, add `showGridLines` to `SheetView` |
| `native.js` | No change — napi-rs auto-generates JS glue; the TS declarations need manual update |
| `openspec/specs/worksheet-views/spec.md` | Add `showGridLines` requirements |
