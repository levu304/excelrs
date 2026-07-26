// enums.d.ts — TypeScript type aliases for enum-like fields in excelrs.
//
// Rust `#[napi(string_enum)]` covers the simple enums (FillKind, BorderStyleStyle,
// AlignmentHorizontal, etc.) in native.d.ts / index.d.ts. This companion file adds
// type aliases for fields that cannot be converted to Rust enums because they are
// flat-tag discriminators (CellValue.valueType) or depend on other fields' values
// (CfRule.operator depends on CfRule.type).

// ---------------------------------------------------------------------------
// CellValue.valueType discriminant
// ---------------------------------------------------------------------------

/** Discriminant values for {@link CellValue.valueType}. */
export type CellValueType =
  | 'Null'
  | 'Number'
  | 'String'
  | 'Boolean'
  | 'Formula'
  | 'Error'
  | 'Hyperlink'
  | 'RichText'
  | 'Merge'
  | 'Date'

// ---------------------------------------------------------------------------
// Conditional-formatting rule types
// ---------------------------------------------------------------------------

/** Conditional-format rule type ({@link CfRule.type}). */
export type CfRuleType =
  | 'cellIs'
  | 'expression'
  | 'colorScale'
  | 'dataBar'
  | 'iconSet'
  | 'top10'
  | 'unique'
  | 'duplicate'
  | 'containsText'
  | 'timePeriod'
  | 'containsBlanks'
  | 'notContainsBlanks'
  | 'containsErrors'
  | 'notContainsErrors'

/** Cell-value operator ({@link CfRule.operator}). */
export type CfRuleOperator =
  | 'lessThan'
  | 'greaterThan'
  | 'equal'
  | 'notEqual'
  | 'greaterThanOrEqual'
  | 'lessThanOrEqual'
  | 'between'
  | 'notBetween'
  | 'containsText'
  | 'beginsWith'
  | 'endsWith'
  | 'notContainsText'

/** Time period value ({@link CfRule.timePeriod}). */
export type CfTimePeriod =
  | 'today'
  | 'yesterday'
  | 'tomorrow'
  | 'last7Days'
  | 'lastWeek'
  | 'thisWeek'
  | 'nextWeek'
  | 'lastMonth'
  | 'thisMonth'
  | 'nextMonth'

/** Conditional-format value object type ({@link Cfvo.type}). */
export type CfvoType =
  | 'num'
  | 'percent'
  | 'percentile'
  | 'formula'
  | 'min'
  | 'max'
  | 'autoMin'
  | 'autoMax'

// ---------------------------------------------------------------------------
// Data validation types
// ---------------------------------------------------------------------------

/** Data validation type ({@link DataValidation.type}). */
export type DataValidationType =
  | 'whole'
  | 'decimal'
  | 'list'
  | 'date'
  | 'time'
  | 'textLength'
  | 'custom'

/** Data validation operator ({@link DataValidation.operator}). */
export type DataValidationOperator =
  | 'between'
  | 'notBetween'
  | 'equal'
  | 'notEqual'
  | 'greaterThan'
  | 'lessThan'
  | 'greaterThanOrEqual'
  | 'lessThanOrEqual'

/** Data validation error style ({@link DataValidation.errorStyle}). */
export type DataValidationErrorStyle = 'information' | 'warning' | 'stop'