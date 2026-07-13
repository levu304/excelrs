/* Hand-maintained type declarations. Generated types emitted to native.d.ts by pnpm build. */
/**
 * A single cell in a worksheet.
 *
 * Holds `Arc<Mutex<CellInner>>` so that every clone shares the same underlying
 * state — value and style mutations made through any handle persist to the
 * worksheet's internal model.
 */
export declare class Cell {
  constructor(address: string, row: number, col: number)
  get value(): CellValue
  /**
   * Accepts JS primitives via serde_json::Value auto-conversion (napi v3 serde-json feature).
   * Dispatches to the correct CellValue variant based on the JSON value type.
   */
  set value(val: CellValue | number | string | boolean | null)
  get address(): string
  get row(): number
  get col(): number
  get formula(): string | null
  /** Returns the cell's style, or `None` if Normal (index 0). */
  get style(): Style | null
  /**
   * Set the cell's style from a JS object. Full-replace semantics
   * (spec §6.9): assigning a new style replaces the existing one.
   *
   * - `null | undefined | {}` → resets to Normal (None).
   * - Throws `ExcelrsError::InvalidStyle` on validation failure.
   */
  set style(val: Style | null)
}

/**
 * A column definition in a worksheet.
 *
 * Mirrors the exceljs `Column` interface: header label, data-binding key,
 * width in characters, hidden state, and 1-indexed column number.
 *
 * `col_num` is optional in the JS object. If omitted (or 0), it is
 * auto-assigned sequentially in `Worksheet.setColumns` — the first column
 * gets col_num=1, the second gets col_num=2, etc.  For sparse definitions
 * (e.g. defining only column B), pass the `colNum` explicitly.
 */
export declare class Column {
  constructor(header: string, key: string, width: number)
  get header(): string
  set header(val: string)
  get key(): string
  set key(val: string)
  get width(): number
  set width(val: number)
  get hidden(): boolean
  set hidden(val: boolean)
  get style(): Style | null
  set style(val: Style | null)
  get colNum(): number
}

/**
 * A row in a worksheet.
 *
 * Cells are stored in a `HashMap<u32, Cell>` keyed by 1-indexed column number.
 * The row number is 1-indexed. Accessing a cell by column creates an empty cell
 * if one doesn't exist — the returned Cell is a clone (see clone-on-read semantics
 * in `cell.rs`).
 */
export declare class Row {
  constructor(number: number)
  get number(): number
  get height(): number | null
  set height(val: number | undefined | null)
  get hidden(): boolean
  set hidden(val: boolean)
  get style(): Style | null
  set style(val: Style | null)
  /**
   * Get cell by 1-indexed column number. Creates an empty cell if none exists.
   * This is the Rust backing for `Row.getCell(col: number)`.
   */
  getCellByColNum(col: number): Cell
  /**
   * Get cell by column letter. Creates an empty cell if none exists.
   * This is the Rust backing for `Row.getCell(col: string)`.
   */
  getCellByColLetter(colLetter: string): Cell
  /**
   * Get cell by 1-indexed column number (JS glue → getCellByColNum).
   */
  getCell(col: number): Cell
  /**
   * Get cell by column letter (JS glue → getCellByColLetter).
   */
  getCell(col: string): Cell
}

/**
 * Top-level workbook document.
 *
 * Wraps `WorkbookInner` behind `Arc<Mutex<>>` so that the `WorkbookXlsx`
 * handle can mutate the workbook state via a shared reference.
 *
 * # Clone-on-read semantics
 * Like all napi-rs model types, accessed worksheets are cloned across the FFI
 * boundary.  Cloning the `Workbook` itself clones the `Arc` — all clones share
 * the same inner state.
 */
export declare class Workbook {
  constructor()
  /**
   * Add a new worksheet with the given name.
   * Returns the created Worksheet.
   */
  addWorksheet(name: string): Worksheet
  /**
   * Get a worksheet by name (string) or 1-indexed position (number).
   * Returns `None` if not found.
   */
  getWorksheet(nameOrIndex: string | number): Worksheet | null
  get worksheets(): Array<Worksheet>
  get worksheetCount(): number
  /** ISO-8601 timestamp of workbook creation. */
  get created(): string
  /** ISO-8601 timestamp of last modification. */
  get modified(): string
  /**
   * Returns a `WorkbookXlsx` handle for async XLSX I/O.
   *
   * The handle shares the same underlying `Arc<Mutex<WorkbookInner>>`,
   * so reads through `.xlsx.read(buf)` mutate this workbook's state.
   */
  get xlsx(): WorkbookXlsx
  /**
   * Returns a `WorkbookCsv` handle for async CSV I/O.
   *
   * The handle shares the same underlying `Arc<Mutex<WorkbookInner>>`,
   * so reads through `.csv.read(buf)` mutate this workbook's state.
   */
  get csv(): WorkbookCsv

  // -- Defined names (v0.7.0) --

  /** Snapshot of all defined names in the workbook. */
  get definedNames(): Array<DefinedName>
  /**
   * Add or upsert a defined name.
   *
   * Workbook-scope: matched by `name` alone.
   * Sheet-scope: matched by `name` + `sheet`.
   */
  addDefinedName(name: string, value: string, sheet?: string | null): void
  /**
   * Remove a defined name by `name` (and optional `sheet`).
   * No-op if no matching name exists.
   */
  removeDefinedName(name: string, sheet?: string | null): void
  /**
   * Get a defined name by `name` (and optional `sheet`).
   * Returns `null` if not found.
   */
  getDefinedName(name: string, sheet?: string | null): DefinedName | null
}

/**
 * Async XLSX read/write handle.
 *
 * Obtained via `Workbook.xlsx` getter.  Shares the same underlying
 * `Arc<Mutex<WorkbookInner>>` as the parent Workbook.
 */
export declare class WorkbookXlsx {
  /**
   * Read an .xlsx file from a JS `Buffer`.  Async.
   *
   * Parses the buffer with calamine, then replaces the workbook state
   * in-place.  All existing worksheets are discarded.
   */
  read(buffer: Buffer): Promise<void>
  /** Read an .xlsx file from disk.  Async. */
  readFile(path: string): Promise<void>
  /**
   * Write the workbook to an .xlsx buffer.  Async.
   *
   * Clones the workbook state briefly under the lock, then builds the
   * .xlsx archive outside the lock (calamine / zip I/O is expensive).
   */
  write(): Promise<Buffer>
  /** Write the workbook to an .xlsx file on disk.  Async. */
  writeFile(path: string): Promise<void>
}

/**
 * Async CSV read/write handle.
 *
 * Obtained via `Workbook.csv` getter. Shares the same underlying
 * `Arc<Mutex<WorkbookInner>>` as the parent Workbook.
 *
 * RFC 4180 parser & serializer (manual, no extra deps). Numeric inference
 * on read, single-sheet on write, optional delimiter and BOM support.
 */
export declare class WorkbookCsv {
  /**
   * Parse a CSV `Buffer` into a single worksheet ("Sheet1"), replacing
   * the workbook's existing worksheets in place.
   *
   * An optional `delimiter` overrides the field separator (default `,`).
   */
  read(buffer: Buffer, delimiter?: string | undefined | null): Promise<void>
  /**
   * Read a CSV file from disk into a single worksheet ("Sheet1").
   *
   * An optional `delimiter` overrides the field separator (default `,`).
   */
  readFile(path: string, delimiter?: string | undefined | null): Promise<void>
  /**
   * Serialize the first worksheet to a CSV `Buffer`.
   *
   * Optional `delimiter` (default `,`) and `withBom` (default `false`).
   * Only `worksheets[0]` is written (CSV is single-sheet).
   */
  write(delimiter?: string | undefined | null, withBom?: boolean | undefined | null): Promise<Buffer>
  /**
   * Serialize the first worksheet to a CSV file on disk.
   *
   * Optional `delimiter` (default `,`) and `withBom` (default `false`).
   */
  writeFile(path: string, delimiter?: string | undefined | null, withBom?: boolean | undefined | null): Promise<void>
}

/**
 * A single worksheet (sheet) in a workbook.
 *
 * Rows are stored behind `Arc<Mutex<>>` so that any clone of a Worksheet
 * shares the same row state.  This is what makes `wb.addWorksheet() → ws`
 * → `ws.addRow(...)` work across the napi-rs FFI boundary.
 */
export declare class Worksheet {
  constructor(name: string)
  get name(): string
  set name(val: string)
  get id(): number
  /** Number of rows with content (highest row index with data). */
  get rowCount(): number
  /** Number of columns with content (highest column index across all rows). */
  get columnCount(): number
  /**
   * Get cell by A1-style address string (e.g., "A1", "BC42").
   * Returns an empty cell if the address is valid but hasn't been populated.
   */
  getCellByAddress(address: string): Cell
  /**
   * Get cell by 1-indexed row and column numbers.
   * Returns the cell from the worksheet's internal row map, so value and style
   * mutations on the returned cell persist into the worksheet.
   * Creates the row (and cell) if absent.
   */
  getCellByRc(row: number, col: number): Cell
  /**
   * Get cell by A1-style address string (JS glue → getCellByAddress).
   * e.g., "A1", "BC42"
   */
  getCell(address: string): Cell
  /**
   * Get cell by 1-indexed row and column numbers (JS glue → getCellByRc).
   */
  getCell(row: number, col: number): Cell
  /** Get row by 1-indexed row number. Creates the row if it doesn't exist. */
  getRow(rowNumber: number): Row
  /** Add a row of cell values. Returns the created Row. */
  addRow(values: Array<CellValue | number | string | boolean | null>): Row
  /**
   * Get a contiguous range of rows starting at `start` (1-indexed).
   * Returns up to `count` rows.
   */
  getRows(start: number, count: number): Array<Row>
  /** Remove a row by number. No-op if the row doesn't exist. */
  removeRow(rowNumber: number): void
  /** All rows with content, sorted by row number. */
  get rows(): Array<Row>
  get columns(): Array<Column>
  /**
   * Set the style of a cell at (row, col).  Bypasses clone-on-read:
   * the cell is mutated inside the locked row map.
   */
  setCellStyle(row: number, col: number, style: Style | null): void
  /**
   * Replace the worksheet's column definitions.
   *
   * Accepts a JS array of column descriptor objects (header, key, width,
   * optional hidden, optional style). Parsed server-side via serde.
   * Each column's style is validated (matching `Cell.set_style` behavior).
   * Replace the worksheet's column definitions.
   *
   * Accepts a JS array of column descriptor objects (header, key, width,
   * optional `colNum`, optional hidden, optional style).  Parsed
   * server-side via serde.  Each column's style is validated (matching
   * `Cell.set_style` behavior).
   *
   * `colNum` auto-assignment: columns with `colNum == 0` get sequential
   * numbers starting from `max(existing col_nums) + 1` (or 1 if none
   * exist).  Duplicate `colNum` values across the same call are rejected.
   */
  setColumns(cols: Array<ColumnInput>): void
  /**
   * Merge a range of cells (e.g. "A1:C3"). Accepts an A1-style range string.
   * Validates that the range parses to a rectangular area; stores it for
   * emission in the writer. Duplicate ranges are silently ignored.
   */
  mergeCells(range: string): void

  // -- Data validation (v0.8.0) --

  /** All data validations on this worksheet. */
  get dataValidations(): Array<DataValidation>
  /**
   * Add or update a data validation. `dv` must include `sqref`, `type`, and `formula1`.
   * Upserts by `sqref` (duplicate ranges overwrite). Throws on invalid type or empty sqref.
   */
  addDataValidation(dv: DataValidation): void
  /** Get a data validation by sqref range string. Returns `null` if not found. */
  getDataValidation(sqref: string): DataValidation | null
  /** Remove a data validation by sqref range string. No-op if not found. */
  removeDataValidation(sqref: string): void
}

/**
 * Input descriptor for `Worksheet.setColumns()`. Mirrors `Column` but as a plain object.
 * `header`, `key`, `width` are required; `colNum`, `hidden`, `style` are optional.
 */
export interface ColumnInput {
  header: string
  key: string
  width: number
  /** 1-indexed column number. Auto-assigned if omitted or 0. */
  colNum?: number
  hidden?: boolean
  style?: Style | null
}

/** Cell content alignment and text wrapping. */
export interface Alignment {
  /** Horizontal: `"left"` | `"center"` | `"right"` | `"fill"` | `"justify"`. */
  horizontal?: string
  /** Vertical: `"top"` | `"middle"` | `"bottom"`. */
  vertical?: string
  wrapText?: boolean
  indent?: number
}

/** All cell-border sides plus diagonals. Each side is optional; `None` means no border. */
export interface Border {
  top?: BorderStyle
  right?: BorderStyle
  bottom?: BorderStyle
  left?: BorderStyle
  /** Diagonal border line style. Only valid edges between top-left ↔ bottom-right. */
  diagonal?: BorderStyle
  /**
   * Whether the diagonal line goes up (bottom-left to top-right).
   * OOXML attribute `diagonalUp` on the `<border>` element.
   */
  diagonalUp?: boolean
  /**
   * Whether the diagonal line goes down (top-left to bottom-right).
   * OOXML attribute `diagonalDown` on the `<border>` element.
   */
  diagonalDown?: boolean
}

/** Border line style and color for one side of a cell border. */
export interface BorderStyle {
  /**
   * Border style: `"thin"` | `"medium"` | `"thick"` | `"dashed"` |
   * `"dotted"` | `"double"`. `"none"` is rejected; use `None` for
   * the border side (e.g. `Border.top = None`) to express no border.
   */
  style: string
  /** Line color (ARGB hex). Default: black (`"FF000000"` in exceljs). */
  color?: string
}

/**
 * A defined (named) range in the workbook.
 *
 * - `name`: the name (case-sensitive per OOXML spec §18.2.7).
 * - `value`: the raw text (no formula evaluation).
 * - `sheet`: sheet scoping. `undefined` = workbook-global.
 */
export interface DefinedName {
  name: string
  value: string
  /** Sheet scope: sheet name string, or `undefined` for workbook-global. */
  sheet?: string
}

export interface CellValue {
  /**
   * Discriminant: "Null" | "Number" | "String" | "Boolean" | "Formula" | "Error"
   * | "Hyperlink" | "RichText" | "Merge"
   */
  valueType: string
  number?: number
  string?: string
  boolean?: boolean
  formula?: string
  errorValue?: string
  /** URL for hyperlink (write-only, Null on read). */
  hyperlink?: string
  /** Display text for hyperlink (write-only, Null on read). */
  hyperlinkText?: string
  /** Rich text runs (write-only, Null on read). */
  richText?: Array<RichTextRun>
}

/** Cell fill: kind, foreground, background, and pattern. */
export interface Fill {
  /** Fill kind: `"none"` | `"solid"` | `"pattern"` | `"gradient"`. */
  kind: string
  /** Foreground color (ARGB hex). Default: None. */
  foreground?: string
  /** Background color (ARGB hex). Default: None. */
  background?: string
  /** Pattern name (for `kind="pattern"`). Default: None. */
  pattern?: string
  /** Gradient type: `"linear"` or `"path"`. Only used when `kind="gradient"`. */
  gradientType?: string
  /** Gradient angle in degrees (linear). Only used when `kind="gradient"`. */
  gradientDegree?: number
  /**
   * Deprecated — previously emitted an invalid `angle` attribute.
   * Use `gradientDegree` (linear) or `gradientLeft`/`gradientRight`/`gradientTop`/`gradientBottom` (path).
   */
  gradientAngle?: number
  /** Left edge position (0.0–1.0) for path gradients. Only used when `kind="gradient"` and `gradientType="path"`. */
  gradientLeft?: number
  /** Right edge position (0.0–1.0) for path gradients. Only used when `kind="gradient"` and `gradientType="path"`. */
  gradientRight?: number
  /** Top edge position (0.0–1.0) for path gradients. Only used when `kind="gradient"` and `gradientType="path"`. */
  gradientTop?: number
  /** Bottom edge position (0.0–1.0) for path gradients. Only used when `kind="gradient"` and `gradientType="path"`. */
  gradientBottom?: number
  /** Gradient stops. Only used when `kind="gradient"`. */
  gradientStops?: Array<GradientStop>
}

/** Font properties: name, size (points), weight, style, and color. */
export interface Font {
  /** Font name (e.g. "Calibri", "Arial"). Default: "Calibri". */
  name?: string
  /** Font size in points. Default: 11. Must be finite. */
  size?: number
  bold?: boolean
  italic?: boolean
  underline?: boolean
  /** ARGB hex (8 chars) or RGB hex (6 chars). Default: None. */
  color?: string
}

/** A single gradient stop: color + position. */
export interface GradientStop {
  /** ARGB hex color (8 chars) or RGB hex (6 chars). */
  color: string
  /** Position in [0.0, 1.0]. */
  position: number
}

/**
 * Flat tagged union for cell values across the FFI boundary.
 *
 * Discriminant is `value_type`:
 * - `"Null"` — no value (default)
 * - `"Number"` — numeric value (field: `number`)
 * - `"String"` — text value (field: `string`)
 * - `"Boolean"` — boolean value (field: `boolean`)
 * - `"Formula"` — formula string (field: `formula`; preserved, not evaluated)
 * - `"Error"` — error value (field: `error_value`)
 *
 * # Write-only variants (v0.5.0)
 * `Hyperlink`, `RichText`, `Merge` are write-only: they can be set via JS and
 * will be written to the XLSX, but calamine does not expose them on the read
 * path so they appear as `Null` when read back (see spec §9.2.1 item 2).
 * A rich text run: a text fragment with optional font formatting.
 */
export interface RichTextRun {
  /** Text content for this run. */
  text: string
  /** Font formatting for this run (optional). */
  font?: Font
}

/**
 * A data validation constraint on a cell range.
 *
 * - `sqref`: cell range reference (e.g. "A1:A10"). Required.
 * - `type`: validation type: "whole" | "decimal" | "list" | "date" | "time" | "textLength" | "custom". Required.
 * - `operator`: comparison operator. Required for whole/decimal/date/time/textLength.
 * - `formula1`: first formula value. Required.
 * - `formula2`: second formula value (for between/notBetween).
 * - `allowBlank`: whether blank cells are allowed.
 * - `showInputMessage`, `showErrorMessage`: display flags.
 * - `prompt`, `promptTitle`: input prompt text/title.
 * - `error`, `errorTitle`, `errorStyle`: error alert text/title/style ("stop", "warning", "information").
 */
export interface DataValidation {
  sqref: string
  type: string
  operator?: string
  formula1: string
  formula2?: string
  allowBlank?: boolean
  showInputMessage?: boolean
  showErrorMessage?: boolean
  prompt?: string
  promptTitle?: string
  error?: string
  errorTitle?: string
  errorStyle?: string
}

/**
 * Aggregate style object. Each sub-field is optional; `None` means
 * that aspect of formatting is left at the built-in "Normal" default.
 *
 * **Semantics: full-replace.** Assigning a new `Style` replaces the
 * existing style entirely. Use the spread idiom (spec §6.9) to preserve
 * specific fields.
 */
export interface Style {
  font?: Font
  fill?: Fill
  border?: Border
  alignment?: Alignment
  /**
   * Format code string, e.g. `"0.00%"`, `"$#,##0.00"`, `"yyyy-mm-dd"`.
   * `None` means no format (Normal). `Some("")` is rejected.
   */
  numFmt?: string
}
