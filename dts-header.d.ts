type CellSimpleValue = number | string | boolean | null
type CellValueResult = CellSimpleValue | Date | CellValue

export interface Cell {
  /** Accepts primitives, CellValue-like objects, or Date — dispatch by shape. */
  set value(val: CellValue | Partial<CellValue> | string | number | boolean | Date | null)
  /**
   * @deprecated Use `cell.value` instead (returns Date for Date cells).
   * Will be removed in v3.
   */
  get date(): Date | null
}