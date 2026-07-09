/* excelrs — Thin JS entry point */
/* eslint-disable */
// @ts-nocheck

// Load the auto-generated native binding (built by napi to native.js)
const nativeBinding = require('./native.js')

// ---------------------------------------------------------------------------
// JS glue: method overloading for exceljs API compatibility
// napi-rs v3 doesn't support method overloading. These monkey-patches dispatch
// between separate Rust methods based on argument types.
// ---------------------------------------------------------------------------

const { Worksheet, Row } = nativeBinding

/**
 * Worksheet.getCell overloading:
 *   getCell(address: string)         → getCellByAddress
 *   getCell(row: number, col: number) → getCellByRc
 */
Worksheet.prototype.getCell = function (a, b) {
  return typeof a === 'string'
    ? this.getCellByAddress(a)
    : this.getCellByRc(a, b)
}

/**
 * Row.getCell overloading:
 *   getCell(col: number) → getCellByColNum
 *   getCell(col: string) → getCellByColLetter
 */
Row.prototype.getCell = function (col) {
  return typeof col === 'number'
    ? this.getCellByColNum(col)
    : this.getCellByColLetter(col)
}

// Export everything from the native binding
module.exports = nativeBinding
