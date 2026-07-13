## Context

excelrs reads/writes XLSX via the `Workbook.xlsx` async handle (calamine for
cells + zip for per-sheet XML), around a `Workbook` →
`Arc<Mutex<WorkbookInner>>` → worksheets → rows → cells-with-typed-`CellValue`
model. CSV is a single-sheet, untyped text format: it cannot represent
multiple sheets, styles, formulas, or cell types. The roadmap lists CSV
read/write as the next capability.

## Goals / Non-Goals

**Goals:**

- `WorkbookCsv` handle via `wb.csv` getter, sharing inner state (mirror `wb.xlsx`)
- `read`/`readFile`: RFC 4180 parse → one Worksheet ("Sheet1"); numeric
  inference; optional `delimiter`
- `write`/`writeFile`: serialize first worksheet to RFC 4180 CSV; formula →
  cached value; optional `delimiter` + BOM
- exceljs-compatible surface (`Workbook.csv.readFile`/`writeFile`)

**Non-Goals:**

- No multi-sheet CSV (only the first worksheet is written)
- No type preservation on write (CSV is text; numbers/dates/booleans emitted
  as their text form)
- No formula evaluation
- No streaming for very large CSV (in-memory; acceptable at this scale)
- No header-row `key` mapping (every row is data; out of scope)

## Decisions

1. **Mirror `WorkbookXlsx` handle shape** — `wb.csv` getter returns a
   `WorkbookCsv` sharing the same `Arc<Mutex<WorkbookInner>>`. Keeps the
   codebase's single established FFI pattern.

2. **Single worksheet** — CSV has no sheet concept. `write`/`writeFile`
   operate on `worksheets()[0]`. Empty workbook → empty file. `read`
   replaces any existing worksheets with the one parsed sheet.

3. **RFC 4180 manual parse/serialize, no new dependency** — quoted fields,
   embedded commas/newlines, and `""` escapes are ~30 lines. Avoids a crate
   for a well-bounded problem.
   (`# ponytail: manual RFC4180 parser; swap to the`csv`crate only if
   quoting edge cases multiply.`)

4. **Numeric inference on read** — a field that parses as a finite `f64`
   becomes a `Number` cell; everything else is `String`. Booleans and dates
   stay `String` (CSV carries no such types; no evaluation). Matches
   exceljs's loose numeric coercion and keeps round-trips predictable.

5. **Formula cells emit cached value on write** — `CellValue::Formula {
   formula, value }` writes `value` when present, else the raw `formula`
   string. No evaluation; consistent with the existing "formulae stored
   verbatim" non-goal.

6. **`delimiter` + `withBom` options** — `write`/`writeFile` accept
   `{ delimiter?: string, withBom?: boolean }` (default `,`, no BOM); `read`
   accepts `{ delimiter?: string }`. Covers TSV / BOM-consuming consumers
   without scope creep.

7. **Encoding** — UTF-8 only (matches the rest of the library; no codepage
   handling).

## Risks / Trade-offs

- **Single-sheet write silently drops extra worksheets** — acceptable; CSV
  cannot represent them. Document in changelog limitations.
- **Numeric inference is lossy for code-like values** (e.g. `"00123"` →
  `123`). Mitigation: infer by default and document; add an `inferTypes`
  opt-out only if a real case appears.
- **exceljs cross-check** — exceljs's csv writer quotes/inferes
  differently; the round-trip test checks equivalence at the cell-value
  level, not byte-identical output.
