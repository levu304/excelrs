# Spec Delta

## MODIFIED Requirements

### Requirement: excelrs declares the ExcelJS-4.4.0 v1.x parity program complete

The v1.x drop-in ExcelJS-4.4.0 parity program is **complete** as of release v2.0.0:
every feature area in the v1.x targeted roadmap (including `streaming XLSX`, and all
v0.x-v1.x areas) was marked `shipped` (or `partial` where explicitly noted), and the
ROADMAP records the program as complete. This is a historical record, not a standing
constraint: an area's status SHALL be whatever the parity matrix currently says, and a
later release that ships a previously-excluded area SHALL advance that area's status
without needing to amend this requirement. The areas excluded from the completed program
at v2.0.0 were: charts, pivot tables, and formula evaluation (distant / deferred), plus
themes-write, sheet state (visible/hidden), tab color, and default worksheet properties
(deferred to post-v2.0.0 triage).

#### Scenario: Streaming closes the final matrix area

- **WHEN** release v2.0.0 was cut
- **THEN** the parity matrix marked `workbook IO (streams)` as `shipped`, leaving no targeted v1.x area unshipped

#### Scenario: Program declared complete with documented exclusions

- **WHEN** the v2.0.0 release was recorded
- **THEN** the ROADMAP recorded the v1.x drop-in ExcelJS-4.4.0 parity program as complete, and charts, pivot tables, formula evaluation, themes-write, sheet state, tab color, and default properties were listed as out of scope (`planned` / `n-a`)

#### Scenario: A later release ships a previously-excluded area

- **WHEN** a release ships an area that was excluded from the v1.x program, such as sheet state or tab color
- **THEN** the parity matrix status for that area SHALL advance to `shipped`, and this requirement SHALL continue to hold as the v2.0.0 historical record without being modified

## ADDED Requirements

### Requirement: Parity matrix rows reflect shipped behavior

The ROADMAP parity matrix SHALL record, for each feature area, the status that the
implemented behavior actually has. An area whose behavior is implemented and round-trip
verified SHALL NOT be listed as `planned` or as "Not implemented". When a release ships an
area, the matrix row for that area SHALL be updated in the same change that ships it.

#### Scenario: Shipped area is not marked not-implemented

- **WHEN** an area's read and write paths are implemented and round-trip verified
- **THEN** the parity matrix SHALL mark that area `shipped` and SHALL NOT describe it as "Not implemented"

#### Scenario: Shipping a change updates its matrix row

- **WHEN** a change ships behavior for a feature area listed in the parity matrix
- **THEN** that change SHALL update the corresponding matrix row to the new status
