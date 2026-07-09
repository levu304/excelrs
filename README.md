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

- **Workbook:** `constructor()`, `addWorksheet()`, `getWorksheet()`, `.xlsx` I/O handle
- **Worksheet:** `getCell()`, `getRow()`, `addRow()`, `removeRow()`, `setColumns()`,
  `setCellStyle()`, `rowCount`, `columnCount`, `columns`, `rows`
- **Row:** `getCell()`, `values`, `height`, `hidden`
- **Cell:** `value` (Number | String | Boolean | Formula | Null), `address`, `formula`,
  `style` (getter/setter, full-replace)
- **Column:** `header`, `key`, `width`, `hidden`, `style` (getter/setter, column default)

See [docs/spec.md](docs/spec.md) for the full API specification.

## v0.2.0 — Style System (write only)

Read and write `.xlsx` files with correct data fidelity. Cell and column
styling for Font, Fill, Border, Alignment, and number formats (write only).

**Limitations (see [spec §9.2.1](docs/spec.md#921-v030-candidate) for full deferred list):**
- No style **read** — round-trip of a styled `.xlsx` drops styles (deferred to v0.3.0)
- Cell-level interior mutability shipped in v0.4.0 — `ws.getCell('A1').style = {...}` and `ws.getCell('A1').value = x` now persist into the worksheet automatically (via `Arc<Mutex<CellInner>>`)
- No `alignment` emission — accepted in the `Style` JS object but silently dropped
  at write time (deferred to v0.3.0)
- No merged cells, no streaming, no formula evaluation, no CSV / XLS / XLSB

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
