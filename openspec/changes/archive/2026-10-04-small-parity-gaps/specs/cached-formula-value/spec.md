# Spec Delta

## ADDED Requirements

### Requirement: Legacy array formulas degrade to plain formulas preserving text and cached value

A cell carrying `<f t="array" ref="...">` SHALL read as a `Formula` cell whose formula text and cached `<v>` scalar are preserved, and SHALL write back as a plain `<f>` with its cached `<v>`. The array type and ref range are not preserved. Recalculation-dependent array (spill) semantics stay out of scope.

#### Scenario: Array cell reads as a plain formula with its cached value

- **WHEN** a workbook whose `C1:C3` each carry `<f t="array" ref="C1:C3">SUM(A1:A3*1)</f><v>6</v>` is read
- **THEN** each of `C1`, `C2`, `C3` SHALL report `formula` `"SUM(A1:A3*1)"` and `value` `6`

#### Scenario: Array cell writes back as a plain formula

- **WHEN** such a workbook is written back to XLSX
- **THEN** each cell SHALL emit `<f>SUM(A1:A3*1)</f><v>6</v>` with no `t` or `ref` attribute on the `<f>`

#### Scenario: Round-tripped array file opens without repair

- **WHEN** the written workbook is opened in Excel or read by ExcelJS
- **THEN** it SHALL open without a repair prompt, presenting three independent plain formulas
