## Context

`src/stream.rs` implements the streaming XLSX reader used by `Workbook.stream.xlsx.read(buffer)`. A qa-expert audit of the PR #24 fixes (issue #26) confirmed two residual risks suitable for a pre-merge hardening pass:

- **A4** — `parse_sheet_rows` toggles an `in_f: bool` formula-capture flag on `<f>`/`</f>`, but does not reset it at a cell boundary. Malformed/truncated XML missing `</f>` leaves the flag set, so the next cell's `<v>`/`<t>` text leaks into the prior cell's formula buffer.
- No automated test exercises **multi-sheet name↔file pairing / document order** — the path `sheet_number_from_target(target)` → `format!("xl/worksheets/sheet{}.xml", sheet_num)` in `stream_read`, which is exactly what the deferred A1 digit-greedy defect would corrupt.

This change addresses both with no public API change (`index.d.ts`, `package.json`, `src/stream_handle.rs` untouched).

## Goals / Non-Goals

**Goals:**

- Close the A4 malformed-XML flag-leak with a one-line guard.
- Add a regression net proving multi-sheet read-back preserves count, document-order names, and per-sheet cell values.

**Non-Goals:**

- Not fixing A1 (digit-greedy filename parse → wrong file pair). That is a *behavioral* change to file pairing; the cleaner fix is to pair by the real rels target path (`open xl/{target}`) rather than re-deriving from filename digits. Deferred to #26 for its own review, not slipped into a release candidate.
- Not fixing A3 (zip-bomb via declared `entry.size()` cap). That is a library-wide input-trust gap — the existing `Workbook.xlsx.read` (`src/reader/xlsx.rs`) has *no* size cap at all. Proper fix is a shared bounded-read helper across both readers; out of scope for this pre-merge streaming bundle. Tracked in #26.

## Decisions

- **Reset `in_f` at `</c>` (cell end).** Setting `in_f = false` when a cell closes bounds the flag's lifetime to a single cell, so a missing `</f>` cannot carry into the next cell. On well-formed XML the flag is already `false` at cell end, so output is identical — pure defensive guard, zero behavior change.
  - *Alternative considered:* reset at `<c>` Start. Equivalent safety; `</c>` End is chosen to keep the flag valid through the whole cell body (a formula may span the cell's children) and is the minimal, localized change.
- **Build the multi-sheet test input via `stream_write`, then read via `stream_read`.** Reusing the existing writer produces a guaranteed-valid zip and directly exercises the reader's pairing path on real round-tripped data, avoiding fragile manual XML construction.
  - *Alternative considered:* hand-build a zip with `zip::ZipWriter`. More control but duplicates writer logic and risks mismatching the exact element/attribute names the reader expects; rejected as more code for no extra coverage.

## Risks / Trade-offs

- [Risk] The `stream_write`→`stream_read` test uses standard `sheet1.xml`/`sheet2.xml` filenames, so it guards ordering/name/value pairing but does **not** by itself reproduce A1's digit-greedy bug (which needs a non-standard filename). → Mitigation: A1's specific fix and a non-standard-filename test are tracked in #26 (post-merge). The test still fails if the pairing/order logic regresses on standard names.
- [Risk] `parse_styles_and_sheet_maps` / `parse_shared_strings` are called by `stream_read` and may expect parts absent from a minimal written workbook. → Mitigation: `stream_write`'s output already contains exactly what `stream_read` consumes (the existing `stream_write_then_read_roundtrip` test proves this path works), so the new test reuses that validated contract.
