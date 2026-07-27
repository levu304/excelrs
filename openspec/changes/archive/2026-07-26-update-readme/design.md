## Context

excelrs's `Worksheet.addImage(opts)` bundles registration and positioning into one call. ExcelJS separates them: `Workbook.addImage(buffer) → imageId` then `Worksheet.addImage(imageId, range)`. The divergence is deliberate (simpler, no global media store needed), but undocumented in README. Users migrating from ExcelJS hit a TypeScript error and must discover the correct API by trial.

## Goals / Non-Goals

**Goals:**

- Document the `addImage` API difference in README so migrating users see it immediately
- Include a working code example

**Non-Goals:**

- No Rust code changes — API stays as-is
- No TypeScript type changes — `index.d.ts` already accurately reflects the API
- No spec changes — the images spec already documents the excelrs API correctly

## Decisions

- **Single README paragraph** in the "API Surface" section (after Quick Start, before Style System). It's the first place users look after hitting a type error.
- **Not a migration guide**: A short inline note suffices. No separate MIGRATION.md needed for one API difference.
- **Example included**: Show both the ExcelJS pattern (for comparison) and the excelrs pattern.

## Risks / Trade-offs

- **Paragraph goes stale** if API changes later. Mitigation: minor risk — `addImage` is stable and shipped since v1.0.0.
- **User might still miss it**: A README note helps but won't prevent every confused search. Mitigation: the TypeScript error itself is the curriculum — user searches, finds the note.
