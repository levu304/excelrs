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

## API Surface

Workbook → Worksheet → Row → Cell — mirrors exceljs exactly.

- **Workbook:** `constructor()`, `addWorksheet()`, `getWorksheet()`, `.xlsx` I/O handle
- **Worksheet:** `getCell()`, `getRow()`, `addRow()`, `removeRow()`, `rowCount`, `columnCount`
- **Row:** `getCell()`, `values`, `height`, `hidden`
- **Cell:** `value` (Number | String | Boolean | Formula | Null), `address`, `formula`

See [docs/spec.md](docs/spec.md) for the full API specification.

## v0.1 — MVP

Read and write `.xlsx` files with correct data fidelity.

**Limitations (see [spec §9.1](docs/spec.md#91-v01--mvp) for full list):**
- No style CRUD (cell styles are read-only)
- No merged cells
- No streaming read/write
- No formula evaluation (preserved as strings)
- No CSV / XLS / XLSB support

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
