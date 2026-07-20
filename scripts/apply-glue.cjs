#!/usr/bin/env node
// scripts/apply-glue.cjs
// Post-build hook for napi --pipe: re-injects ExcelJS-compat getCell overloads
// into the generated index.js / index.d.ts.
//
// Called by:  napi build --pipe "node scripts/apply-glue.cjs"
// Mechanism: napi passes the output file path as process.argv[2].

const fs = require('fs')
const path = require('path')

const JS_MARKER = '__EXCELJS_GETCELL_GLUE__'

const JS_GLUE = `
// __EXCELJS_GETCELL_GLUE__
// JS glue: ExcelJS-compat getCell overloads (delegate to Rust getCellBy* APIs)
nativeBinding.Worksheet.prototype.getCell = function (a, b) {
  return b === undefined ? this.getCellByAddress(a) : this.getCellByRc(a, b)
}
nativeBinding.Row.prototype.getCell = function (col) {
  return typeof col === 'number' ? this.getCellByColNum(col) : this.getCellByColLetter(col)
}
`

const DTS_MARKER = '__EXCELJS_GETCELL_GLUE__'

const DTS_GLUE = `
// __EXCELJS_GETCELL_GLUE__
// ExcelJS-compat getCell overloads (TypeScript declaration merging)
export interface Worksheet {
  /** Get cell by A1-style address string (JS glue → getCellByAddress). */
  getCell(address: string): Cell
  /** Get cell by 1-indexed row and column numbers (JS glue → getCellByRc). */
  getCell(row: number, col: number): Cell
}
export interface Row {
  /** Get cell by 1-indexed column number (JS glue → getCellByColNum). */
  getCell(col: number): Cell
  /** Get cell by column letter (JS glue → getCellByColLetter). */
  getCell(col: string): Cell
}
`

function processFile(filePath) {
  const basename = path.basename(filePath)

  // Only act on index.js and index.d.ts
  if (basename === 'index.js') {
    let content = fs.readFileSync(filePath, 'utf8')
    if (content.includes(JS_MARKER)) return false
    content = content.trimEnd() + '\n' + JS_GLUE + '\n'
    fs.writeFileSync(filePath, content)
    console.log(`[apply-glue] Patched ${filePath} (added Worksheet/Row getCell)`)
    return true
  }

  if (basename === 'index.d.ts') {
    let content = fs.readFileSync(filePath, 'utf8')
    if (content.includes(DTS_MARKER)) return false
    content = content.trimEnd() + '\n' + DTS_GLUE + '\n'
    fs.writeFileSync(filePath, content)
    console.log(`[apply-glue] Patched ${filePath} (added getCell type overloads)`)
    return true
  }

  return false
}

// napi --pipe passes the output file path as process.argv[2]
const filePath = process.argv[2]
if (filePath) {
  processFile(filePath)
} else {
  // Fallback: patch index.js and index.d.ts in cwd
  for (const f of ['index.js', 'index.d.ts']) {
    if (fs.existsSync(f)) processFile(f)
  }
}