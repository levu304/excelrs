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
- **Worksheet:** `getCell()`, `getRow()`, `addRow()`, `removeRow()`, `setColumns()` (use this to set columns),
  `setCellStyle()`, `headerFooter`, `pageSetup`, `addImage()`, `rowCount`, `columnCount`, `columns` (getter-only), `rows`
- **Row:** `getCell()`, `values`, `height`, `hidden`
- **Cell:** `value` (Number | String | Boolean | Formula | Null), `address`, `formula`,
  `style` (getter/setter, full-replace), `note` / `comment`
- **Column:** `header`, `key`, `width`, `hidden`, `style` (getter/setter, column default)

See [docs/spec.md](docs/spec.md) for the full API specification.

> **ExcelJS compat note — Images:** ExcelJS places `addImage` on the
> `Workbook` (two-step: `workbook.addImage(buffer) → imageId` then
> `worksheet.addImage(imageId, range)`). excelrs places it on `Worksheet`
> as a single call.
>
> The anchor shape also differs: ExcelJS uses `{ tl: {col,row}, br: {col,row} }`
> while excelrs uses the same ExcelJS shape via `ImageAnchorInput` (with
> `tl`, `br` or `ext`). The anchor type (one-cell vs two-cell) is inferred
> from whether `br` (two-cell) or `ext` (one-cell with explicit size) is
> provided — no `anchorType` field is needed. `col`/`row` support fractional
> values (e.g. `5.5`) for sub-cell positioning.
>
> ```ts
> const ws = wb.addWorksheet('Sheet1');
> // Two-cell anchor (fractional col/row allowed)
> ws.addImage({ extension: 'png', buffer,
>   anchor: { tl: { col: 0, row: 0 }, br: { col: 5.5, row: 2.2 } } });
> // One-cell anchor with explicit size
> ws.addImage({ extension: 'png', buffer,
>   anchor: { tl: { col: 1, row: 1 }, ext: { width: 120, height: 60 } } });
> ```
>
> There is no `Workbook.addImage`, `Workbook.getImage`,
> `Worksheet.addBackgroundImage`, or `Worksheet.getBackgroundImageId` —
> the ExcelJS global image registry is not replicated.

> **ExcelJS compat note — Column setter:** In ExcelJS `worksheet.columns` is
> a read-write property (getter + setter). In excelrs it is **getter-only**
> — direct assignment (`worksheet.columns = [...]`) is not supported.
> Call `worksheet.setColumns([...])` instead. This matches the method-call
> pattern used elsewhere (`setCellStyle`, `setColumns`). The getter
> `worksheet.columns` returns the current column definitions.

## v0.2.0 — Style System (write only)

Read and write `.xlsx` files with correct data fidelity. Cell and column
styling for Font, Fill, Border, Alignment, and number formats (write only).

**Limitations (see [spec §9.2.1](docs/spec.md#921-v030-candidate) for full deferred list):**

- Style **read** round-trip shipped in v0.3.0 (styled `.xlsx` preserves styles).
- Cell-level interior mutability shipped in v0.4.0 — `ws.getCell('A1').style = {...}` and `ws.getCell('A1').value = x` now persist into the worksheet automatically (via `Arc<Mutex<CellInner>>`)
- Alignment emission shipped in v0.3.0 (accepted in `Style` JS object, emitted on write).
- CSV via `wb.csv` — single-sheet only on write (CSV cannot represent multiple worksheets); numbers are inferred on read, all other CSV values are strings; no formula evaluation (cached value is emitted when available)
- **Formula evaluation** available via the `formula-eval` Cargo feature (built into release binaries since v2.7.0). Provides `FormulaEvaluator` with 20 built-in functions (SUM, AVERAGE, MIN, MAX, etc.), Excel-spec error propagation, and `Worksheet::recalculate()` (Rust-only for now — JS exposure deferred). `Cell.cachedValue` JS getter returns cached computed values from formula cells.
- **No XLS / XLSB** support (merged cells, data validation, freeze panes, CSV, headers/footers, page setup, comments, images: shipped).
- Theme color references are **preserved on write** (v0.13.0): `<color theme="N"/>` (+`tint`) is emitted instead of a flattened ARGB; the public `color` value remains the resolved ARGB string
- Date cell values are **preserved as JS `Date`** (v0.13.0): `Cell.value` returns `Date | CellValue` from Date cells; the setter accepts a JS `Date`, storing it as the Excel serial number and injecting an appropriate date `numFmt` (if none is set) so the value survives read→write round-trip as a true Date

## Streaming XLSX (v2.1.0+)

Streaming XLSX for large `.xlsx` files.

- **Read** is constant-memory: `StreamReader` materializes one sheet at a time, so peak memory stays bounded by a single sheet regardless of workbook size.
- **Write** buffers every sheet in memory and builds the full archive at `finalize()` — it is **not** constant-memory. Use it when the whole output fits in RAM; for very large outputs prefer writing to disk or generating in parts.

```ts
import { StreamReader, StreamWriter } from '@levu304/excelrs'

// Read: yields sheets one at a time via for-await-of
const reader = new StreamReader(buffer)
for await (const sheet of reader) {
  console.log(sheet.name, sheet.rows.length)
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

## Bridging Features

This repo includes two streaming XLSX capabilities:

### streaming-node-bridge (archived)

Previously delivered: async iterable `wb.stream.xlsx.read()` and `write(sheets)` that preserve values only (no styles). Original constant-memory intent for Node.

### streaming-safety (active)

Newly delivered via `streaming-hardening`: zip‑bomb rejection, stream termination, and per‑sheet constant‑memory reading. Read path is now constant‑memory; write path remains buffered.

Both are additive; the legacy v2.0.0 streaming APIs remain available.
