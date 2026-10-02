# design-record Specification

## Purpose

Keeps the rationale behind excelrs design decisions durably reachable, so a reader can
establish why the system is built the way it is, and whether that reasoning still holds,
without reconstructing it from git history.

## Requirements

### Requirement: Every design decision is a file with an explicit standing

Each recorded design decision SHALL exist as its own file under `docs/adr/`, named by its
number, and SHALL state its status as one of `current` or `superseded`, together with the
decision, the context that produced it, and the alternatives that were rejected.

#### Scenario: A decision's standing is readable without archaeology

- **WHEN** a reader opens any file under `docs/adr/`
- **THEN** the file SHALL state the decision, its status, why it was chosen, and which
  alternatives were rejected and on what grounds

#### Scenario: A decision that no longer holds is marked, not deleted

- **WHEN** a decision is replaced by a later decision
- **THEN** the earlier file SHALL remain present with status `superseded` and SHALL name the
  decision that superseded it

### Requirement: The decision record has an index

`docs/adr/` SHALL contain an index listing every decision by number, title, and status, so
the set of decisions is discoverable without opening each file.

#### Scenario: Standing of any decision is resolvable from the index

- **WHEN** a reader consults the index to learn whether a given decision still governs
- **THEN** the index SHALL report that decision's number, title, and current status

### Requirement: Reconstructed decisions record their provenance

A decision file produced by recovering a superseded or lost decision SHALL state which
primary sources its content was recovered from, so a reader can judge the reconstruction's
reliability and locate the underlying reasoning.

#### Scenario: A reconstructed decision is distinguishable from a contemporaneous one

- **WHEN** a reader opens a decision file whose content was recovered rather than authored at
  decision time
- **THEN** the file SHALL name the sources it was recovered from

### Requirement: The design reference carries only non-derivable rationale

`docs/spec.md` SHALL document only the rationale a requirement cannot state — platform and
FFI constraints, crate and module architecture, and rejected alternatives. It SHALL NOT restate
externally observable behavior, and SHALL direct readers to `openspec/specs/` for that.

#### Scenario: Behavior is looked up in one place

- **WHEN** a reader needs to know what the system does
- **THEN** `docs/spec.md` SHALL point to `openspec/specs/` rather than state the behavior
  itself

#### Scenario: A rationale has no requirement to live in

- **WHEN** a constraint explains why a design is shaped a certain way but is not observable
  behavior — for example a language or platform restriction on the FFI layer
- **THEN** that constraint SHALL be documented in `docs/spec.md` or an ADR rather than
  recorded as a requirement

### Requirement: A document citing a decision cites a standing one

A project document that cites a decision number as current authority SHALL cite a decision
whose status is `current`. A document asserting policy that a superseded decision supports
SHALL be corrected.

#### Scenario: A superseded decision is not cited as current policy

- **WHEN** a document cites a decision whose status is `superseded` as the governing policy
  for current behavior
- **THEN** the citation SHALL be corrected to reflect the decision's actual standing

#### Scenario: A replacement is cited instead

- **WHEN** a document's policy claim is supported by a newer decision
- **THEN** the document SHALL cite that newer decision
