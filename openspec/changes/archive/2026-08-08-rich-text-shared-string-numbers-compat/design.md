## Context

Rich-text cells are currently written as **inline strings**: `write_cell_xml`'s
`CellType::RichText` arm (src/writer/xlsx.rs:1878) emits `<c … t="inlineStr"><is><r><rPr>…</rPr><t>…</t></r></is></c>`. The
output is schema-valid and Excel/LibreOffice render it correctly, but **Apple
Numbers ignores inline-string rich-text run fonts** and falls back to the cell's
default (Calibri). Investigation confirmed the writer is correct — the produced
`output.xlsx` contains `<rFont val="Times New Roman"/>` on every run — so this
is a compatibility gap, not a defect.

The shared-string table already exists and is the natural home for rich text:

- `build_shared_strings(worksheets)` (xlsx.rs:374) returns
  `(Vec<String>, HashMap<String, u32>)` — keyed by **plain `String`** today.
- `write_shared_strings(w, &string_table)` (xlsx.rs:1071) emits
  `<sst><si><t>…</t></si>…</sst>`.
- The read side (v0.12.0) already parses shared-string `<si><r>` runs, so the
  round-trip back is supported.

This change reroutes rich-text emission through that table.

## Goals / Non-Goals

**Goals:**

- Write rich text as shared strings (`t="s"` + `<v>idx</v>`, runs in
  `xl/sharedStrings.xml`) so Numbers renders per-run fonts.
- Preserve round-trip fidelity (read-back matches written runs).
- Preserve whitespace/newlines in run text via `xml:space="preserve"`.

**Non-Goals:**

- No change to the JavaScript API (`cell.value = { richText }` unchanged).
- No change to the reader (already supports shared-string rich text).
- No rich-text support in the streaming writer (it does not emit rich text
  today; that is a separate gap).
- No new formula functions or other capabilities.

## Decisions

1. **Shared strings over inline — and why not "fix inline".**
   The root cause is that Numbers does not honor inline-string rich-text run
   fonts; there is no reliable inline-only workaround. Shared strings are what
   Excel itself emits and what Numbers imports reliably. Side benefit: shared
   strings dedupe, so repeated rich-text content costs less than repeated
   inline blobs.

2. **Extend the shared-string key/table to carry rich text.**
   Today `string_indices: HashMap<String, u32>` and `string_table: Vec<String>`
   are plain-text only. Introduce a key/entry that can be either plain or rich:
   - A `SharedString` enum (or equivalent) with `Plain(String)` and
     `Rich(Vec<RichTextRun>)` variants.
   - `build_shared_strings` inserts rich-text cells under their run content;
     identical runs hit the same `or_insert_with` slot (dedupe preserved).
   - `write_shared_strings` emits `<si><t>…</t></si>` for `Plain` and
     `<si><r><rPr>…</rPr><t>…</t></r></si>` for `Rich` (reusing the existing
     per-run `<rPr>` serialization already written inline).
   - `write_cell_xml`'s rich-text arm looks up the shared index and emits
     `t="s"` + `<v>idx</v>` instead of `<is>…</is>`.
   Alternative considered: serialize rich text to a canonical `String` key and
   keep `HashMap<String, u32>` — rejected because it conflates plain and rich
   keys and complicates `write_shared_strings` dispatch; an enum key is cleaner
   and keeps dedupe correct.

3. **`xml:space="preserve"` on run text with significant whitespace.**
   When a run's text has leading/trailing whitespace or a newline, emit
   `<t xml:space="preserve">…</t>`. Applied wherever run text is serialized
   (shared-string path), so the user's `"B: (11) = (7) + (10)\n"` keeps its
   trailing newline.

## Risks / Trade-offs

- [Numbers still shows Calibri after shared strings] → **Mitigation (validation
  gate, first task):** reproduce on a Mac with Numbers using the current inline
  output (expect Calibri) then with shared-string output (expect correct font).
  If shared strings do **not** resolve it, stop and pivot — the cause is
  elsewhere (e.g. stale native addon, or a deeper Numbers quirk) and this change
  is the wrong fix. Do not assume shared strings is the cure without the Numbers
  test.
- [Shared-string table growth / `count` attributes] → Mitigation: dedupe by
  content key (same as today); keep `sharedStrings.xml` `count`/`uniqueCount`
  accurate when rich entries are added. Growth bounded by distinct strings.
- [Breakage of existing inline-rich-text readers] → Mitigation: write path only;
  old inline files still read back fine (reader handles both). No migration.

## Migration Plan

Output-only change; no data migration and no rollback hazard. To revert, revert
the change — previously written inline files remain readable. No schema/version
bump required beyond the normal release.

## Open Questions

- Does shared-string rich text definitively fix the Numbers rendering? This is
  the go/no-go gate (first implementation task), not a design fork — if it
  fails, the change is suspended and re-scoped rather than re-architected.
