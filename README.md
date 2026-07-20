# excelrs

Native XLSX spreadsheet library for Node.js — a Rust port of
[exceljs](https://github.com/exceljs/exceljs) via
[napi-rs](https://napi.rs).

**10–100× faster** than exceljs for read/write, with a **drop-in compatible API**.

## Install

```bash
npm install @levu304/excelrs
```

## Quick Start

```typescript
import { Workbook } from '@levu304/excelrs';

// Read
const wb = new Workbook();
await wb.xlsx.readFile('input.xlsx');
const ws = wb.getWorksheet('Sheet1');
console.log(ws.getCell('B2').value);

// Write
const wb2 = new Workbook();
const ws2 = wb2.addWorksheet('Data');
ws2.addRow(['Name', 'Age', 'Active']);
ws2.addRow(['Alice', 30, true]);
const buf = await wb2.xlsx.write();
require('fs').writeFileSync('output.xlsx', buf);
```

> **Async contract:** `wb.xlsx.read(buffer)` / `wb.xlsx.readFile(path)` and
> `wb.xlsx.write()` / `wb.xlsx.writeFile(path)` are async — the workbook
> state is only swapped once the returned Promise resolves. Accessing
> worksheets before awaiting the Promise will see stale state.

## v1.0.0 — Drop-in ExcelJS compatibility milestone

v1.0.0 release closes remaining medium-effort ExcelJS parity gaps. All five areas below are read/write round-trippable verified against ExcelJS 4.4.0:

- **Headers & footers** — `ws.headerFooter` read/write (`<headerFooter>` `&C`/`&L`/`&R` format codes).
- **Page setup / print** — `ws.pageSetup` read/write (`pageMargins`, `paperSize`, `orientation`, `printArea`, `printTitles` via defined names).
- **Workbook views & calc properties** — `workbook.views` / `workbook.calcProperties` (`<bookViews>`, `<calcPr>`).
- **Comments** — `Cell.note` / `Cell.comment` read/write (`xl/commentsN.xml` + relationship, authors list).
- **Images / drawings** — `ws.addImage` read/write (`xl/drawings/`, `xl/media/`, anchors, relationship resolution).

See `ROADMAP.md` for full parity matrix and `docs/spec.md` for complete API specification.

### Feature parity snapshot

| Area | Status |
| --- | --- |
| XLSX read / write | shipped (v0.1.0) |
| CSV read / write | shipped (v0.9.0) |
| Styles (font / fill / border / alignment / numFmt) | shipped (v0.2.0+) |
| Merged cells, data validation, hyperlinks, freeze panes, sheet protection, auto filter | shipped (v0.5.0 / v0.8.0 / v0.11.0) |
| Theme / indexed color refs, JS Date bridge | shipped (v0.6.0 / v0.13.0) |
| Headers & footers, page setup, workbook views & calc, comments, images | shipped (v1.0.0) |
| Formula evaluation, tables, charts, conditional formatting, pivots | planned (post-v1) |

## Style System (v0.2.0)

Write-only support for cell and column styling. Font, Fill, Border, and
Alignment properties with inline number formats — full-replace semantics.

```typescript
const wb = new Workbook();
const ws = wb.addWorksheet('Sales');

// Column-level default style
ws.setColumns([
  { header: 'Name', key: 'name', width: 20, style: { font: { bold: true } } },
  { header: 'Amount', key: 'amount', width: 12 },
]);

ws.addRow(['Widget', 1250]);
ws.addRow(['Gadget', 990]);

// Cell-level override (full-replace — see spec §6.9)
ws.setCellStyle(2, 2, {
  font: { color: 'FF00FF00', bold: true },
  fill: { kind: 'solid', foreground: 'FFFFFF00' },
  numFmt: '"$"#,##0.00',
});

const buf = await wb.xlsx.write();
```

## API Surface

Workbook → Worksheet → Row → Cell — mirrors exceljs exactly.

- **Workbook:** `constructor()`, `addWorksheet()`, `getWorksheet()`, `views`, `calcProperties`, `.xlsx` I/O handle
- **Worksheet:** `getCell()`, `getRow()`, `addRow()`, `removeRow()`, `setColumns()`,
  `setCellStyle()`, `headerFooter`, `pageSetup`, `addImage()`, `rowCount`, `columnCount`, `columns`, `rows`
- **Row:** `getCell()`, `values`, `height`, `hidden`
- **Cell:** `value` (Number | String | Boolean | Formula | Null), `address`, `formula`,
  `style` (getter/setter, full-replace), `note` / `comment`
- **Column:** `header`, `key`, `width`, `hidden`, `style` (getter/setter, column default)

See [docs/spec.md](docs/spec.md) for the full API specification.

## v0.2.0 — Style System (write only)

Read and write `.xlsx` files with correct data fidelity. Cell and column
styling for Font, Fill, Border, Alignment, and number formats (write only).

**Limitations (see [spec §9.2.1](docs/spec.md#921-v030-candidate) for full deferred list):**

- Style **read** round-trip shipped in v0.3.0 (styled `.xlsx` preserves styles).
- Cell-level interior mutability shipped in v0.4.0 — `ws.getCell('A1').style = {...}` and `ws.getCell('A1').value = x` now persist into the worksheet automatically (via `Arc<Mutex<CellInner>>`)
- Alignment emission shipped in v0.3.0 (accepted in `Style` JS object, emitted on write).
- CSV via `wb.csv` — single-sheet only on write (CSV cannot represent multiple worksheets); numbers are inferred on read, all other CSV values are strings; no formula evaluation (cached value is emitted when available)
- No formula evaluation, no XLS / XLSB (merged cells, data validation, freeze panes, CSV, headers/footers, page setup, comments, images: shipped).
- Theme color references are **preserved on write** (v0.13.0): `<color theme="N"/>` (+`tint`) is emitted instead of a flattened ARGB; the public `color` value remains the resolved ARGB string
- Date cell values are **preserved as JS `Date`** (v0.13.0): `Cell.value` returns `Date | CellValue` from Date cells; the setter accepts a JS `Date`, storing it as the Excel serial number and injecting an appropriate date `numFmt` (if none is set) so the value survives read→write round-trip as a true Date

## Streaming XLSX (v2.1.0+)

Constant-memory streaming read/write for large `.xlsx` files. Only one sheet is materialized at a time — the entire workbook is never held in memory.

```ts
import { StreamReader, StreamWriter } from '@levu304/excelrs'

// Read: yields sheets one at a time via for-await-of
const reader = new StreamReader(buffer)
for await (const sheet of reader) {
  console.log(sheet.name, sheet.rowCount)
  // Each sheet's rows are yielded here — only one in memory at a time
}

// Write: accepts sheets incrementally
const writer = new StreamWriter()
writer.writeSheet(sheet1)
writer.writeSheet(sheet2)
const output = writer.finalize() // Buffer
```

**Hand-written bridge functions** (Node `Readable` / `Writable` / `AsyncIterable` adapters):

```ts
import { read, write, readAsReadable, writeToWritable } from '@levu304/excelrs/stream-bridge'

// AsyncIterable
for await (const sheet of read(buffer)) { ... }

// Node Readable
Readable.from(read(buffer))

// Node Writable
await writeToWritable(read(buffer), writable)
```

## Development

```bash
pnpm build              # Build Rust → native addon
cargo test              # Rust unit tests
pnpm test               # JS integration tests
cargo clippy -- -D warnings
cargo fmt -- --check
```

## License

Dual-licensed under MIT or Apache-2.0 — see [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE).
