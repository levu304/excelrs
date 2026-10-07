# Spec Delta

## ADDED Requirements

### Requirement: Themed and plain twin styles never share a style-table slot

The style-table builder SHALL assign separate sub-table slots to entries that differ in theme linkage, even when they resolve to the same ARGB. A themed entry (theme index plus optional tint) and a plain entry carrying the identical resolved ARGB SHALL NOT merge; each keeps its own emission form.

#### Scenario: Font twins stay separate regardless of order

- **WHEN** a workbook holds a themed font color (`theme="4"` resolving to `"FF4F81BD"`) and a plain font color `"FF4F81BD"`
- **THEN** the built style table contains two font slots, one emitted as `<color theme="4"/>` and the other as `<color rgb="FF4F81BD"/>`, whichever entry is seen first

#### Scenario: Fill twins stay separate

- **WHEN** a themed fill foreground and a plain fill foreground resolve to the same ARGB
- **THEN** they occupy two fill slots and keep their respective `theme=` / `rgb=` emission forms

#### Scenario: Single-kind workbooks are unaffected

- **WHEN** a workbook contains only themed entries (or only plain entries) with distinct values
- **THEN** style-table slot assignment is identical to before this change (no spurious splits)
