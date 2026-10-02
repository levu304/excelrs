# Spec Delta

## MODIFIED Requirements

### Requirement: A document citing a decision cites a standing one

A project document that cites a decision number as current authority SHALL cite a decision
whose status is `current`. A document asserting policy that a superseded decision supports
SHALL be corrected. A citation SHALL name the decision that actually records the cited
decision: where a decision has been renumbered, the citation SHALL name the current number,
and a number that resolves to an existing decision standing for unrelated subject matter is a
wrong citation even though that decision's status satisfies the first condition.

#### Scenario: A superseded decision is not cited as current policy

- **WHEN** a document cites a decision whose status is `superseded` as the governing policy
  for current behavior
- **THEN** the citation SHALL be corrected to reflect the decision's actual standing

#### Scenario: A replacement is cited instead

- **WHEN** a document's policy claim is supported by a newer decision
- **THEN** the document SHALL cite that newer decision

#### Scenario: A renumbered decision is cited by its current number

- **WHEN** a decision was renumbered to resolve a collision with an unrelated decision, and a
  document cites it by the number it held before the renumbering
- **THEN** the citation SHALL be corrected to the current number, and SHALL NOT be left pointing
  at the decision that now occupies the earlier number