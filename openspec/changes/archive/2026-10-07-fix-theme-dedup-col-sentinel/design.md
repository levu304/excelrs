# Design

## Context

See proposal.md (Why) for motivation. Current state:

- `src/writer/styles.rs:72-76` — `canonical_key` serializes `Font`/`Fill`/`Border` via `serde_json::to_string`; the theme-link fields (`Font.color_theme`/`color_tint` at `src/model/style.rs:235-240`, `Fill.foreground/background_theme/tint` at `:296-308`, `BorderStyle.color_theme/tint` at `:372-377`) are all `#[serde(skip)]`, so they are invisible to the key. Dedup maps (`font_map`/`fill_map`/`border_map`, `styles.rs:126-153`) use first-wins `or_insert_with`, so the twin arriving second inherits the first twin's slot and emission form. The `#[serde(skip)]` is load-bearing for the napi/JS surface and must stay.
- `src/reader/xlsx.rs` — `parse_col_dims_from_xml` (`:1376`) and `parse_col_outline_levels_from_xml` (`:1294`) both loop `for c in lo..=hi.min(16384)`: upper-clamped against the range-bomb guard, lower-unclamped, so `min="0"` yields `col_num = 0`, a value the rest of the model reserves as the auto-assign sentinel (`column.rs`, `worksheet.rs:645-653`; `get_column_by_num` rejects `0` at `:679-681`). The writer formats `min` verbatim (`writer/xlsx.rs:1900`), so the sentinel round-trips as schema-invalid `min="0"`.

## Goals / Non-Goals

**Goals:**
- Themed/plain twins occupy separate sub-table slots with zero behavior change for twin-free workbooks.
- `col_num = 0` becomes unreachable from file input in both `<col>` parsers.

**Non-Goals:**
- No change to the `#[serde(skip)]` model fields, the napi surface, or `emit_color_attrs` (theme-wins stands).
- No repair/normalization of other out-of-range values (e.g. `max > 16384` stays upper-clamped as today); no new validation errors surfacing to JS.

## Decisions

**D1 — Theme-aware dedup key via a key wrapper, not by un-skipping serde fields.**
Append the linkage to the key at the `canonical_key` call sites (e.g. key on `(canonical_key(value), theme_link(value))` or a small `theme_key` helper per sub-table type), leaving `#[serde(skip)]` and the serialized shape untouched. Rationale: un-skipping would leak `color_theme`/`color_tint` into the napi/JS object and the `RenderedRunKey`/shared-string dedup in `writer/xlsx.rs`, widening the blast radius to the FFI contract. Alternative considered (un-skip + `#[napi(skip)]`-style hiding): rejected — serde is the FFI shape here, so there is no separate hiding mechanism.
Applied uniformly to font, fill (foreground and background links), and all border-side links, since the repro confirmed the same hole in all three sub-tables.

**D2 — Clamp the lower bound (`lo.max(1)`) in both `<col>` parsers, not just the dims parser.**
`for c in lo.max(1)..=hi.min(16384)` at both `xlsx.rs:1294` and `:1376`. Rationale: the outline parser shares the identical pattern and the bug-hunter repro confirmed it creates the same sentinel; fixing one while leaving the other preserves a crafted-input path to the same invalid output. An inverted range after clamping (e.g. `min=0 max=0`) is safely empty, matching existing behavior for inverted ranges. Alternative considered (explicit error on `min < 1`): rejected — the reader's posture toward malformed descriptors is skip-and-continue (cf. `Err(_) => break`, outline-missing handling), and erroring would turn a benign crafted file into a read failure.

**D3 — No migration; sentinel columns vanish on re-read.**
Any `col_num = 0` definition persisted only in memory or in files written by the buggy writer. After D2, re-reading such a file drops the descriptor; no data migration or version gate is needed. Style-index shifts in mixed twin workbooks require no migration either (indices are recomputed per write).

## Risks / Trade-offs

- [Risk] Style-table slot count grows for mixed twin workbooks (one slot becomes two) → Mitigation: twin-free workbooks are byte-identical by construction (keys unchanged when no theme link is present on either side); add a byte-stability test for plain-only and themed-only outputs.
- [Risk] Key-wrapper must stay in sync if new skipped link fields are added later → Mitigation: the new unit test pins a themed/plain twin pair per sub-table type, so a future skipped field that affects emission fails loudly instead of silently merging.
- [Risk] `lo.max(1)` silently drops crafted descriptors rather than reporting them → Mitigation: accepted; consistent with the reader's existing skip-and-continue posture for malformed `<col>` input.

**Status: implemented.** Both fixes landed with unit tests (twin pairs per sub-table type, twin-free byte-stability, `min="0"` ignored in both parsers, valid `min="1"` still parsed) and a writer round-trip guard; `cargo test --lib` (454), clippy, `openspec validate` (44/44), and vitest (222/222) all pass.

## Open Questions

None — both defects are reproduced with root cause and fix direction confirmed; no spec-, approach-, or task-changing unknowns remain.
