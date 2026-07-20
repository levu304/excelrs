## ADDED Requirements

### Requirement: Streaming reader shifts bare column and row references in shared formulas

The streaming reader SHALL, when resolving a shared-formula *member* cell, shift
bare column references (e.g. `A`) and bare row references (e.g. `5`) in the master
formula text by the member's offset, so that the resolved text matches what the
whole-workbook (calamine) reader produces. This extends the existing shared-formula
member resolution beyond `Cell` references (`A1`) and `Cell` ranges (`A1:A3`). The
streaming reader SHALL NOT shift tokens that are not valid references (function
names such as `COLUMN`, `SUM`, and quoted strings), preserving them verbatim.

#### Scenario: Bare column reference shifts by the member offset

- **WHEN** a shared-formula master text contains a bare column reference such as `A` (e.g. `=A+B`) and the member cell is shifted one column to the right
- **THEN** the streaming reader resolves the member to `=B+C`, matching the whole-workbook reader, not the unshifted `=A+B`

#### Scenario: Bare row reference shifts by the member offset

- **WHEN** a shared-formula master text contains a bare row reference such as `5` (e.g. `=A1*5`) and the member cell is shifted one row down
- **THEN** the streaming reader resolves the member to `=A2*6`, matching the whole-workbook reader, not the unshifted `=A1*5`

#### Scenario: Function names and quoted strings stay verbatim

- **WHEN** a shared-formula master text contains a function-name token (e.g. `COLUMN`, `SUM`) or a quoted string (e.g. `"A1"`)
- **THEN** the streaming reader copies those tokens verbatim and does not attempt to shift them, identical to the whole-workbook reader
