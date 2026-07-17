# Tasks: v1.0.0 — Drop-in ExcelJS-compat milestone

## 1. Headers & footers

- [x] 1.1 Add `HeaderFooter` model type in `src/model/` (odd/even/first header+footer + `alignWithMargins`/`differentFirst`/`differentOddEven`); store format codes verbatim
- [x] 1.2 Writer: emit `<headerFooter>` with only present odd/even/first children at the CT_Worksheet position (after sheet views, before pageMargins)
- [x] 1.3 Reader: parse `<headerFooter>` from `xl/worksheets/sheetN.xml` into `ws.headerFooter`; leave `null` when absent
- [x] 1.4 napi bridge: expose `ws.headerFooter` getter/setter; add round-trip fixture (Excel-authored + ExcelJS-authored)

## 2. Page setup / print

- [x] 2.1 Add `PageSetup` model type (orientation, paperSize, fitToPage/Width/Height, dpi, margins, printArea, printTitles)
- [x] 2.2 Writer: emit `<pageMargins>` + `<pageSetup>`; register `printArea`/`printTitles` as `_xlnm.Print_Area` / `_xlnm.Print_Titles` defined names
- [x] 2.3 Reader: parse `<pageMargins>`/`<pageSetup>`; resolve `_xlnm.Print_*` defined names back to `ws.pageSetup`
- [x] 2.4 napi bridge: expose `ws.pageSetup`; add round-trip fixture

## 3. Workbook views & calc properties

- [x] 3.1 Add `WorkbookView` + `CalcProperties` model types on `Workbook`
- [x] 3.2 Writer: emit `<bookViews><workbookView .../></bookViews>` and `<calcPr .../>` in `xl/workbook.xml` (after `sheets`, before `definedNames`); omit when unset
- [x] 3.3 Reader: parse `<bookViews>`/`<calcPr>` into `wb.views`/`wb.calcProperties`; default gracefully when absent
- [x] 3.4 napi bridge: expose `wb.views` + `wb.calcProperties`; add round-trip fixture

## 4. Comments

- [x] 4.1 Add `Comment` model type (text, author, anchor ref) and per-cell `note`/`comment` accessor
- [x] 4.2 Writer: emit `xl/commentsN.xml` (`<commentList>` + author table) and register `comments` relationship in the sheet `.rels`; omit when no comments
- [x] 4.3 Reader: resolve sheet `.rels` → `xl/commentsN.xml`, populate `cell.note` (and author)
- [x] 4.4 napi bridge: expose cell comment access; add round-trip fixture (text + author)

## 5. Images / drawings

- [x] 5.1 Add `Image` model type + `Worksheet.addImage(opts)` / `getImages()` (one-cell & two-cell anchors)
- [x] 5.2 Writer: write bytes to `xl/media/imageM.<ext>`, emit `xl/drawings/drawingN.xml` with anchors, register `drawing` (sheet `.rels`) + `image` (drawing `.rels`) relationships; omit when no images
- [x] 5.3 Reader: resolve sheet `.rels` → drawing part → media rels to `xl/media/`; populate `ws.getImages()` with `{ extension, buffer }` + anchor
- [x] 5.4 napi bridge: expose `addImage`/`getImages`; add round-trip fixture (byte-exact + anchor)

## 6. Release & parity bookkeeping

- [x] 6.1 Bump `package.json` `version` `0.13.0` → `1.0.0`; update `CHANGELOG.md` and `ROADMAP.md` parity matrix (five areas → `shipped`)
- [x] 6.2 Sync the `exceljs-parity` spec delta and archive the change when all features pass their round-trip fixtures
