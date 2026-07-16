# comments Specification

## Purpose
TBD - created by archiving change v1-0-0. Update Purpose after archive.
## Requirements
### Requirement: Cell exposes a comment/note

A cell SHALL expose a comment accessor (ExcelJS `cell.note`) returning the
cell's comment text and author, and an equivalent setter. Setting a comment
SHALL attach it to the owning worksheet's comment part.

#### Scenario: Set a cell comment

- **WHEN** `ws.getCell("A1").note = "Review needed"`
- **THEN** `ws.getCell("A1").note` (or `cell.comment.text`) equals `"Review needed"`, and the comment is anchored to cell `A1`

### Requirement: Writer emits comments part and relationship

When a worksheet has comments, the writer SHALL emit `xl/commentsN.xml`
containing `<commentList><comment ref="<cellRef>" authorId="<n>"><text>...`
plus an author table, and register a `comments` relationship from the sheet's
`.rels` to the part. A worksheet without comments SHALL NOT emit a comments
part or relationship.

#### Scenario: Emit comments part

- **WHEN** cells A1 and B2 carry comments
- **THEN** `xl/commentsN.xml` exists with two `<comment>` entries (refs A1, B2), and `xl/worksheets/_rels/sheetN.xml.rels` contains a `comments` relationship

#### Scenario: No comments omits part

- **WHEN** a worksheet has no comments
- **THEN** no `xl/commentsN.xml` is emitted for that sheet

### Requirement: Reader parses comments part

The reader SHALL parse each sheet's `xl/commentsN.xml` (resolved via the sheet
`.rels`) and populate `cell.note` (and author) for the referenced cells. A
sheet without a comments relationship SHALL leave its cells comment-less.

#### Scenario: Read comments from Excel/ExcelJS

- **WHEN** `xl/commentsN.xml` contains `<comment ref="A1" authorId="0"><text><t>Done</t></text></comment>`
- **THEN** `ws.getCell("A1").note === "Done"` (or `cell.comment.text === "Done"`)

#### Scenario: Round-trip preserves text and author

- **WHEN** a comment with text and author is written, then the workbook is read back
- **THEN** the read-back comment text and author equal the originally written values

