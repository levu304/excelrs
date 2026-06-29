/**
 * Thin JS glue layer for napi-rs impedance mismatches.
 *
 * napi-rs v3 does not support:
 * 1. Method overloading — `getCell(address: string)` vs `getCell(row: number, col: number)`
 * 2. Union parameter types — `Row.getCell(col: number | string)`
 *
 * This file provides the overload dispatch by monkey-patching the native prototypes.
 * It is loaded by `index.js` and contains no business logic — only mechanical type dispatch.
 */

/* eslint-disable @typescript-eslint/no-unsafe-call, @typescript-eslint/no-unsafe-member-access, @typescript-eslint/no-explicit-any */

import { Worksheet, Row } from './index';

/**
 * Method overloading: getCell(address: string) vs getCell(row: number, col: number)
 *
 * In napi-rs, this is two separate Rust methods:
 *   - getCellByAddress(address: string)
 *   - getCellByRC(row: u32, col: u32)
 *
 * The JS glue dispatches based on argument types.
 */
(Worksheet.prototype as any).getCell = function (this: any, a: string | number, b?: number) {
  return typeof a === 'string'
    ? this.getCellByAddress(a)
    : this.getCellByRc(a, b!);
};

/**
 * Row.getCell overloading: getCell(col: number) vs getCell(col: string)
 *
 * In napi-rs, this is:
 *   - getCellByColNum(col: u32)
 *   - getCellByColLetter(colLetter: string)
 */
(Row.prototype as any).getCell = function (this: any, col: number | string) {
  return typeof col === 'number'
    ? this.getCellByColNum(col)
    : this.getCellByColLetter(col);
};
